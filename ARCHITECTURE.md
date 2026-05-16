# Architecture

## Workspace shape

```
batty/                              ← workspace root (this repo)
├── Cargo.toml                      virtual workspace, members = [...]
├── probe/                          diagnostic binary
│   └── src/main.rs                 prints every field once and exits
├── batty-mac-extras/               macOS-only shim crate
│   └── src/lib.rs                  shells to ioreg + pmset, parses plist
└── batty-top/                      the TUI binary "batty"
    └── src/
        ├── main.rs                 CLI entry, logger init, error plumbing
        ├── errors.rs               crate-local Error/Result enum
        └── app/
            ├── application.rs      battery::Manager + the run() loop
            ├── config.rs           clap 4 derive: -v/-d/-u
            ├── events.rs           crossterm event::poll loop
            ├── mod.rs              re-exports + macOS Extras alias
            └── ui/
                ├── chart.rs        rolling-history data for the 3 charts
                ├── interface.rs    Terminal setup + Drop guard + panic hook
                ├── mod.rs          re-exports
                ├── painter.rs      ratatui rendering — the big file
                ├── tabs.rs         TabBar state (current index, titles)
                ├── units.rs        Human / SI enum
                └── view.rs         per-battery state owned by Interface
```

Why three crates and not one:
- `batty-mac-extras` is independently testable from the TUI and may be useful to other Rust tools.
- `probe` exists so we can A/B every field against `ioreg` without launching the TUI.
- `batty-top` is the only thing the end user runs.

## Data flow

```
        ┌────────────────────────────────────────────────┐
        │  starship-battery 0.11   (from crates.io)      │
        │  Manager → Battery       (cross-platform)      │
        │  state_of_charge, voltage, energy_rate, …      │
        └─────────────┬──────────────────────────────────┘
                      │
                      │ &Battery
                      ▼
        ┌────────────────────────────────────────────────┐
        │  batty-top::ui::View                           │
        │  - holds one Battery                           │
        │  - tracks rolling history (ChartData × 3)      │
        └─────────────┬──────────────────────────────────┘
                      │
                      │ &View
                      ▼
        ┌────────────────────────────────────────────────┐  ┌──────────────────────────────────────┐
        │  batty-top::ui::Painter                        │◄─┤  batty-mac-extras::BatteryExtras     │
        │  - reads &View + Option<&Extras> via Context   │  │  read() shells to:                   │
        │  - renders gauge, tables, charts via ratatui   │  │   • /usr/sbin/ioreg -a -r -c …       │
        └─────────────┬──────────────────────────────────┘  │   • /usr/bin/pmset -g rawbatt        │
                      │                                     │  Parsed once at startup, immutable.  │
                      │ frame                               └──────────────────────────────────────┘
                      ▼
        ┌────────────────────────────────────────────────┐
        │  ratatui::Terminal<CrosstermBackend<Stdout>>   │
        │  alt screen, raw mode, cursor hidden           │
        └────────────────────────────────────────────────┘
```

## The event loop (single-threaded)

`batty-top/src/app/events.rs` is a `loop { … }` that calls `crossterm::event::poll(remaining_tick)`:

- If a key event arrives within the remaining-tick budget, return `Event::Key…`.
- If the budget expires, return `Event::Tick`.
- A `ctrlc` handler flips an `AtomicBool` that the loop checks every iteration to deliver `Event::Exit` cleanly on SIGINT.

This is much simpler than upstream battop's two-thread `mpsc` model. We dropped that because:
- `crossterm::event::poll` already handles "wait until a key arrives or timeout", which is the whole reason termion needed a separate thread.
- No more thread join handles, no channel send/recv error propagation, half the LOC.

## Terminal restore — three guards

If any single guard is missing, the user's terminal ends up in raw mode or stuck in the alternate screen. We install all three:

| Guard                 | Where                      | Fires on                |
|-----------------------|----------------------------|-------------------------|
| `impl Drop for Interface` | `ui/interface.rs`      | normal exit, error propagation, scope unwind |
| `panic::set_hook(...)`    | `ui/interface.rs`      | panic anywhere in render path |
| `ctrlc::set_handler(...)` | `app/events.rs`        | SIGINT (Ctrl-C)         |

The Ctrl-C handler doesn't call `disable_raw_mode` directly — it sets a flag, returns to the main loop, and lets the normal exit path run the Drop guard. That keeps cleanup ordering deterministic.

## How `batty-mac-extras` actually works

We considered linking directly against IOKit via `objc2-io-kit` (the same crate `starship-battery` uses internally) but settled on shelling out to `/usr/sbin/ioreg -a -r -c AppleSmartBattery`, which emits a plist, plus `/usr/bin/pmset -g rawbatt` for the cycle service limit. Reasons:

- 30 lines of `plist::Value::as_dictionary()` traversal is much easier to audit than the `IOServiceMatching` / `IORegistryEntryCreateCFProperties` / `CFRelease` dance.
- `ioreg` and `pmset` are stable across macOS versions; their command-line flags haven't changed in over a decade.
- Subprocess overhead is ~10ms each on a current Mac. We invoke them once at startup, never again — `BatteryExtras` is immutable after `read()`.

The decode of `ManufacturerData` byte pairs (pack lot, PCB lot, firmware, hardware revision, cell revision) mirrors what `system_profiler` does internally. The `DateOfFirstUse` field is CFAbsoluteTime — seconds since 2001-01-01 UTC — which we convert to a `SystemTime` and ultimately a `YYYY-MM-DD` string in the painter (Howard Hinnant's civil-date algorithm, inline).

The vendor lookup (`SUPPLIER_CODES` table in `batty-mac-extras/src/lib.rs`) is defensive only. On Apple Silicon the supplier code is not present in any IOKit field, so the lookup virtually never matches and the painter falls through to `lot <pack_lot>`.

## Layout (TUI)

```
+--------+------+-----+
| BAT0   | BAT1 | BAT2|              tabs (height 3)
+--------+------+-----+
|  gauge   |          |              left col width 40, right col flexes
|  [info]  |  voltage |
|  [info]  |  chart   |
|  [energy]|          |
|  [time]  |  power   |
|  [env+   |  chart   |
|   life]  |          |
|          |  temp    |
|          |  chart   |
+----------+----------+
```

Left-column constraints (in `painter.rs::draw`):

| Block                       | Constraint        |
|-----------------------------|-------------------|
| State-of-charge gauge       | `Length(3)`       |
| Information                 | `Length(12)`      |
| Energy                      | `Length(9)`       |
| Time                        | `Length(5)`       |
| Environment + lifetime      | `Min(10)`         |

Info tables use `[Constraint::Min(14), Constraint::Min(18)]` so long values like the 18-char S/N render in full without forcing the label column to be variable-width.
