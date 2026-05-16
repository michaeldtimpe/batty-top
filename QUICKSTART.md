# Quickstart

For impatient readers. About 60 seconds to a running TUI on an Apple Silicon Mac.

## Prerequisites

- macOS (Apple Silicon recommended; should also build on Intel and Linux but the lifetime/extras panel is mac-only)
- Rust 1.85 or newer

Check your toolchain:

```sh
rustc --version
```

If you don't have Rust, the official installer is one line:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Homebrew's `rust` formula also works but lags rustup; if you're picking, use rustup.

## Build and run

```sh
git clone https://github.com/michaeldtimpe/batty-top.git
cd batty-top
cargo run --release -p batty
```

First build pulls roughly 100 crates and takes 30–60s on a current Mac. After that, incremental rebuilds are sub-second.

## Verify your data (optional)

The `probe` crate prints every value the TUI knows about, in one shot, without taking over your terminal. It's how we sanity-check against `ioreg` / `system_profiler`:

```sh
cargo run -p probe
```

Output looks like:

```
Found 1 battery/batteries

Battery #0
  vendor:           None
  model:            Some("SN7038")
  serial:           Some("F8YHS20011U0000XBL")
  state of charge:  97.0%
  ...

=== macOS extras (batty-mac-extras) ===
  pack lot code:    Some("F8")
  firmware version: Some("PR")
  technology:       Lithium Polymer
  health metric:    Some(25)
  cycle design:     Some(300)
  ...
```

Cross-check anything suspicious against:

```sh
ioreg -a -r -c AppleSmartBattery -d 0 | plutil -p -
system_profiler SPPowerDataType
pmset -g rawbatt
```

## Install permanently

When you like what you see:

```sh
cargo install --path batty-top
```

That puts `batty` on your `$PATH` (`~/.cargo/bin/batty`). Run from anywhere:

```sh
batty
```

## Common flags

```sh
batty -d 2          # update every 2 seconds instead of 1
batty -u si         # show energy in joules and temperature in kelvin
batty -vvv          # verbose logs to stderr (visible after you quit)
batty --help        # full flag reference
```

## Exit

`q`, `Esc`, or `Ctrl-C`. Your terminal will be restored cleanly — no leftover raw-mode garbage.

## What's next

- [README.md](README.md) — full feature list and Apple-Silicon caveats
- [ARCHITECTURE.md](ARCHITECTURE.md) — how the three workspace crates fit together
- [LESSONS.md](LESSONS.md) — what we learned the hard way; useful before making changes
- [AGENTS.md](AGENTS.md) — guidance for AI coding agents working in this repo
