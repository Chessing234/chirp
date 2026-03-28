# PingPal

**Keyboard-first, ultra-fast desktop task companion.**

A minimalist task manager combining the speed of a command palette with the power of recurring reminders.

<img src="resources/icon.svg" alt="PingPal" width="120">

## Features

- **⚡ Global Shortcut** - Summon anywhere with `Cmd/Ctrl + Shift + Space`
- **🎯 Quick Add** - Natural language task entry: "Call mom tomorrow 5pm ping 2h"
- **🔔 Smart Pings** - Recurring reminders that keep tasks top of mind
- **📋 Multiple Lists** - Organize tasks into focused lists
- **🔍 Fuzzy Search** - Find any task instantly
- **⌨️ Keyboard-First** - Navigate entirely with keyboard

## Natural Language Examples

```
Finish essay tomorrow 5pm ping every 2h
Call Rahul ping 30m
Review PR at 3pm
Submit report next monday
Buy groceries in 2h
```

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Open PingPal | `Cmd/Ctrl + Shift + Space` |
| Close | `Esc` |
| Navigate tasks | `↑` `↓` |
| Toggle task | `Cmd/Ctrl + Enter` |
| Add task | `Enter` |
| Switch list | `Tab` |

## Commands

Type in the input field:
- `/list` - Switch between lists
- `/new <name>` - Create new list
- `/settings` - Open settings
- `/clear` - Clear completed tasks
- `/quit` - Quit application

## Installation

### Prerequisites

- Node.js 18+ (20+ recommended)
- npm 9+

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/pingpal.git
cd pingpal

# Install dependencies (automatically rebuilds native modules for Electron)
npm install

# Build the application
npm run build

# Run the built application
npm run start
```

### Development Mode

```bash
# Terminal 1: Start Vite dev server for hot reload
npm run dev:renderer

# Terminal 2: Build and start Electron (watches for changes)
npm run dev:main

# Or run both concurrently
npm run dev
```

### Build for Production

```bash
# Build the application
npm run build

# Package for your platform
npm run package          # All platforms
npm run package:mac      # macOS only (.dmg, .zip)
npm run package:win      # Windows only (.exe)
npm run package:linux    # Linux only (.AppImage, .deb)
```

### Troubleshooting

If you encounter native module issues:
```bash
npm run rebuild
```

## Project Structure

```
/src
  /main                 # Electron main process
    index.ts           # App entry, window management, shortcuts
    database.ts        # SQLite database operations
    reminder-engine.ts # Background reminder scheduler
    preload.ts         # IPC bridge for renderer

  /renderer            # React application
    /components
      CommandPalette.tsx   # Main UI component
      TaskItem.tsx         # Individual task display
      ListSelector.tsx     # List switching dropdown
      SettingsPanel.tsx    # App settings
    /hooks
      useKeyboard.ts       # Keyboard navigation
      useFuzzySearch.ts    # Search functionality
    /lib
      store.ts             # Zustand state management
      parser.ts            # Natural language parsing
    /styles
      index.css            # Tailwind + custom styles

  /shared
    types.ts            # TypeScript interfaces

/resources              # App icons and assets
```

## Tech Stack

- **Framework**: Electron
- **UI**: React + TypeScript
- **Styling**: TailwindCSS
- **State**: Zustand
- **Database**: SQLite (better-sqlite3)
- **Build**: Vite + electron-builder

## Design Philosophy

- **Dark theme only** - Easy on the eyes
- **Monochrome + accent** - Minimal visual noise
- **Terminal aesthetic** - JetBrains Mono font
- **Smooth animations** - 150-250ms transitions
- **Keyboard-first** - Speed over clicks

## Data Storage

All data is stored locally in SQLite:
- **macOS**: `~/Library/Application Support/pingpal/pingpal.db`
- **Windows**: `%APPDATA%/pingpal/pingpal.db`
- **Linux**: `~/.config/pingpal/pingpal.db`

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing`)
5. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details.

---

Built with focus and intention.
