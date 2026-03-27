import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { List, Task, AppSettings, ListReminder } from '../../shared/types';

// Check if running in Tauri (v2 detection)
export const isTauri = () => {
  return typeof window !== 'undefined' &&
    ('__TAURI__' in window || '__TAURI_INTERNALS__' in window);
};

// Tauri API implementation with error handling
export const tauriApi = {
  lists: {
    getAll: async (): Promise<List[]> => {
      try {
        return await invoke('get_all_lists');
      } catch (e) {
        console.error('Failed to get lists:', e);
        return [];
      }
    },
    create: async (name: string, color?: string): Promise<List> => {
      try {
        return await invoke('create_list', { name, color });
      } catch (e) {
        console.error('Failed to create list:', e);
        throw e;
      }
    },
    update: async (id: string, updates: { name?: string; color?: string; reminder?: ListReminder | null }): Promise<List> => {
      try {
        return await invoke('update_list', {
          id,
          updates: {
            name: updates.name,
            color: updates.color,
            reminder: updates.reminder ? JSON.stringify(updates.reminder) : undefined,
          }
        });
      } catch (e) {
        console.error('Failed to update list:', e);
        throw e;
      }
    },
    delete: async (id: string): Promise<void> => {
      try {
        console.log('Deleting list:', id);
        await invoke('delete_list', { id });
        console.log('List deleted successfully');
      } catch (e) {
        console.error('Failed to delete list:', e);
        throw e;
      }
    },
  },

  tasks: {
    getAll: async (): Promise<Task[]> => {
      try {
        return await invoke('get_all_tasks');
      } catch (e) {
        console.error('Failed to get tasks:', e);
        return [];
      }
    },
    getByList: async (listId: string): Promise<Task[]> => {
      try {
        return await invoke('get_tasks_by_list', { listId });
      } catch (e) {
        console.error('Failed to get tasks by list:', e);
        return [];
      }
    },
    create: async (task: {
      list_id: string;
      content: string;
      due_at?: number;
      ping_interval?: number;
      parent_id?: string;
    }): Promise<Task> => {
      try {
        return await invoke('create_task', { task });
      } catch (e) {
        console.error('Failed to create task:', e);
        throw e;
      }
    },
    update: async (id: string, updates: Partial<{
      content: string;
      completed: boolean;
      due_at: number | null;
      ping_interval: number | null;
      last_ping_at: number | null;
    }>): Promise<Task> => {
      try {
        return await invoke('update_task', { id, updates });
      } catch (e) {
        console.error('Failed to update task:', e);
        throw e;
      }
    },
    delete: async (id: string): Promise<void> => {
      try {
        await invoke('delete_task', { id });
      } catch (e) {
        console.error('Failed to delete task:', e);
        throw e;
      }
    },
    toggleComplete: async (id: string): Promise<Task> => {
      try {
        return await invoke('toggle_task_complete', { id });
      } catch (e) {
        console.error('Failed to toggle task:', e);
        throw e;
      }
    },
  },

  app: {
    hide: async (): Promise<void> => {
      try {
        const win = getCurrentWindow();
        await win.hide();
      } catch (e) {
        console.error('Failed to hide window:', e);
      }
    },
    show: async (): Promise<void> => {
      try {
        const win = getCurrentWindow();
        await win.show();
        await win.setFocus();
      } catch (e) {
        console.error('Failed to show window:', e);
      }
    },
    minimize: async (): Promise<void> => {
      try {
        const win = getCurrentWindow();
        await win.minimize();
      } catch (e) {
        console.error('Failed to minimize window:', e);
      }
    },
    close: async (): Promise<void> => {
      try {
        const win = getCurrentWindow();
        await win.hide();
      } catch (e) {
        console.error('Failed to close window:', e);
      }
    },
    quit: async (): Promise<void> => {
      try {
        const win = getCurrentWindow();
        await win.hide();
      } catch (e) {
        console.error('Failed to quit:', e);
      }
    },
    forceQuit: async (): Promise<void> => {
      try {
        const win = getCurrentWindow();
        await win.destroy();
      } catch (e) {
        console.error('Failed to force quit:', e);
      }
    },
    getMode: async (): Promise<'overlay' | 'full'> => {
      return 'full';
    },
    setMode: async (_mode: 'overlay' | 'full'): Promise<'overlay' | 'full'> => {
      return 'full';
    },
    getSettings: async (): Promise<AppSettings> => {
      return {
        launchOnStartup: false,
        globalShortcut: 'Command+Shift+P',
        soundEnabled: true,
        theme: 'dark',
      };
    },
    setSettings: async (_key: string, _value: unknown): Promise<AppSettings> => {
      return {
        launchOnStartup: false,
        globalShortcut: 'Command+Shift+P',
        soundEnabled: true,
        theme: 'dark',
      };
    },
    getCurrentShortcut: async (): Promise<string | null> => {
      return 'Command+Shift+P';
    },
  },

  notification: {
    show: async (title: string, body: string, _taskId?: string): Promise<void> => {
      if ('Notification' in window && Notification.permission === 'granted') {
        new Notification(title, { body });
      }
    },
  },

  reminder: {
    snooze: async (_taskId: string, _minutes: number): Promise<void> => {},
    markDone: async (taskId: string): Promise<void> => {
      await invoke('toggle_task_complete', { id: taskId });
    },
  },

  on: (_channel: string, _callback: (...args: unknown[]) => void) => {
    // Tauri events not implemented yet
  },
};

// Always use Tauri API since we're building for Tauri now
export const getApi = () => {
  return tauriApi;
};
