# Lessons

Hard-won notes from building `batty`. Some of these are general-Rust lessons; others are macOS-IOKit specifics that cost real hours to figure out. Add to the bottom as new ones surface.

---

## 1. "The data is missing" usually means "the data isn't there"

We started this project planning to fork `starship-battery` and "fix" three obviously-buggy fields on Apple Silicon:

- `vendor: None`
- `technology: Unknown`
- `cycle_count: 0`

After a long IOKit dive, the situation turned out to be:

- **`vendor`** — There is **no** `Manufacturer` key in `AppleSmartBattery` on M-series. `system_profiler SPPowerDataType` doesn't show a vendor either, because Apple stopped surfacing it. coconutBattery and iStat Menus show one only because they maintain hardcoded lookup tables. There is nothing to "fix" upstream.
- **`technology`** — No `Type` key either. The chemistry isn't in IOKit. All Apple Silicon Macs use Li-poly, so we hardcode `"Lithium Polymer"` on macOS — but this is a heuristic, not a read.
- **`cycle_count: 0`** — Genuinely the correct value on a fresh battery. Apple counts a cycle as ~100% cumulative discharge. A laptop that's barely been off the charger really does report 0.

**The lesson:** before forking an upstream library to "fix" missing data, verify with the OS's own tools (`system_profiler`, `ioreg`, `pmset`) that the data actually exists somewhere in IOKit. We almost burned a week on a fork+PR that would have shipped no functional change. Always start from the data, not from the assumption.

---

## 2. Shell-out + plist beats raw IOKit FFI for this scope

We considered linking `batty-mac-extras` directly against IOKit via `objc2-io-kit` (which is already in the dep graph through `starship-battery`). Lifecycle of `CFDictionary` and `io_service_t` is fiddly: `IOServiceMatching` returns a dict that's consumed by `IOServiceGetMatchingService` on success but leaks on failure paths; `IORegistryEntryCreateCFProperties` allocates a new dict the caller must release; missing a single `CFRelease` is a leak.

Switching to `Command::new("/usr/sbin/ioreg").args(["-a", …])` + `plist::from_bytes` cut the code in half and made the data flow obvious. `ioreg` and `pmset` are stable across macOS versions and ship in `/usr/sbin` and `/usr/bin` respectively — they've been there since OS X 10.0.

**The lesson:** subprocess + structured-text-parsing isn't always a hack. For a one-shot read at startup of a tool that's already user-facing, "shell to the OS utility that already does the parsing right" can be the cleaner answer.

---

## 3. Don't fork upstream until you know what you'd PR

Original plan: fork `starship/rust-battery`, fix the three macOS fields, open a PR upstream once probe-validated. Final reality: we didn't fork at all. The "fixes" turned out to be either non-fixes (cycle_count was right) or heuristics (vendor/technology) that no upstream maintainer would accept.

Had we forked first, we would have:
- Set up `git subtree` plumbing for ~200 lines of pure overhead
- Added our changes to a long-lived branch that diverges from upstream
- Eventually given up on the PR and carried the fork forever

Instead we built `batty-mac-extras` as a separate workspace crate, kept `starship-battery 0.11` as a registry dep, and acquired exactly zero fork-maintenance burden.

**The lesson:** "Fork first, refine later" is backwards. Validate that the fix is something upstream actually wants *before* setting up the fork mechanics.

---

## 4. Constraint::Length is brittle for variable text

Our initial info table used `[Constraint::Length(17), Constraint::Length(17)]`. The S/N field on Apple Silicon is 18 chars (`F8YHS20011U0000XBL`), so it rendered as `F8YHS20011U0000XB` — silently truncated, no error, no warning.

Fix: `[Constraint::Min(14), Constraint::Min(18)]`. Both dimensions flex; values get as much space as the panel allows.

**The lesson:** in ratatui (and most text-layout systems), `Length` is a "hide overflow silently" trap for any column that holds variable-length text. Default to `Min` for text, reserve `Length` for things that genuinely have a fixed display width (gauges, fixed-glyph icons).

---

## 5. The `&format!(...)` temporary trap

The natural shape of our vendor-fallback chain was:

```rust
let vendor: &str = match (battery.vendor(), extras.pack_lot_code) {
    (Some(v), _) => v,
    (_, Some(lot)) => &format!("lot {}", lot),  // borrow-check error
    _ => "N/A",
};
```

The `format!` macro returns an owned `String` that gets dropped at the end of the match arm; `&format!(...)` returns a dangling reference. The compiler catches it, but only after a confusing first read.

Fixes are: return `String` from the match (and `.to_string()` the static branches), or hoist a `let lot_string;` binding outside the match and assign inside. We picked the first because the lifetime calculus is local and obvious.

**The lesson:** when a match returns `&str`, every arm that needs to format a new string must materialize the owned `String` in an outer-scope binding, or the whole match must change return type. The shape `&format!(...)` is always wrong.

---

## 6. Terminal restore needs three guards, not one

A TUI app that's manipulating raw mode and the alternate screen has at least three exit paths:

1. **Normal exit** — user presses `q`. Handled by `impl Drop for Interface`.
2. **Panic mid-render** — out-of-bounds array index, unwrap on a `None`, anything. Handled by `panic::set_hook(...)`.
3. **SIGINT** — user presses `Ctrl-C` or the process gets killed. Handled by `ctrlc::set_handler(...)`.

Any one of these missing leaves the terminal in raw mode with cursor hidden — usually requires the user to type `reset` blindly. We've installed all three; they all set the same global state (raw mode off, alt screen off, cursor visible), so duplication is fine.

The Ctrl-C handler in particular should **not** call `disable_raw_mode` directly — it should set an atomic flag and let the normal exit path run. Otherwise you race with the main thread's draw call.

**The lesson:** "graceful shutdown" for a TUI means *every* abnormal exit path also runs cleanup, not just the happy path.

---

## 7. `cycle_count: 0` is not a bug; lifetime telemetry has many "looks like a bug" moments

This one comes up repeatedly:

- `cycle_count: 0` — real on a new battery.
- `time_to_full: None` AND `time_to_empty: None` simultaneously — real when fully charged and plugged in.
- `vendor: None` — real on M-series.
- `cycle_count_design: 300` — comes from pmset hardcoding it, not from Apple-spec rated life (which is 1000 cycles on current MacBooks). Display what pmset says because that's what the user sees with their own tools.

**The lesson:** before "fixing" a battery reading, run `pmset -g rawbatt` and `system_profiler SPPowerDataType` and confirm the OS reports the same thing. If your tool disagrees with `system_profiler`, that's a bug. If they agree, the value is just unflattering reality.

---

## 8. `Option<T>` carries information that `T::default()` destroys

For diagnostic fields like `battery_health_metric`, `system_disconnect_count`, lifetime peak temperature, etc., it's tempting to default missing to `0`. Don't — `0` and "I don't have a reading" mean very different things. The painter renders `None` as `N/A`, which is the honest display.

**The lesson:** the moment you `unwrap_or(0)` a diagnostic field, you've lied to the user about whether the reading exists. Reserve `unwrap_or_default` for cases where the default genuinely is "this didn't happen" (e.g. counters that start at zero).

---

## 9. The original battop's two-thread event loop existed because of termion, not because it's the right design

`svartalf/rust-battop` had this shape:

```
thread A: stdin.keys() → mpsc::Sender<Event>
thread B: thread::sleep(tick) → mpsc::Sender<Event>
main:    mpsc::Receiver<Event>::recv()
```

That was necessary because `termion`'s key-reading is a blocking iterator with no timeout. On `crossterm`, `event::poll(remaining_tick)` already does "wait up to N ms for a key event, return whether one arrived" — which collapses the whole pattern to a single loop with one timer state variable.

We migrated and dropped the channels, the thread handles, and the `mpsc::RecvError` variant from the error enum. Half the LOC of `events.rs`.

**The lesson:** when migrating event loops, check whether the new library makes the old structural choices obsolete. Don't auto-mirror.

---

## 10. Don't commit `Cargo.lock` inside member crates of a workspace

When we `git subtree add`'d the original battop, it brought its own `Cargo.lock` along. Cargo silently ignores it (only the workspace root's lock matters), but committing it pollutes diffs and confuses contributors.

`rm batty-top/Cargo.lock` once, never again. Workspace root owns the lock.

---

## 11. The user's pack-lot code "F8" is not a vendor code

We spent an embarrassing amount of time trying to map `F8` (the first two bytes of `ManufacturerData` decoded as ASCII) to a supplier. Community-attested supplier codes are 3-char (`SMP`, `SWD`, `DSY`, `ATL`, `NVT`, `LGC`) or 2-char (`DP`); `F8` matches none of them. Apple's pack-lot prefix appears to be an internal manufacturing-location code, not a supplier identifier.

We ship the lookup table anyway as defensive code. It will not match on this user's machine, but it costs nothing to keep, and might match on someone else's Mac, or on a future macOS version that reorganizes the surface.

**The lesson:** be willing to ship code that you expect will not run successfully on your test environment, if it's defensive and zero-cost. But document loudly that it's defensive (see the comment block on `SUPPLIER_CODES`).
