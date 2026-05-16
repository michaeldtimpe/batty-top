# batty

An interactive terminal battery viewer for macOS, tuned for Apple Silicon.

`batty` is a modernized fork of [svartalf/rust-battop](https://github.com/svartalf/rust-battop) (unmaintained since 2020). It pairs the maintained [starship-battery](https://github.com/starship/rust-battery) crate with a small macOS-specific shim that reads the extra fields IOKit exposes on M-series Macs (manufacture lot codes, firmware version, lifetime min/max telemetry, cycle service limit, etc.) and surfaces them in the TUI.

## Features

- Live state-of-charge gauge with colour thresholds (red ≤15%, yellow ≤30%, green above)
- Real-time charts for voltage, power draw / charge rate, and temperature
- Information panel with vendor, model, S/N, firmware, cell revision, charge state, cycle count vs service limit
- Energy panel with voltage, capacity, current/last-full/design energy (Wh or J)
- Time-to-full / time-to-empty estimates
- Environment + lifetime panel with current temperature, total operating time, first-use date, battery health metric, lifetime peak temperature/charge/voltage, and system disconnect count
- Multiple-battery support (separate tab per battery)
- Clean terminal restore on `q`, `Esc`, `Ctrl-C`, panic, or any unexpected exit

## Install (from source)

Requires Rust 1.85 or newer (edition 2024).

```sh
git clone https://github.com/michaeldtimpe/batty-top.git
cd batty-top
cargo install --path batty-top
```

The binary is named `batty`; run it from any terminal.

For a fast iteration loop:

```sh
cargo run --release -p batty
```

## Controls

| Key                | Action            |
|--------------------|-------------------|
| `←` / `→`          | Previous / next battery tab |
| `q` / `Esc` / `Ctrl-C` | Quit            |
| `-v`, `-vv`, … (CLI) | Increase log verbosity (logs go to stderr) |
| `-d <secs>` (CLI)  | Update interval (default 1s) |
| `-u si\|human` (CLI) | Use joules+kelvin vs Wh+°C |

Run `batty --help` for the full flag reference.

## Apple Silicon notes

A few fields aren't available on M-series IORegistry no matter how hard you look (see [LESSONS.md](LESSONS.md) for the long version):

- **Vendor name** — `Manufacturer` key isn't exposed on Apple Silicon. `batty` falls back to the pack-lot code from `ManufacturerData` (e.g. `lot F8`), and runs a defensive substring lookup for the well-attested supplier codes (Simplo/Sunwoda/Desay/Amperex/etc.) which almost never match on M-series in practice.
- **Technology / chemistry** — no `Type` key in IOKit either. We hardcode `Lithium Polymer` on macOS, which is correct for every Apple Silicon Mac and recent Intel Mac.
- **Cycle count `0`** — that's a real value, not a missing one. A fresh battery genuinely reports `0` until you accumulate roughly one full equivalent discharge.
- **Cycle service limit** — Apple does not expose this through IOKit. `batty` reads it by parsing `pmset -g rawbatt` output, which itself hardcodes 300 internally.

Everything else (state of charge, voltage, power, temperature, capacity, etc.) is read straight from the standard `starship-battery` interface.

## Workspace layout

This repository is a Cargo workspace with three crates:

- [`batty-top/`](batty-top/) — the TUI binary
- [`batty-mac-extras/`](batty-mac-extras/) — macOS-only shim that pulls supplemental fields from `AppleSmartBattery` IORegistry and `pmset`
- [`probe/`](probe/) — a tiny diagnostic binary that dumps every field for verification against `ioreg` / `system_profiler`

See [ARCHITECTURE.md](ARCHITECTURE.md) for how they fit together.

## Licence

Dual-licensed under Apache-2.0 OR MIT, matching upstream `rust-battop`.

## Acknowledgements

- [svartalf/rust-battop](https://github.com/svartalf/rust-battop) — the original TUI we forked
- [starship/rust-battery](https://github.com/starship/rust-battery) — the maintained battery-information crate
- [ratatui](https://github.com/ratatui-org/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm) — the modern TUI stack
