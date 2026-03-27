export type ReminderType = 'none' | 'interval' | 'daily' | 'weekly' | 'activity';

export interface ListReminder {
  type: ReminderType;
  interval?: number; // minutes for 'interval' type
  time?: string; // HH:MM for 'daily' and 'weekly'
  days?: number[]; // 0-6 (Sun-Sat) for 'weekly'
  activityOnly?: boolean; // only remind when user is active
  enabled: boolean;
}

export interface List {
  id: string;
  name: string;
  color?: string;
  reminder?: ListReminder;
  created_at: number;
  updated_at: number;
}

export interface Task {
  id: string;
  list_id: string;
  content: string;
  completed: boolean;
  due_at?: number;
  ping_interval?: number; // in minutes (task-specific override)
  last_ping_at?: number;
  parent_id?: string; // for subtasks
  created_at: number;
  updated_at: number;
}

export interface ParsedTask {
  content: string;
  due_at?: Date;
  ping_interval?: number;
}

export interface AppSettings {
  launchOnStartup: boolean;
  globalShortcut: string;
  soundEnabled: boolean;
  theme: 'dark' | 'light';
}

export type IpcChannels =
  | 'db:lists:getAll'
  | 'db:lists:create'
  | 'db:lists:update'
  | 'db:lists:delete'
  | 'db:tasks:getAll'
  | 'db:tasks:getByList'
  | 'db:tasks:create'
  | 'db:tasks:update'
  | 'db:tasks:delete'
  | 'db:tasks:toggleComplete'
  | 'app:hide'
  | 'app:show'
  | 'app:quit'
  | 'app:getSettings'
  | 'app:setSettings'
  | 'notification:show'
  | 'notification:action'
  | 'reminder:snooze'
  | 'reminder:markDone';
