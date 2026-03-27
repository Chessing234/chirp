import {
  app,
  BrowserWindow,
  globalShortcut,
  ipcMain,
  Tray,
  Menu,
  nativeImage,
  Notification,
  screen,
  systemPreferences,
  dialog,
  shell,
} from 'electron';
import path from 'path';
import { Database } from './database';
import { ReminderEngine } from './reminder-engine';
import Store from 'electron-store';
import type { ListReminder } from '../shared/types';

// Prevent EPIPE crashes when stdout/stderr pipe is broken (e.g., parent terminal closed)
process.stdout?.on?.('error', (err: NodeJS.ErrnoException) => {
  if (err.code === 'EPIPE') return;
  throw err;
});
process.stderr?.on?.('error', (err: NodeJS.ErrnoException) => {
  if (err.code === 'EPIPE') return;
  throw err;
});

// Handle uncaught EPIPE errors (console.log throws synchronously when pipe is broken)
process.on('uncaughtException', (err: NodeJS.ErrnoException) => {
  if (err.code === 'EPIPE') return; // Ignore broken pipe errors
  console.error('Uncaught exception:', err);
  process.exit(1);
});

// Safe console wrapper that silently ignores EPIPE errors
const safeConsole = {
  log: (...args: unknown[]) => {
    try { console.log(...args); } catch { /* ignore */ }
  },
  warn: (...args: unknown[]) => {
    try { console.warn(...args); } catch { /* ignore */ }
  },
  error: (...args: unknown[]) => {
    try { console.error(...args); } catch { /* ignore */ }
  },
};

safeConsole.log('PingPal starting...');

// Set app name for macOS menu bar
app.setName('PingPal');

const isDev = !app.isPackaged;
safeConsole.log('isDev:', isDev);

let mainWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let db: Database;
let reminderEngine: ReminderEngine;

// Track window mode: 'overlay' (via shortcut, auto-hides) or 'full' (via dock click, stays open)
let windowMode: 'overlay' | 'full' = 'full';

// Flag to track if app is actually quitting (vs just closing window)
let isQuitting = false;

// Helper to safely send IPC messages to the window
function safeSend(channel: string, ...args: unknown[]): void {
  try {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send(channel, ...args);
    }
  } catch {
    // Window may have been destroyed, ignore
  }
}

// Helper to safely show the window
function safeShowWindow(): void {
  try {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.show();
      mainWindow.focus();
    }
  } catch {
    // Window may have been destroyed, ignore
  }
}

const store = new Store({
  defaults: {
    launchOnStartup: true,  // Auto-start so global shortcut always works
    globalShortcut: 'CommandOrControl+Shift+P',
    soundEnabled: true,
    theme: 'dark',
  },
});

function createWindow(): void {
  const { width: screenWidth, height: screenHeight } = screen.getPrimaryDisplay().workAreaSize;

  const windowWidth = 680;
  const windowHeight = 600;

  // Get the appropriate icon for the platform
  let appIcon: Electron.NativeImage | undefined;
  try {
    if (process.platform === 'win32') {
      // Windows uses .ico
      const icoPath = path.join(__dirname, '../../resources/icon.ico');
      appIcon = nativeImage.createFromPath(icoPath);
    } else if (process.platform === 'linux') {
      // Linux uses .png
      const pngPath = path.join(__dirname, '../../resources/icon.png');
      appIcon = nativeImage.createFromPath(pngPath);
    } else {
      // macOS uses .icns but also accepts .png for window icon
      const pngPath = path.join(__dirname, '../../resources/icon.png');
      appIcon = nativeImage.createFromPath(pngPath);
    }
  } catch {
    // Icon loading failed, continue without
  }

  const windowOptions: Electron.BrowserWindowConstructorOptions = {
    width: windowWidth,
    height: windowHeight,
    minWidth: 500,
    minHeight: 400,
    x: Math.round((screenWidth - windowWidth) / 2),
    y: Math.round((screenHeight - windowHeight) / 3),
    frame: false,
    resizable: true,
    movable: true,
    alwaysOnTop: false,
    skipTaskbar: false,
    show: false,
    hasShadow: true,
    transparent: false,
    backgroundColor: '#111111',
    icon: appIcon && !appIcon.isEmpty() ? appIcon : undefined,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js'),
    },
  };

  // Windows: rounded corners
  if (process.platform === 'win32') {
    windowOptions.roundedCorners = true;
  }

  mainWindow = new BrowserWindow(windowOptions);

  if (isDev) {
    mainWindow.loadURL('http://localhost:5173');
    // Enable DevTools for debugging
    mainWindow.webContents.openDevTools({ mode: 'detach' });
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
  }

  mainWindow.on('blur', () => {
    // Only auto-hide in overlay mode (when opened via shortcut)
    if (mainWindow && !isDev && windowMode === 'overlay') {
      mainWindow.hide();
    }
  });

  mainWindow.once('ready-to-show', () => {
    // First launch is in full mode
    windowMode = 'full';
    mainWindow?.show();
  });

  // Prevent window from being destroyed on close - just hide it
  // This keeps the app running in the background so the shortcut works
  mainWindow.on('close', (event) => {
    if (!isQuitting) {
      event.preventDefault();
      mainWindow?.hide();
    }
  });
}

function createTray(): void {
  const iconPath = path.join(__dirname, '../../resources/tray-icon.png');

  // Create a simple tray icon
  let icon: Electron.NativeImage;
  try {
    icon = nativeImage.createFromPath(iconPath);
    if (icon.isEmpty()) {
      // Create a simple 16x16 icon if file doesn't exist
      icon = nativeImage.createEmpty();
    }
  } catch {
    icon = nativeImage.createEmpty();
  }

  tray = new Tray(icon.isEmpty() ? createDefaultTrayIcon() : icon);

  const contextMenu = Menu.buildFromTemplate([
    {
      label: 'Open PingPal',
      click: () => toggleWindow(),
    },
    {
      label: 'Settings',
      click: () => {
        if (mainWindow) {
          mainWindow.show();
          mainWindow.webContents.send('navigate', 'settings');
        }
      },
    },
    { type: 'separator' },
    {
      label: 'Launch on Startup',
      type: 'checkbox',
      checked: store.get('launchOnStartup') as boolean,
      click: (menuItem) => {
        store.set('launchOnStartup', menuItem.checked);
        try {
          app.setLoginItemSettings({ openAtLogin: menuItem.checked });
        } catch {
          // May fail due to permissions
        }
      },
    },
    { type: 'separator' },
    {
      label: 'Quit (Stay in Background)',
      click: () => {
        // Just hide - app stays running so shortcut works
        mainWindow?.hide();
        try {
          new Notification({
            title: 'PingPal Running',
            body: `Still running in background. Press ${currentShortcut?.replace('Command', '⌘').replace('Option', '⌥').replace('Control', '⌃').replace('Shift', '⇧').replace(/\+/g, '') || '⌘⌥P'} to open.`,
            silent: true,
          }).show();
        } catch {
          // Notification may fail
        }
      },
    },
    {
      label: 'Force Quit',
      click: () => {
        isQuitting = true;
        app.quit();
      },
    },
  ]);

  tray.setToolTip('PingPal');
  tray.setContextMenu(contextMenu);
  tray.on('click', () => toggleWindow());
}

function createDefaultTrayIcon(): Electron.NativeImage {
  // Create a simple 16x16 white circle icon
  const size = 16;
  const canvas = Buffer.alloc(size * size * 4);

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const idx = (y * size + x) * 4;
      const cx = size / 2;
      const cy = size / 2;
      const r = size / 2 - 2;
      const dist = Math.sqrt((x - cx) ** 2 + (y - cy) ** 2);

      if (dist <= r) {
        canvas[idx] = 234;     // R
        canvas[idx + 1] = 234; // G
        canvas[idx + 2] = 234; // B
        canvas[idx + 3] = 255; // A
      } else {
        canvas[idx] = 0;
        canvas[idx + 1] = 0;
        canvas[idx + 2] = 0;
        canvas[idx + 3] = 0;
      }
    }
  }

  return nativeImage.createFromBuffer(canvas, { width: size, height: size });
}

// Toggle window in overlay mode (via global shortcut)
function toggleWindow(): void {
  if (!mainWindow || mainWindow.isDestroyed()) {
    safeConsole.log('toggleWindow: mainWindow is null or destroyed, recreating...');
    createWindow();
    return;
  }

  const isVisible = mainWindow.isVisible();
  const isFocused = mainWindow.isFocused();
  safeConsole.log('toggleWindow: isVisible=', isVisible, 'isFocused=', isFocused);

  if (isVisible && isFocused) {
    safeConsole.log('Hiding window');
    mainWindow.hide();
  } else {
    windowMode = 'overlay';
    safeSend('mode:changed', 'overlay');

    // Re-center window on current display
    const cursor = screen.getCursorScreenPoint();
    const currentDisplay = screen.getDisplayNearestPoint(cursor);
    const { width: screenWidth, height: screenHeight } = currentDisplay.workArea;
    const { x: displayX, y: displayY } = currentDisplay.workArea;

    const windowBounds = mainWindow.getBounds();
    const x = displayX + Math.round((screenWidth - windowBounds.width) / 2);
    const y = displayY + Math.round((screenHeight - windowBounds.height) / 3);

    mainWindow.setPosition(x, y);
    mainWindow.show();
    mainWindow.focus();
  }
}

// Show window in full mode (via dock click or app launch)
function showFullWindow(): void {
  if (!mainWindow) return;

  windowMode = 'full';
  safeSend('mode:changed', 'full');

  // Re-center window on current display
  const cursor = screen.getCursorScreenPoint();
  const currentDisplay = screen.getDisplayNearestPoint(cursor);
  const { width: screenWidth, height: screenHeight } = currentDisplay.workArea;
  const { x: displayX, y: displayY } = currentDisplay.workArea;

  const windowBounds = mainWindow.getBounds();
  const x = displayX + Math.round((screenWidth - windowBounds.width) / 2);
  const y = displayY + Math.round((screenHeight - windowBounds.height) / 3);

  mainWindow.setPosition(x, y);
  mainWindow.show();
  mainWindow.focus();
}

// Track the currently registered shortcut
let currentShortcut: string | null = null;

// Check and request accessibility permissions on macOS
async function checkAccessibilityPermissions(): Promise<boolean> {
  if (process.platform !== 'darwin') {
    return true; // Not needed on Windows/Linux
  }

  const isTrusted = systemPreferences.isTrustedAccessibilityClient(false);
  safeConsole.log('Accessibility permission status:', isTrusted);

  if (!isTrusted) {
    // Show dialog to user
    const result = await dialog.showMessageBox({
      type: 'warning',
      title: 'Accessibility Permission Required',
      message: 'PingPal needs Accessibility permission to use global keyboard shortcuts.',
      detail: 'Please grant PingPal access in System Preferences > Privacy & Security > Accessibility, then restart the app.',
      buttons: ['Open System Preferences', 'Later'],
      defaultId: 0,
    });

    if (result.response === 0) {
      // Open System Preferences to Accessibility
      shell.openExternal('x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility');
    }
    return false;
  }

  return true;
}

async function registerGlobalShortcut(): Promise<void> {
  // First, unregister any existing shortcuts
  globalShortcut.unregisterAll();
  currentShortcut = null;

  // Check accessibility permissions on macOS
  const hasPermission = await checkAccessibilityPermissions();
  if (!hasPermission) {
    safeConsole.warn('No accessibility permissions - shortcuts will not work');
    // Still try to register as user might grant permission later
  }

  // Shortcuts to try - using less common combinations to avoid conflicts
  // Cmd+Shift+P conflicts with many apps, so we try alternatives first
  const shortcutsToTry = [
    'Command+Option+P',           // macOS specific - less likely to conflict
    'CommandOrControl+Shift+P',   // Cross-platform
    'Command+Control+P',          // macOS specific alternative
    'CommandOrControl+Option+P',  // Cross-platform with Option
    'CommandOrControl+Shift+.',   // Alternative punctuation
  ];

  // If user has a custom shortcut, try it first
  const userShortcut = store.get('globalShortcut') as string;
  if (userShortcut && !shortcutsToTry.includes(userShortcut)) {
    shortcutsToTry.unshift(userShortcut);
  }

  for (const shortcut of shortcutsToTry) {
    if (!shortcut) continue;

    try {
      safeConsole.log('Attempting to register shortcut:', shortcut);

      const registered = globalShortcut.register(shortcut, () => {
        safeConsole.log('=== SHORTCUT TRIGGERED ===');
        safeConsole.log('Shortcut:', shortcut);
        safeConsole.log('Window exists:', !!mainWindow);
        safeConsole.log('Window destroyed:', mainWindow?.isDestroyed());
        toggleWindow();
      });

      if (registered) {
        // Double-check that it's actually registered
        const isActuallyRegistered = globalShortcut.isRegistered(shortcut);
        safeConsole.log('Registration check:', isActuallyRegistered);

        if (isActuallyRegistered) {
          currentShortcut = shortcut;
          safeConsole.log('Successfully registered global shortcut:', shortcut);

          // Update store with working shortcut
          if (shortcut !== store.get('globalShortcut')) {
            store.set('globalShortcut', shortcut);
          }

          // Show success notification
          try {
            new Notification({
              title: 'PingPal Ready',
              body: `Press ${shortcut.replace('Command', '⌘').replace('Option', '⌥').replace('Control', '⌃').replace('Shift', '⇧').replace(/\+/g, '')} to toggle`,
              silent: true,
            }).show();
          } catch {
            // Notification may fail
          }

          break;
        }
      }

      safeConsole.log('Failed to register shortcut:', shortcut);
    } catch (error) {
      safeConsole.error('Error registering shortcut:', shortcut, error);
    }
  }

  if (!currentShortcut) {
    safeConsole.error('=== FAILED TO REGISTER ANY SHORTCUT ===');
    safeConsole.error('Tried shortcuts:', shortcutsToTry);
    safeConsole.error('This usually means:');
    safeConsole.error('1. Accessibility permissions not granted');
    safeConsole.error('2. Another app is using all these shortcuts');
    safeConsole.error('3. System security settings blocking shortcuts');

    // Show error notification
    try {
      new Notification({
        title: 'PingPal Shortcut Failed',
        body: 'Could not register keyboard shortcut. Check Accessibility permissions.',
        silent: false,
      }).show();
    } catch {
      // Notification may fail
    }
  }
}

// Getter for current shortcut (for UI display)
function getCurrentShortcut(): string | null {
  return currentShortcut;
}

function setupIpcHandlers(): void {
  // List handlers
  ipcMain.handle('db:lists:getAll', () => db.getAllLists());
  ipcMain.handle('db:lists:create', (_, name: string, color?: string) => db.createList(name, color));
  ipcMain.handle('db:lists:update', (_, id: string, updates: { name?: string; color?: string; reminder?: ListReminder | null }) =>
    db.updateList(id, updates));
  ipcMain.handle('db:lists:delete', (_, id: string) => db.deleteList(id));

  // Task handlers
  ipcMain.handle('db:tasks:getAll', () => db.getAllTasks());
  ipcMain.handle('db:tasks:getByList', (_, listId: string) => db.getTasksByList(listId));
  ipcMain.handle('db:tasks:create', (_, task: {
    list_id: string;
    content: string;
    due_at?: number;
    ping_interval?: number;
    parent_id?: string;
  }) => db.createTask(task));
  ipcMain.handle('db:tasks:update', (_, id: string, updates: Partial<{
    content: string;
    completed: boolean;
    due_at: number | null;
    ping_interval: number | null;
    last_ping_at: number | null;
  }>) => db.updateTask(id, updates));
  ipcMain.handle('db:tasks:delete', (_, id: string) => db.deleteTask(id));
  ipcMain.handle('db:tasks:toggleComplete', (_, id: string) => db.toggleTaskComplete(id));

  // App handlers
  ipcMain.handle('app:hide', () => mainWindow?.hide());
  ipcMain.handle('app:show', () => {
    mainWindow?.show();
    mainWindow?.focus();
  });
  ipcMain.handle('app:minimize', () => mainWindow?.minimize());
  ipcMain.handle('app:close', () => mainWindow?.hide());
  ipcMain.handle('app:quit', () => {
    // Just hide - app stays running so shortcut works
    mainWindow?.hide();
  });
  ipcMain.handle('app:forceQuit', () => {
    isQuitting = true;
    app.quit();
  });
  ipcMain.handle('app:getMode', () => windowMode);
  ipcMain.handle('app:setMode', (_, mode: 'overlay' | 'full') => {
    windowMode = mode;
    return windowMode;
  });
  ipcMain.handle('app:getSettings', () => store.store);
  ipcMain.handle('app:setSettings', (_, key: string, value: unknown) => {
    store.set(key, value);
    if (key === 'globalShortcut') {
      registerGlobalShortcut();
    }
    if (key === 'launchOnStartup') {
      try {
        app.setLoginItemSettings({ openAtLogin: value as boolean });
      } catch {
        // May fail due to permissions on some platforms
      }
    }
    return store.store;
  });

  // Get the currently registered shortcut
  ipcMain.handle('app:getCurrentShortcut', () => getCurrentShortcut());

  // Notification handlers
  ipcMain.handle('notification:show', (_, title: string, body: string, taskId?: string) => {
    try {
      const notificationOptions: Electron.NotificationConstructorOptions = {
        title,
        body,
        silent: !store.get('soundEnabled'),
      };

      // Actions are only supported on macOS
      if (process.platform === 'darwin' && taskId) {
        notificationOptions.actions = [
          { type: 'button', text: 'Mark Done' },
          { type: 'button', text: 'Snooze 10m' },
        ];
      }

      const notification = new Notification(notificationOptions);

      notification.on('click', () => {
        safeShowWindow();
      });

      if (process.platform === 'darwin') {
        notification.on('action', (_event, index) => {
          if (taskId) {
            if (index === 0) {
              db.toggleTaskComplete(taskId);
              safeSend('task:updated', taskId);
            } else if (index === 1) {
              reminderEngine.snoozeTask(taskId, 10);
            }
          }
        });
      }

      notification.show();
    } catch {
      // Notification may fail, ignore
    }
  });

  ipcMain.handle('reminder:snooze', (_, taskId: string, minutes: number) => {
    reminderEngine.snoozeTask(taskId, minutes);
  });

  ipcMain.handle('reminder:markDone', (_, taskId: string) => {
    db.toggleTaskComplete(taskId);
    safeSend('task:updated', taskId);
  });
}

// Single instance lock
safeConsole.log('Requesting single instance lock...');
const gotTheLock = app.requestSingleInstanceLock();
safeConsole.log('Got lock:', gotTheLock);

if (!gotTheLock) {
  safeConsole.log('Another instance is running, quitting...');
  app.quit();
} else {
  safeConsole.log('Lock acquired, proceeding with app initialization...');
  app.on('second-instance', () => {
    if (mainWindow) {
      mainWindow.show();
      mainWindow.focus();
    }
  });

  app.whenReady().then(() => {
    // Set dock icon on macOS
    if (process.platform === 'darwin' && app.dock) {
      const dockIconPath = path.join(__dirname, '../../resources/icon.png');
      try {
        const dockIcon = nativeImage.createFromPath(dockIconPath);
        if (!dockIcon.isEmpty()) {
          app.dock.setIcon(dockIcon);
        }
      } catch {
        // Icon may not exist in dev mode
      }
    }

    // Initialize database
    db = new Database();

    // Create default list if none exists
    const lists = db.getAllLists();
    if (lists.length === 0) {
      db.createList('Inbox', '#4a9f6e');
    }

    createWindow();
    createTray();
    registerGlobalShortcut();
    setupIpcHandlers();

    // Initialize reminder engine
    reminderEngine = new ReminderEngine(db, {
      onTaskReminder: (task) => {
        // IMMEDIATELY show the app in the user's face
        windowMode = 'full';
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.show();
          mainWindow.focus();
          // Bring to front even if other apps are focused
          mainWindow.setAlwaysOnTop(true);
          setTimeout(() => {
            mainWindow?.setAlwaysOnTop(false);
          }, 1000);
        }

        // Navigate to the task's list
        safeSend('navigate:list', task.list_id);

        // Also show notification
        try {
          const notification = new Notification({
            title: '🔔 PingPal Reminder',
            body: task.content,
            silent: !store.get('soundEnabled'),
          });
          notification.show();
        } catch {
          // Notification may fail, ignore
        }

        // Update last ping time
        db.updateTask(task.id, { last_ping_at: Date.now() });
        safeSend('task:updated', task.id);
      },

      onListReminder: (list, pendingCount) => {
        // IMMEDIATELY show the app in the user's face
        windowMode = 'full';
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.show();
          mainWindow.focus();
          // Bring to front even if other apps are focused
          mainWindow.setAlwaysOnTop(true);
          setTimeout(() => {
            mainWindow?.setAlwaysOnTop(false);
          }, 1000);
        }

        // Navigate to the specific list
        safeSend('navigate:list', list.id);

        // Also show notification
        try {
          const reminderTypeText = list.reminder?.type === 'activity' ? 'Activity check' :
                                   list.reminder?.type === 'daily' ? 'Daily reminder' :
                                   list.reminder?.type === 'weekly' ? 'Weekly reminder' : 'Reminder';

          const notification = new Notification({
            title: `📋 ${list.name}`,
            body: `${reminderTypeText}: You have ${pendingCount} pending task${pendingCount > 1 ? 's' : ''}`,
            silent: !store.get('soundEnabled'),
          });
          notification.show();
        } catch {
          // Notification may fail, ignore
        }
      }
    });
    reminderEngine.start();

    // Set login item settings (may fail due to permissions on some platforms)
    try {
      app.setLoginItemSettings({
        openAtLogin: store.get('launchOnStartup') as boolean,
      });
    } catch {
      // Ignore - may not have permission to set login items
    }
  });
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  // When clicking dock icon, open in full mode
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  } else {
    showFullWindow();
  }
});

app.on('before-quit', (event) => {
  // Only actually quit if Force Quit was used
  if (!isQuitting) {
    event.preventDefault();
    mainWindow?.hide();

    // Show notification that app is still running
    try {
      new Notification({
        title: 'PingPal Running',
        body: `Still running in background. Press ${currentShortcut?.replace('Command', '⌘').replace('Option', '⌥').replace('Control', '⌃').replace('Shift', '⇧').replace(/\+/g, '') || '⌘⌥P'} to open. Use tray menu "Force Quit" to fully exit.`,
        silent: true,
      }).show();
    } catch {
      // Notification may fail
    }
  }
});

app.on('will-quit', () => {
  globalShortcut.unregisterAll();
  reminderEngine?.stop();
  db?.close();
});

// Note: We keep the dock icon visible so users can click on it
