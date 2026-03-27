import { create } from 'zustand';
import type { List, Task, AppSettings, ListReminder } from '../../shared/types';
import { getApi, isTauri } from './tauri-api';

interface AppState {
  // Data
  lists: List[];
  tasks: Task[];
  settings: AppSettings;

  // UI State
  selectedListId: string | null;
  selectedTaskIndex: number;
  searchQuery: string;
  view: 'command' | 'lists' | 'settings' | 'list-settings';
  isInputFocused: boolean;

  // Actions
  setLists: (lists: List[]) => void;
  setTasks: (tasks: Task[]) => void;
  setSettings: (settings: AppSettings) => void;
  setSelectedListId: (id: string | null) => void;
  setSelectedTaskIndex: (index: number) => void;
  setSearchQuery: (query: string) => void;
  setView: (view: 'command' | 'lists' | 'settings' | 'list-settings') => void;
  setInputFocused: (focused: boolean) => void;

  // Data operations
  loadData: () => Promise<void>;
  refreshLists: () => Promise<void>;
  createList: (name: string, color?: string) => Promise<List>;
  updateList: (id: string, updates: { name?: string; color?: string; reminder?: ListReminder | null }) => Promise<void>;
  deleteList: (id: string) => Promise<void>;
  createTask: (task: {
    list_id: string;
    content: string;
    due_at?: number;
    ping_interval?: number;
    parent_id?: string;
  }) => Promise<Task>;
  updateTask: (
    id: string,
    updates: Partial<{
      content: string;
      completed: boolean;
      due_at: number | null;
      ping_interval: number | null;
    }>
  ) => Promise<void>;
  deleteTask: (id: string) => Promise<void>;
  toggleTaskComplete: (id: string) => Promise<void>;
  refreshTasks: () => Promise<void>;
}

// Get the API (works with both Tauri and Electron)
const api = getApi();

export const useStore = create<AppState>((set, get) => ({
  // Initial state
  lists: [],
  tasks: [],
  settings: {
    launchOnStartup: false,
    globalShortcut: 'CommandOrControl+Shift+P',
    soundEnabled: true,
    theme: 'dark',
  },
  selectedListId: null,
  selectedTaskIndex: 0,
  searchQuery: '',
  view: 'command',
  isInputFocused: true,

  // Setters
  setLists: (lists) => set({ lists }),
  setTasks: (tasks) => set({ tasks }),
  setSettings: (settings) => set({ settings }),
  setSelectedListId: (id) => set({ selectedListId: id, selectedTaskIndex: 0 }),
  setSelectedTaskIndex: (index) => set({ selectedTaskIndex: index }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setView: (view) => set({ view }),
  setInputFocused: (focused) => set({ isInputFocused: focused }),

  // Data operations
  loadData: async () => {
    try {
      const [lists, tasks, settings] = await Promise.all([
        api.lists.getAll(),
        api.tasks.getAll(),
        api.app.getSettings(),
      ]);

      const defaultListId = lists.length > 0 ? lists[0].id : null;

      set({
        lists,
        tasks,
        settings,
        selectedListId: get().selectedListId || defaultListId,
      });
    } catch (error) {
      console.error('Failed to load data:', error);
    }
  },

  refreshLists: async () => {
    const lists = await api.lists.getAll();
    set({ lists });
  },

  createList: async (name, color) => {
    const list = await api.lists.create(name, color);
    set((state) => ({ lists: [...state.lists, list] }));
    return list;
  },

  updateList: async (id, updates) => {
    const updatedList = await api.lists.update(id, updates);
    if (updatedList) {
      set((state) => ({
        lists: state.lists.map((l) => (l.id === id ? updatedList : l)),
      }));
    }
  },

  deleteList: async (id) => {
    try {
      console.log('Store: Deleting list', id);
      await api.lists.delete(id);
      console.log('Store: Delete API call succeeded');

      const currentLists = get().lists;
      const newLists = currentLists.filter((l) => l.id !== id);

      // If this was the last list, create a new default Inbox
      if (newLists.length === 0) {
        console.log('Store: Last list deleted, creating new Inbox');
        const inbox = await api.lists.create('Inbox', '#4a9f6e');
        set({
          lists: [inbox],
          tasks: [],
          selectedListId: inbox.id,
        });
      } else {
        set((state) => ({
          lists: newLists,
          tasks: state.tasks.filter((t) => t.list_id !== id),
          selectedListId:
            state.selectedListId === id ? newLists[0].id : state.selectedListId,
        }));
      }
    } catch (error) {
      console.error('Store: Failed to delete list:', error);
      throw error;
    }
  },

  createTask: async (task) => {
    const newTask = await api.tasks.create(task);
    set((state) => ({ tasks: [newTask, ...state.tasks] }));
    return newTask;
  },

  updateTask: async (id, updates) => {
    await api.tasks.update(id, updates);
    set((state) => ({
      tasks: state.tasks.map((t) => {
        if (t.id !== id) return t;
        // Convert null to undefined for optional fields
        const cleanUpdates: Partial<Task> = {};
        if (updates.content !== undefined) cleanUpdates.content = updates.content;
        if (updates.completed !== undefined) cleanUpdates.completed = updates.completed;
        if (updates.due_at !== undefined) cleanUpdates.due_at = updates.due_at ?? undefined;
        if (updates.ping_interval !== undefined) cleanUpdates.ping_interval = updates.ping_interval ?? undefined;
        return { ...t, ...cleanUpdates };
      }),
    }));
  },

  deleteTask: async (id) => {
    await api.tasks.delete(id);
    set((state) => ({
      tasks: state.tasks.filter((t) => t.id !== id && t.parent_id !== id),
    }));
  },

  toggleTaskComplete: async (id) => {
    const task = get().tasks.find((t) => t.id === id);
    if (!task) return;

    await api.tasks.toggleComplete(id);
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === id ? { ...t, completed: !t.completed } : t
      ),
    }));
  },

  refreshTasks: async () => {
    const tasks = await api.tasks.getAll();
    set({ tasks });
  },
}));

// Subscribe to updates from main process (for Electron compatibility)
if (typeof window !== 'undefined' && !isTauri()) {
  const electronApi = (window as { pingpal?: typeof api }).pingpal;
  if (electronApi) {
    electronApi.on('task:updated', () => {
      useStore.getState().refreshTasks();
    });

    electronApi.on('list:updated', () => {
      useStore.getState().refreshLists();
    });

    electronApi.on('navigate:list', (listId) => {
      useStore.getState().setSelectedListId(listId as string);
      useStore.getState().setView('command');
    });
  }
}
