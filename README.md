# Chirp v0.3.1

Minimalist task manager for the terminal. Vim keys, natural language input, persistent ping reminders that won't let you forget.

## Install

```bash
cargo install chirp-tui
```

Or build from source:

```bash
cargo install --path .
```

The daemon auto-installs on first run. It sends desktop notifications (with sound) even when the TUI is closed. For urgent overdue p1 tasks, it opens a terminal window in your face.

Requires Rust 1.70+. Works on macOS, Linux, and Windows.

## Features

- Vim navigation (`j`/`k`, `h`/`l`)
- Natural language: `buy milk tomorrow 5pm p1 ping 30m daily`
- Priority levels (`p1`/`p2`/`p3`) -- auto-sorted, color-coded
- Recurring tasks (`daily`/`weekly`/`monthly`)
- Ping reminders with persistent desktop notifications
- Background daemon -- runs 24/7, auto-starts on login
- Fuzzy search, multiple lists, task notes
- Cross-platform: macOS, Linux, Windows

## Keybindings

| Key | Action |
|-----|--------|
| `i` | Add task |
| `e` | Edit task |
| `space` | Toggle done |
| `d` | Delete |
| `s` | Snooze ping |
| `j`/`k` | Navigate |
| `J`/`K` | Reorder |
| `h`/`l` | Switch lists |
| `t` | Today view |
| `/` | Search |
| `enter` | Detail pane |
| `?` | Help |
| `q` | Quit |

## Input syntax

```
buy groceries tomorrow 5pm p2
review PR in 2h p1 ping 30m
exercise daily ping 1h p3
standup monday 10am weekly
```

Modifiers: `p1`/`p2`/`p3`, `daily`/`weekly`/`monthly`, `ping 30m`, `tomorrow`/`monday`/`in 2h`/`5pm`.

Attach notes: type `note <text>` with a task selected.

## Daemon

```bash
chirp daemon status      # check if running
chirp daemon stop        # stop the daemon
chirp daemon restart     # stop + start
chirp daemon install     # manual install (auto-starts on login)
chirp daemon uninstall   # remove auto-start service
```

The daemon auto-installs on first `chirp` launch. It uses launchd (macOS), systemd (Linux), or Task Scheduler (Windows).

## Import

```bash
chirp --import tasks.json
chirp --import tasks.csv
```

## Data

- macOS: `~/Library/Application Support/Chirp/chirp.db`
- Linux: `~/.local/share/Chirp/chirp.db`
- Windows: `%APPDATA%\Chirp\chirp.db`

## License

MIT
