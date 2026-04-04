# Chirp

A keyboard-first task manager and ping reminder for the terminal. Vim-style navigation, natural language task entry, recurring reminders with native desktop notifications.

## Install

```bash
# from source
cargo install --path .

# then run
chirp
```

Requires Rust 1.70+. On macOS, notifications work out of the box. On Linux, ensure `libdbus` is installed (`apt install libdbus-1-dev` or equivalent).

## Features

- Vim-style navigation (`j`/`k`, `h`/`l`)
- Multiple task lists with quick switching
- Natural language dates and times
- Recurring ping reminders with desktop notifications
- Fuzzy search across tasks
- Persistent SQLite storage
- Context-sensitive keybind bar (no memorization needed)

## Keybindings

### Normal mode

| Key | Action |
|-----|--------|
| `i` / `a` | Add new task |
| `j` / `k` / arrows | Navigate tasks |
| `h` / `l` / tab | Switch lists |
| `g` / `G` | Jump to top / bottom |
| `space` / `enter` / `x` | Toggle complete |
| `d` | Delete task |
| `/` | Fuzzy search |
| `c` | Show/hide completed |
| `n` | New list |
| `r` | Rename list |
| `D` | Delete list |
| `?` | Help overlay |
| `q` / `esc` | Quit |

### Insert mode (adding a task)

| Key | Action |
|-----|--------|
| `enter` | Add the task |
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

Type tasks with embedded dates, times, and ping intervals:

```
buy groceries tomorrow 5pm
review proposal in 2h
call mom monday 9am ping 30m
exercise next week ping 1h
standup today 10am ping 15m
deploy friday 3pm
```

### Supported date/time formats

- `tomorrow`, `today`, `next week`
- Day names: `monday`, `tuesday`, etc.
- Times: `5pm`, `3:30pm`, `at 9am`
- Relative: `in 30m`, `in 2h`

### Ping reminders

Add `ping <interval>` to any task to get recurring desktop notifications:

```
ping 30m     -- every 30 minutes
ping 1h      -- every hour
every 15m    -- alternative syntax
```

Pings start firing once the task's due time passes (or immediately if no due date is set). Notifications repeat at the specified interval until the task is marked complete.

## Data

SQLite database stored at:
- **macOS**: `~/Library/Application Support/Chirp/chirp.db`
- **Linux**: `~/.local/share/Chirp/chirp.db`

## License

MIT
