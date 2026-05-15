//! Apple Silicon battery fields that `starship-battery` doesn't expose.
//!
//! Reads the `AppleSmartBattery` IORegistry entry by shelling out to `ioreg -a`
//! (which emits a plist) and decoding the `ManufacturerData` byte pairs the
//! same way `system_profiler SPPowerDataType` does internally.

#![cfg(target_os = "macos")]

use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use plist::Value;

/// Apple's CFAbsoluteTime epoch: 2001-01-01 00:00:00 UTC, expressed as Unix seconds.
const CF_EPOCH_UNIX_SECONDS: u64 = 978_307_200;

#[derive(Debug, Clone)]
pub struct BatteryExtras {
    /// Pack lot code, e.g. "F8". Decoded from bytes 0-1 of `ManufacturerData`.
    pub pack_lot_code: Option<String>,
    /// PCB lot code, e.g. "YH". Bytes 2-3.
    pub pcb_lot_code: Option<String>,
    /// Firmware version, e.g. "PR". Bytes 4-5.
    pub firmware_version: Option<String>,
    /// Hardware revision, e.g. "00". Bytes 6-7.
    pub hardware_revision: Option<String>,
    /// Cell revision, e.g. "EL". Bytes 8-9.
    pub cell_revision: Option<String>,
    /// Date the battery was first used. Decoded from `BatteryData.DateOfFirstUse`
    /// (CFAbsoluteTime — seconds since 2001-01-01 UTC).
    pub first_use: Option<SystemTime>,
    /// Battery chemistry. Hardcoded to "Lithium Polymer" on macOS — every
    /// Apple Silicon Mac and recent Intel Mac uses Li-poly cells, and IOKit
    /// exposes no `Type` key to read this from.
    pub technology: &'static str,
    /// Design capacity in mAh.
    pub design_capacity_mah: Option<i64>,
    /// Nominal full-charge capacity in mAh (current full capacity).
    pub nominal_charge_capacity_mah: Option<i64>,
    /// Lifetime operating time in hours. From `BatteryData.LifetimeData.TotalOperatingTime`.
    pub total_operating_time_hours: Option<i64>,
    /// Maximum capacity percentage (state of health, 1-100).
    pub max_capacity_percent: Option<i64>,
}

#[derive(Debug)]
pub enum Error {
    IoregFailed(std::io::Error),
    IoregNonZero { status: i32, stderr: String },
    PlistParse(plist::Error),
    NoBattery,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoregFailed(e) => write!(f, "failed to run ioreg: {}", e),
            Error::IoregNonZero { status, stderr } => {
                write!(f, "ioreg exited {}: {}", status, stderr)
            }
            Error::PlistParse(e) => write!(f, "failed to parse ioreg plist: {}", e),
            Error::NoBattery => write!(f, "no AppleSmartBattery found in IORegistry"),
        }
    }
}

impl std::error::Error for Error {}

pub fn read() -> Result<BatteryExtras, Error> {
    let output = Command::new("/usr/sbin/ioreg")
        .args(["-a", "-r", "-c", "AppleSmartBattery"])
        .output()
        .map_err(Error::IoregFailed)?;

    if !output.status.success() {
        return Err(Error::IoregNonZero {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let root: Value = plist::from_bytes(&output.stdout).map_err(Error::PlistParse)?;

    let entry = root
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_dictionary())
        .ok_or(Error::NoBattery)?;

    let battery_data = entry.get("BatteryData").and_then(Value::as_dictionary);
    let lifetime_data =
        battery_data.and_then(|d| d.get("LifetimeData")).and_then(Value::as_dictionary);

    let mfg_bytes: Option<&[u8]> = entry.get("ManufacturerData").and_then(Value::as_data);
    let decode_pair = |start: usize| {
        mfg_bytes.and_then(|b| b.get(start..start + 2)).and_then(|pair| {
            let s: String = pair.iter().map(|&c| c as char).collect();
            if s.chars().all(|c| c.is_ascii_graphic()) { Some(s) } else { None }
        })
    };

    let first_use = battery_data
        .and_then(|d| d.get("DateOfFirstUse"))
        .and_then(Value::as_signed_integer)
        .and_then(|cf_seconds| {
            if cf_seconds < 0 {
                None
            } else {
                Some(UNIX_EPOCH + Duration::from_secs(CF_EPOCH_UNIX_SECONDS + cf_seconds as u64))
            }
        });

    Ok(BatteryExtras {
        pack_lot_code: decode_pair(0),
        pcb_lot_code: decode_pair(2),
        firmware_version: decode_pair(4),
        hardware_revision: decode_pair(6),
        cell_revision: decode_pair(8),
        first_use,
        technology: "Lithium Polymer",
        design_capacity_mah: entry.get("DesignCapacity").and_then(Value::as_signed_integer),
        nominal_charge_capacity_mah: entry
            .get("NominalChargeCapacity")
            .and_then(Value::as_signed_integer),
        total_operating_time_hours: lifetime_data
            .and_then(|d| d.get("TotalOperatingTime"))
            .and_then(Value::as_signed_integer),
        max_capacity_percent: entry.get("MaxCapacity").and_then(Value::as_signed_integer),
    })
}
