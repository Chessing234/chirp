# Chirp

Minimalist task manager for the terminal. Vim keys, natural language input, persistent ping reminders that won't let you forget.

Built with Rust, [ratatui](https://github.com/ratatui/ratatui), and SQLite.

## Preview

```
 chirp   today  Inbox  Work                              2 due  ⚡  NORMAL

 ▸ p1 ○ deploy to prod                     Today 3:00 PM  ~30m 12m~
   p2 ○ buy groceries                      Tomorrow 5:00 PM
      ○ review PR                           Friday 9:00 AM
   p3 ○ exercise                            daily  ~1h 45m~
      ○ call dentist                        next week

   -- completed (2) --
      ✓ send weekly report
      ✓ fix login bug

 ─────────────────────────────────────────────────────────────────────────
 ▸ 5 pending
 i add  e edit  ␣ done  d del  u undo  s snooze  / search  t today  ? help  q quit
```

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

- Vim navigation (`j`/`k`, `h`/`l`), mouse support
- Natural language: `buy milk tomorrow 5pm p1 ping 30m daily`
- Priority levels (`p1`/`p2`/`p3`) -- auto-sorted, color-coded
- Recurring tasks (`daily`/`weekly`/`monthly`)
- Ping reminders with persistent desktop notifications
- Background daemon -- runs 24/7, auto-starts on login
- Undo support (`u`) for deletes and toggles
- Fuzzy search, multiple lists, task notes
- CLI subcommands for scripting (`add`, `list`, `done`, `export`)
- Cross-platform: macOS, Linux, Windows

## CLI

```bash
# Launch the TUI
chirp
chirp --list Work          # open directly into a list

# Manage tasks without the TUI
chirp add "buy milk tomorrow 5pm p2"
chirp add "standup daily" --list Work
chirp list                 # show all lists with task counts
chirp list --json          # JSON output for scripting
chirp done                 # today's completed tasks
chirp done --json

# Backup and migration
chirp export               # dump all tasks as JSON
chirp --import tasks.json  # import from JSON or CSV

# Daemon management
chirp daemon status
chirp daemon stop
chirp daemon restart
chirp daemon install       # manual install (auto-starts on login)
chirp daemon uninstall
```

## Keybindings

| Key | Action |
|-----|--------|
| `i` | Add task |
| `e` | Edit task |
| `space` | Toggle done |
| `d` | Delete |
| `u` | Undo last action |
| `s` | Snooze ping |
| `j`/`k` | Navigate |
| `J`/`K` | Reorder |
| `h`/`l` | Switch lists |
| `t` | Today view |
| `/` | Search |
| `c` | Show/hide completed |
| `enter` | Detail pane |
| `?` | Help |
| `q` | Quit |

Mouse: click to select, click checkbox to toggle, scroll wheel to navigate.

## Input syntax

```
buy groceries tomorrow 5pm p2
review PR in 2h p1 ping 30m
exercise daily ping 1h p3
standup monday 10am weekly
```

Modifiers: `p1`/`p2`/`p3`, `daily`/`weekly`/`monthly`, `ping 30m`, `tomorrow`/`monday`/`in 2h`/`5pm`.

Attach notes: type `note <text>` with a task selected.

## Data

- macOS: `~/Library/Application Support/Chirp/chirp.db`
- Linux: `~/.local/share/Chirp/chirp.db`
- Windows: `%APPDATA%\Chirp\chirp.db`

## License

MIT
