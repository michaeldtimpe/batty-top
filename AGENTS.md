# AGENTS.md

Notes for AI coding agents (Claude Code, Cursor, etc.) working in this repo. Humans are welcome to read too.

## What this codebase is

A small, three-crate Cargo workspace producing an interactive macOS terminal battery viewer. Single binary target (`batty`). Read [ARCHITECTURE.md](ARCHITECTURE.md) first; it has the full crate breakdown and data flow.

## Ground rules

- **Don't add new dependencies** without considering whether an existing one already covers the need. The current dep list is the minimum that does the job — `clap`, `ratatui`, `crossterm`, `ctrlc`, `log`, `env_logger`, `itertools`, `humantime`, `starship-battery`, `plist`.
- **Don't bump `starship-battery`** speculatively. We pin 0.11 because that's the version validated against M-series IOKit; newer releases may rearrange fields.
- **`batty-mac-extras` shells to `/usr/sbin/ioreg` and `/usr/bin/pmset`.** Don't switch it to direct IOKit FFI just because that "feels cleaner." The subprocess approach is deliberate (see ARCHITECTURE.md and LESSONS.md).
- **Don't reintroduce a multi-thread event loop.** The single-thread `crossterm::event::poll` pattern in `app/events.rs` is intentional. The old two-thread mpsc model has no advantage on current `crossterm`.
- **Don't touch the three terminal-restore guards** without understanding all three:
  - `impl Drop for Interface` in `ui/interface.rs`
  - `panic::set_hook(...)` installed in `ui/interface.rs::install_panic_hook`
  - `ctrlc::set_handler(...)` in `app/events.rs::EventHandler::from_config`
  Removing or merging any of them leaves the user's terminal in raw mode on at least one exit path.
- **Constraints in painter.rs**: prefer `Constraint::Min(_)` over `Constraint::Length(_)` for text columns. Long values (S/N, serial-like strings) will silently truncate with `Length`.
- **Lifetime telemetry fields stay `Option<T>`.** Don't normalize "missing" to `0` — they have diagnostic meaning and the painter renders `N/A` for `None`.

## Where to look first

| If you want to…                                              | Start in                                            |
|--------------------------------------------------------------|-----------------------------------------------------|
| Change which fields show in the Information panel            | `batty-top/src/app/ui/painter.rs::draw_common_info` |
| Change which fields show in Environment / lifetime           | `batty-top/src/app/ui/painter.rs::draw_environment_info` |
| Add a new key from IOKit (e.g. a lifetime stat we missed)    | `batty-mac-extras/src/lib.rs` — extend `BatteryExtras` + `read()`, then surface in painter |
| Change update interval / CLI flags                           | `batty-top/src/app/config.rs`                       |
| Change keybindings or add a hotkey                           | `batty-top/src/app/events.rs::EventHandler::next`   |
| Verify a new field against the OS                            | Run `cargo run -p probe`, cross-check with `ioreg -a -r -c AppleSmartBattery -d 0 | plutil -p -` |
| Change overall layout / panel sizes                          | `batty-top/src/app/ui/painter.rs::draw` (left-column constraints) |
| Modify what shows in tab titles                              | `batty-top/src/app/ui/view.rs::title`               |

## Validation before "done"

- `cargo build --release -p batty` — must succeed with no warnings.
- `cargo run -p probe` — must run cleanly and print all fields; eyeball anything that newly shows `None`.
- For UI changes: launch `cargo run --release -p batty` in a real terminal, tab through, exit with `q`. **You can't run the TUI from a CI-style headless tool — it needs a real TTY.** Hand-off to a human reviewer if you can't validate visually.

## Things that don't work and why

See [LESSONS.md](LESSONS.md). Especially relevant for agents:
- Trying to "fix" the `vendor: None` reading by tracking down some private IOKit class — Apple does not expose a clean vendor string on M-series, full stop. The defensive `SUPPLIER_CODES` table is the right level of effort; don't escalate to private APIs.
- Trying to decode `ManufactureDate` (the i64 packed timestamp from `BatteryData`) — format is undocumented and likely Mach-time-based; nobody has reverse-engineered it publicly. `DateOfFirstUse` (CFAbsoluteTime, working) is what we expose instead.
- Trying to "fix" `cycle_count: 0` on a fresh machine — that's a real reading.

## Style

- Comments only when the WHY isn't obvious from names. Don't narrate WHAT the code does.
- No emoji in source or commits.
- Commit messages: imperative present-tense subject ≤72 chars, then a paragraph that says WHY. The existing log is the style reference.
- No CI in this repo (deliberately). Don't add GitHub Actions or pre-commit hooks without asking.
