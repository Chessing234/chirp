import { contextBridge, ipcRenderer } from 'electron';
import type { List, Task, AppSettings, ListReminder } from '../shared/types';

const api = {
  // List operations
  lists: {
    getAll: (): Promise<List[]> => ipcRenderer.invoke('db:lists:getAll'),
    create: (name: string, color?: string): Promise<List> =>
      ipcRenderer.invoke('db:lists:create', name, color),
    update: (id: string, updates: { name?: string; color?: string; reminder?: ListReminder | null }): Promise<List | undefined> =>
      ipcRenderer.invoke('db:lists:update', id, updates),
    delete: (id: string): Promise<boolean> => ipcRenderer.invoke('db:lists:delete', id),
  },

  // Task operations
  tasks: {
    getAll: (): Promise<Task[]> => ipcRenderer.invoke('db:tasks:getAll'),
    getByList: (listId: string): Promise<Task[]> =>
      ipcRenderer.invoke('db:tasks:getByList', listId),
    create: (task: {
      list_id: string;
      content: string;
      due_at?: number;
      ping_interval?: number;
      parent_id?: string;
    }): Promise<Task> => ipcRenderer.invoke('db:tasks:create', task),
    update: (
      id: string,
      updates: Partial<{
        content: string;
        completed: boolean;
        due_at: number | null;
        ping_interval: number | null;
      }>
    ): Promise<Task | undefined> => ipcRenderer.invoke('db:tasks:update', id, updates),
    delete: (id: string): Promise<boolean> => ipcRenderer.invoke('db:tasks:delete', id),
    toggleComplete: (id: string): Promise<Task | undefined> =>
      ipcRenderer.invoke('db:tasks:toggleComplete', id),
  },

  // App operations
  app: {
    hide: (): Promise<void> => ipcRenderer.invoke('app:hide'),
    show: (): Promise<void> => ipcRenderer.invoke('app:show'),
    minimize: (): Promise<void> => ipcRenderer.invoke('app:minimize'),
    close: (): Promise<void> => ipcRenderer.invoke('app:close'),
    quit: (): Promise<void> => ipcRenderer.invoke('app:quit'),
    getSettings: (): Promise<AppSettings> => ipcRenderer.invoke('app:getSettings'),
    setSettings: (key: string, value: unknown): Promise<AppSettings> =>
      ipcRenderer.invoke('app:setSettings', key, value),
    getMode: (): Promise<'overlay' | 'full'> => ipcRenderer.invoke('app:getMode'),
    setMode: (mode: 'overlay' | 'full'): Promise<'overlay' | 'full'> =>
      ipcRenderer.invoke('app:setMode', mode),
    getCurrentShortcut: (): Promise<string | null> => ipcRenderer.invoke('app:getCurrentShortcut'),
  },

  // Notifications
  notification: {
    show: (title: string, body: string, taskId?: string): Promise<void> =>
      ipcRenderer.invoke('notification:show', title, body, taskId),
  },

  // Reminders
  reminder: {
    snooze: (taskId: string, minutes: number): Promise<void> =>
      ipcRenderer.invoke('reminder:snooze', taskId, minutes),
    markDone: (taskId: string): Promise<void> =>
      ipcRenderer.invoke('reminder:markDone', taskId),
  },

  // Event listeners
  on: (channel: string, callback: (...args: unknown[]) => void) => {
    const validChannels = ['navigate', 'navigate:list', 'task:updated', 'list:updated', 'mode:changed'];
    if (validChannels.includes(channel)) {
      const subscription = (_event: Electron.IpcRendererEvent, ...args: unknown[]) =>
        callback(...args);
      ipcRenderer.on(channel, subscription);
      return () => ipcRenderer.removeListener(channel, subscription);
    }
    return () => {};
  },
};

contextBridge.exposeInMainWorld('pingpal', api);

export type PingPalAPI = typeof api;
