# CLAUDE.md

## Project overview

Chirp is a minimalist TUI task manager written in Rust. It uses ratatui/crossterm for the terminal UI, SQLite (bundled via rusqlite) for persistence, and a background daemon for ping reminders. Cross-platform: macOS, Linux, Windows.

## Architecture

```
src/
  main.rs     Entry point, CLI routing, TUI event loop, key handling
  app.rs      App state, task/list CRUD logic, search, agenda, undo stack
  ui.rs       Ratatui rendering (header, tasks, input, keybinds, dialogs)
  db.rs       SQLite schema, migrations, queries (WAL mode for concurrent access)
  parser.rs   Natural language parsing (dates, times, priorities, pings, recurrence)
  daemon.rs   Background reminder service, PID management, OS service install
  import.rs   JSON/CSV bulk import
```

## Build & test

```bash
cargo build          # debug build
cargo test           # 26+ tests across db, parser, daemon
cargo clippy -- -D warnings   # must pass with zero warnings
```

CI runs on macOS, Linux, and Windows. Release builds trigger on `v*` tags.

## CLI subcommands

```
chirp                    Launch TUI
chirp --list <name>      Launch TUI into a specific list
chirp add "text"         Add task from CLI (supports --list, natural language)
chirp list [--json]      Show all lists with pending/total counts
chirp done [--json]      Show today's completed tasks
chirp export             Dump all tasks as JSON to stdout
chirp --import <file>    Import from JSON/CSV
chirp daemon start|stop|restart|install|uninstall|status
```

The `--json` flag works with `list` and `done` for script-friendly output.

## Key conventions

- **Cross-platform**: all platform-specific code uses `#[cfg(target_os = "...")]`. Never use Unix-only APIs without a Windows alternative.
- **Data paths**: `db::data_dir()` returns the correct platform path. On Windows use `dirs::config_dir()` (%APPDATA%), on Unix use `dirs::data_dir()`.
- **Daemon**: auto-installs on first TUI launch via `daemon::auto_install()` which checks `is_installed() || is_running()` to avoid repeat work. Uses launchd (macOS), systemd (Linux), Task Scheduler (Windows).
- **Notifications**: use `notify-rust` with a fallback to `notify-send` CLI on Linux if libdbus is missing.
- **Terminal popup**: on macOS, check `$TERM_PROGRAM` to detect the user's terminal. Only Terminal.app and iTerm2 support AppleScript `do script`; others (Alacritty, Kitty, WezTerm) use `open -a`.
- **UI style**: minimalist dark theme. Unicode symbols (○/✓/▸), no box borders on main layout, single-line keybind bar at bottom. Colors defined as constants at top of ui.rs.
- **Undo**: app.rs maintains an undo stack (max 20 entries) for delete task, toggle done, and delete list. Press `u` to undo.
- **Mouse**: click to select, click checkbox to toggle, scroll wheel navigates. Enabled via crossterm EnableMouseCapture.
- **Tests**: all tests use `Database::in_memory()` to avoid touching real data. Parser tests are pure functions. Daemon tests use temp PID files and restore state.
- **Clippy**: must pass `cargo clippy -- -D warnings` before committing.
- **Commit style**: imperative mood, first line summarizes the change, body explains why.
- **Release**: `git tag v0.X.0 && git push --tags` triggers cross-platform builds via GitHub Actions.
- **Crate name**: `chirp-tui` on crates.io (chirp was taken). Binary installs as `chirp` via `[[bin]] name`.

## Dependencies (keep minimal)

ratatui, crossterm, rusqlite (bundled), chrono, uuid, dirs, regex, fuzzy-matcher, notify-rust, serde, serde_json, csv. Release profile uses LTO + strip + opt-level=z for small binaries.
