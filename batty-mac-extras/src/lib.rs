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

/// Community-attested supplier codes. Sources: MacRumors thread 2237683,
/// Apple Community thread 255404782, ELService Centre macbook-battery-manufacturer.
/// Typically does NOT match on Apple Silicon — supplier codes are not exposed in
/// any public IOKit field on M-series. Kept for defensive coverage (legacy Intel
/// paths, future Apple API changes). Non-exhaustive.
const SUPPLIER_CODES: &[(&str, &str)] = &[
    ("SMP", "Simplo Technology"),
    ("SWD", "Sunwoda Electronic"),
    ("DSY", "Desay Battery"),
    ("ATL", "Amperex Technology"),
    ("NVT", "Navitas/CosMX"),
    ("LGC", "LG Chem"),
    ("DP", "Dynapack International"), // 2-char — anchored to start (see decode_vendor)
];

fn decode_vendor(mfg_data_ascii: &str) -> Option<&'static str> {
    SUPPLIER_CODES.iter().find_map(|(code, name)| {
        let hit = if code.len() <= 2 {
            mfg_data_ascii.starts_with(code)
        } else {
            mfg_data_ascii.contains(code)
        };
        if hit { Some(*name) } else { None }
    })
}

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
    /// Best-effort supplier name decoded from `ManufacturerData`. Typically `None`
    /// on Apple Silicon — see `SUPPLIER_CODES` for the lookup table.
    pub vendor: Option<&'static str>,
    /// Battery health score from `BatteryData.BatteryHealthMetric`. Exposed raw —
    /// Apple does not document the scale or whether higher/lower is better.
    pub battery_health_metric: Option<i64>,
    /// Lifetime peak temperature ever recorded, in °C.
    /// From `BatteryData.LifetimeData.MaximumTemperature` (0.1 °C units on recent Macs).
    pub lifetime_max_temperature_c: Option<f64>,
    /// Lifetime peak charge current ever recorded, in mA.
    pub lifetime_max_charge_current_ma: Option<i64>,
    /// Lifetime peak pack voltage ever recorded, in mV.
    pub lifetime_max_pack_voltage_mv: Option<i64>,
    /// Number of times the system has been disconnected from the battery
    /// (e.g. battery removed/reinstalled). May reset across firmware updates.
    pub system_disconnect_count: Option<i64>,
    /// Cycle count "service limit" reported by `pmset -g rawbatt` (the `M` in
    /// `Cycles=N/M`). On Apple Silicon this is typically 300 — a pmset-internal
    /// constant, not an IOKit-readable design limit. Useful as display context
    /// next to the cycle count.
    pub cycle_count_design: Option<i64>,
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

    // Full ASCII view of MfgData (truncated at first null) for supplier-code matching.
    let mfg_ascii: Option<String> = mfg_bytes.map(|b| {
        b.iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as char)
            .collect()
    });
    let vendor = mfg_ascii.as_deref().and_then(decode_vendor);

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
        vendor,
        battery_health_metric: battery_data
            .and_then(|d| d.get("BatteryHealthMetric"))
            .and_then(Value::as_signed_integer),
        lifetime_max_temperature_c: lifetime_data
            .and_then(|d| d.get("MaximumTemperature"))
            .and_then(Value::as_signed_integer)
            .map(|raw| {
                let scaled = raw as f64 / 10.0;
                // Sanity clamp: if scaled would exceed 100 °C the source was probably
                // already in whole °C (older Macs reportedly used that convention).
                if scaled > 100.0 { raw as f64 } else { scaled }
            }),
        lifetime_max_charge_current_ma: lifetime_data
            .and_then(|d| d.get("MaximumChargeCurrent"))
            .and_then(Value::as_signed_integer),
        lifetime_max_pack_voltage_mv: lifetime_data
            .and_then(|d| d.get("MaximumPackVoltage"))
            .and_then(Value::as_signed_integer),
        system_disconnect_count: lifetime_data
            .and_then(|d| d.get("SystemDisconnectCount"))
            .and_then(Value::as_signed_integer),
        cycle_count_design: read_pmset_cycle_design(),
    })
}

/// Parse the `Cycles=N/M` field from `pmset -g rawbatt` output and return `M`.
/// Returns `None` if pmset fails, is missing, or doesn't include the field.
fn read_pmset_cycle_design() -> Option<i64> {
    let out = Command::new("/usr/bin/pmset").args(["-g", "rawbatt"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&out.stdout).ok()?;
    // Expected fragment: "Cycles=N/M;"
    let idx = text.find("Cycles=")?;
    let rest = &text[idx + "Cycles=".len()..];
    let slash = rest.find('/')?;
    let after = &rest[slash + 1..];
    let end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
    after[..end].parse().ok()
}
