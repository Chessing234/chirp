# PingPal

**Keyboard-first, ultra-fast desktop task companion.**

A minimalist task manager combining the speed of a command palette with the power of recurring reminders.

<img src="resources/icon.svg" alt="PingPal" width="120">

## Features

- **Global Shortcut** - Summon anywhere with `Cmd + E`
- **Quick Add** - Natural language task entry: "Call mom tomorrow 5pm ping 2h"
- **Smart Pings** - Recurring reminders that keep tasks top of mind
- **Multiple Lists** - Organize tasks into color-coded lists
- **Fuzzy Search** - Find any task instantly
- **Keyboard-First** - Navigate entirely with keyboard
- **System Tray** - Always accessible from your menu bar
- **Launch at Login** - Always ready when you need it

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Open PingPal | `Cmd + E` |
| Close | `Esc` |
| Navigate tasks | `↑` `↓` |
| Toggle task | `Cmd + Enter` |
| Add task | `Enter` |
| Switch list | `Tab` |

## Commands

Type in the input field:
- `/list` - Switch between lists
- `/new <name>` - Create new list
- `/settings` - Open settings
- `/clear` - Clear completed tasks

## Installation

### From Release

1. Download `PingPal-v1.0.0-macos-arm64.zip` from [Releases](https://github.com/Chessing234/PingPal/releases)
2. Extract the zip file
3. Move `PingPal.app` to `/Applications`
4. Run in Terminal to remove quarantine:
   ```bash
   xattr -cr /Applications/PingPal.app
   ```
5. Open PingPal and grant Accessibility permissions when prompted

### Build from Source

Prerequisites:
- Node.js 18+
- Rust (install via [rustup](https://rustup.rs))

```bash
# Clone the repository
git clone https://github.com/Chessing234/PingPal.git
cd PingPal

# Install dependencies
npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Tech Stack

- **Framework**: Tauri 2 (Rust)
- **UI**: React + TypeScript
- **Styling**: TailwindCSS
- **State**: Zustand
- **Database**: SQLite (rusqlite)
- **Build**: Vite + Tauri

## Project Structure

```
/src
  /renderer            # React frontend
    /components
      CommandPalette.tsx   # Main UI component
      TaskItem.tsx         # Individual task display
      ListSelector.tsx     # List switching dropdown
    /hooks
      useKeyboard.ts       # Keyboard navigation
    /lib
      store.ts             # Zustand state management
      parser.ts            # Natural language parsing
      tauri-api.ts         # Tauri IPC bindings

/src-tauri             # Rust backend
  /src
    lib.rs             # App logic, database, shortcuts
    main.rs            # Entry point
```

## Data Storage

All data is stored locally in SQLite:
- **macOS**: `~/Library/Application Support/com.pingpal.app/pingpal.db`

## Design Philosophy

- **Dark theme only** - Easy on the eyes
- **Monochrome + accent** - Minimal visual noise
- **Terminal aesthetic** - JetBrains Mono font
- **Smooth animations** - 150-250ms transitions
- **Keyboard-first** - Speed over clicks

## Requirements

- macOS 10.15 (Catalina) or later
- Apple Silicon (arm64)

## License

MIT License - see [LICENSE](LICENSE) for details.

---

Built with Rust and intention.
