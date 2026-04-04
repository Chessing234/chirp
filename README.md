# Chirp

A keyboard-first task manager and ping reminder for the terminal. Vim-style navigation, natural language task entry, recurring reminders with native desktop notifications.

## Install

```bash
# from source
cargo install --path .

# or via Homebrew (build from HEAD)
brew install --formula Formula/chirp.rb

# then run
chirp
```

Requires Rust 1.70+. On macOS, notifications and sound work out of the box. On Linux, ensure `libdbus` is installed (`apt install libdbus-1-dev`).

## Features

- Vim-style navigation (`j`/`k`, `h`/`l`)
- **Priority levels** (`p1`/`p2`/`p3`) — color-coded and auto-sorted
- **Recurring tasks** (`daily`/`weekly`/`monthly`) — auto-recreate on complete
- **Ping reminders** with desktop notifications and sound
- **Snooze** pings with a single keypress
- **Task editing** — press `e` to modify any task inline
- **Drag reorder** — `Shift+J`/`K` to move tasks up/down
- Fuzzy search across tasks
- Multiple lists with quick switching
- Natural language dates and times
- Import from JSON/CSV
- Persistent SQLite storage
- Context-sensitive keybind bar

## Keybindings

### Normal mode

| Key | Action |
|-----|--------|
| `i` / `a` | Add new task |
| `e` | Edit selected task |
| `j` / `k` / arrows | Navigate tasks |
| `J` / `K` (shift) | Move task down / up |
| `h` / `l` / tab | Switch lists |
| `g` / `G` | Jump to top / bottom |
| `space` / `enter` / `x` | Toggle complete |
| `s` | Snooze ping (one interval) |
| `d` | Delete task |
| `/` | Fuzzy search |
| `c` | Show/hide completed |
| `n` | New list |
| `r` | Rename list |
| `D` | Delete list |
| `?` | Help overlay |
| `q` / `esc` | Quit |

### Insert / Edit mode

| Key | Action |
|-----|--------|
| `enter` | Save task |
| `esc` | Cancel |
| `ctrl+a` / `ctrl+e` | Jump to start / end |
| `ctrl+w` | Delete word |
| `ctrl+u` | Clear line |

### Search mode

| Key | Action |
|-----|--------|
| Type | Fuzzy filter tasks |
| `enter` | Confirm selection |
| `ctrl+n` / `ctrl+p` | Navigate results |
| `esc` | Cancel search |

## Natural language input

Type tasks with embedded dates, times, priorities, pings, and recurrence:

```
buy groceries tomorrow 5pm p2
review proposal in 2h p1
call mom monday 9am ping 30m
exercise daily ping 1h p3
standup today 10am ping 15m
deploy friday 3pm p1
weekly review weekly p2
```

### Dates and times

- `tomorrow`, `today`, `next week`
- Day names: `monday`, `tuesday`, etc.
- Times: `5pm`, `3:30pm`, `at 9am`
- Relative: `in 30m`, `in 2h`

### Priority

- `p1` — high (red)
- `p2` — medium (yellow)
- `p3` — low (blue)

Tasks are automatically sorted by priority within each list.

### Ping reminders

```
ping 30m     — notify every 30 minutes
ping 1h      — notify every hour
every 15m    — alternative syntax
```

Pings fire as desktop notifications with sound once the due time passes. They repeat at the interval until marked complete. Press `s` to snooze for one interval.

The task list shows a live countdown to the next ping.

### Recurring tasks

```
exercise daily ping 1h
review weekly
report monthly p2
```

When a recurring task is completed, a new copy is automatically created with the next due date.

## Import

Import tasks from JSON or CSV files:

```bash
chirp --import tasks.json
chirp --import tasks.csv
```

### JSON format

```json
[
  {
    "content": "buy groceries",
    "list": "Inbox",
    "priority": 2,
    "due": "2025-06-15T17:00:00",
    "ping": "30m",
    "recurrence": "daily"
  }
]
```

### CSV format

```csv
content,list,priority,due,ping,recurrence
buy groceries,Inbox,2,2025-06-15T17:00:00,30m,daily
review PR,Work,1,,1h,
```

All fields except `content` are optional. Lists are created automatically if they don't exist.

## Data

SQLite database stored at:
- **macOS**: `~/Library/Application Support/Chirp/chirp.db`
- **Linux**: `~/.local/share/Chirp/chirp.db`

## CI / Releases

GitHub Actions builds cross-platform binaries on every tag:
- macOS universal binary (x86_64 + Apple Silicon)
- Linux x86_64

Create a release: `git tag v0.2.0 && git push --tags`

## License

MIT
