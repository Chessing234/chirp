import BetterSqlite3 from 'better-sqlite3';
import { app } from 'electron';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';
import type { List, Task, ListReminder } from '../shared/types';

// Internal type for raw database rows
interface TaskRow {
  id: string;
  list_id: string;
  content: string;
  completed: number;
  due_at: number | null;
  ping_interval: number | null;
  last_ping_at: number | null;
  parent_id: string | null;
  created_at: number;
  updated_at: number;
}

interface ListRow {
  id: string;
  name: string;
  color: string | null;
  reminder: string | null;
  created_at: number;
  updated_at: number;
}

function rowToTask(row: TaskRow): Task {
  return {
    id: row.id,
    list_id: row.list_id,
    content: row.content,
    completed: Boolean(row.completed),
    due_at: row.due_at ?? undefined,
    ping_interval: row.ping_interval ?? undefined,
    last_ping_at: row.last_ping_at ?? undefined,
    parent_id: row.parent_id ?? undefined,
    created_at: row.created_at,
    updated_at: row.updated_at,
  };
}

function rowToList(row: ListRow): List {
  let reminder: ListReminder | undefined;
  if (row.reminder) {
    try {
      reminder = JSON.parse(row.reminder);
    } catch {
      reminder = undefined;
    }
  }
  return {
    id: row.id,
    name: row.name,
    color: row.color ?? undefined,
    reminder,
    created_at: row.created_at,
    updated_at: row.updated_at,
  };
}

export class Database {
  private db: BetterSqlite3.Database;

  constructor() {
    const dbPath = path.join(app.getPath('userData'), 'pingpal.db');
    this.db = new BetterSqlite3(dbPath);
    this.db.pragma('journal_mode = WAL');
    this.initialize();
  }

  private initialize(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS lists (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        color TEXT,
        reminder TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        list_id TEXT NOT NULL,
        content TEXT NOT NULL,
        completed INTEGER NOT NULL DEFAULT 0,
        due_at INTEGER,
        ping_interval INTEGER,
        last_ping_at INTEGER,
        parent_id TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (list_id) REFERENCES lists(id) ON DELETE CASCADE,
        FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
      );

      CREATE INDEX IF NOT EXISTS idx_tasks_list_id ON tasks(list_id);
      CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks(parent_id);
      CREATE INDEX IF NOT EXISTS idx_tasks_due_at ON tasks(due_at);
      CREATE INDEX IF NOT EXISTS idx_tasks_completed ON tasks(completed);
    `);

    // Migration: Add reminder column if it doesn't exist
    this.migrate();
  }

  private migrate(): void {
    // Check if reminder column exists in lists table
    const tableInfo = this.db.prepare("PRAGMA table_info(lists)").all() as { name: string }[];
    const hasReminderColumn = tableInfo.some(col => col.name === 'reminder');

    if (!hasReminderColumn) {
      this.db.exec('ALTER TABLE lists ADD COLUMN reminder TEXT');
    }
  }

  // List operations
  getAllLists(): List[] {
    const stmt = this.db.prepare('SELECT * FROM lists ORDER BY created_at ASC');
    const rows = stmt.all() as ListRow[];
    return rows.map(rowToList);
  }

  getList(id: string): List | undefined {
    const stmt = this.db.prepare('SELECT * FROM lists WHERE id = ?');
    const row = stmt.get(id) as ListRow | undefined;
    return row ? rowToList(row) : undefined;
  }

  createList(name: string, color?: string, reminder?: ListReminder): List {
    const id = uuidv4();
    const now = Date.now();
    const reminderJson = reminder ? JSON.stringify(reminder) : null;
    const stmt = this.db.prepare(
      'INSERT INTO lists (id, name, color, reminder, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)'
    );
    stmt.run(id, name, color || null, reminderJson, now, now);
    return { id, name, color, reminder, created_at: now, updated_at: now };
  }

  updateList(id: string, updates: { name?: string; color?: string; reminder?: ListReminder | null }): List | undefined {
    const list = this.getList(id);
    if (!list) return undefined;

    const now = Date.now();
    const newName = updates.name ?? list.name;
    const newColor = updates.color ?? list.color;
    const newReminder = updates.reminder === null ? null : (updates.reminder ?? list.reminder);
    const reminderJson = newReminder ? JSON.stringify(newReminder) : null;

    const stmt = this.db.prepare(
      'UPDATE lists SET name = ?, color = ?, reminder = ?, updated_at = ? WHERE id = ?'
    );
    stmt.run(newName, newColor, reminderJson, now, id);
    return { ...list, name: newName, color: newColor, reminder: newReminder ?? undefined, updated_at: now };
  }

  getListsWithReminders(): List[] {
    const stmt = this.db.prepare('SELECT * FROM lists WHERE reminder IS NOT NULL ORDER BY created_at ASC');
    const rows = stmt.all() as ListRow[];
    return rows.map(rowToList).filter(list => list.reminder?.enabled);
  }

  deleteList(id: string): boolean {
    // Delete all tasks in the list first
    const deleteTasksStmt = this.db.prepare('DELETE FROM tasks WHERE list_id = ?');
    deleteTasksStmt.run(id);

    const stmt = this.db.prepare('DELETE FROM lists WHERE id = ?');
    const result = stmt.run(id);
    return result.changes > 0;
  }

  // Task operations
  getAllTasks(): Task[] {
    const stmt = this.db.prepare('SELECT * FROM tasks ORDER BY created_at DESC');
    const rows = stmt.all() as TaskRow[];
    return rows.map(rowToTask);
  }

  getTasksByList(listId: string): Task[] {
    const stmt = this.db.prepare(
      'SELECT * FROM tasks WHERE list_id = ? ORDER BY completed ASC, created_at DESC'
    );
    const rows = stmt.all(listId) as TaskRow[];
    return rows.map(rowToTask);
  }

  getTask(id: string): Task | undefined {
    const stmt = this.db.prepare('SELECT * FROM tasks WHERE id = ?');
    const row = stmt.get(id) as TaskRow | undefined;
    if (row) {
      return rowToTask(row);
    }
    return undefined;
  }

  getPendingTasksWithReminders(): Task[] {
    const stmt = this.db.prepare(`
      SELECT * FROM tasks
      WHERE completed = 0
      AND (due_at IS NOT NULL OR ping_interval IS NOT NULL)
      ORDER BY due_at ASC
    `);
    const rows = stmt.all() as TaskRow[];
    return rows.map(rowToTask);
  }

  createTask(task: {
    list_id: string;
    content: string;
    due_at?: number;
    ping_interval?: number;
    parent_id?: string;
  }): Task {
    const id = uuidv4();
    const now = Date.now();

    const stmt = this.db.prepare(`
      INSERT INTO tasks (id, list_id, content, completed, due_at, ping_interval, parent_id, created_at, updated_at)
      VALUES (?, ?, ?, 0, ?, ?, ?, ?, ?)
    `);
    stmt.run(
      id,
      task.list_id,
      task.content,
      task.due_at || null,
      task.ping_interval || null,
      task.parent_id || null,
      now,
      now
    );

    return {
      id,
      list_id: task.list_id,
      content: task.content,
      completed: false,
      due_at: task.due_at,
      ping_interval: task.ping_interval,
      parent_id: task.parent_id,
      created_at: now,
      updated_at: now,
    };
  }

  updateTask(
    id: string,
    updates: Partial<{
      content: string;
      completed: boolean;
      due_at: number | null;
      ping_interval: number | null;
      last_ping_at: number | null;
    }>
  ): Task | undefined {
    const task = this.getTask(id);
    if (!task) return undefined;

    const now = Date.now();
    const fields: string[] = ['updated_at = ?'];
    const values: (string | number | null)[] = [now];

    if (updates.content !== undefined) {
      fields.push('content = ?');
      values.push(updates.content);
    }
    if (updates.completed !== undefined) {
      fields.push('completed = ?');
      values.push(updates.completed ? 1 : 0);
    }
    if (updates.due_at !== undefined) {
      fields.push('due_at = ?');
      values.push(updates.due_at);
    }
    if (updates.ping_interval !== undefined) {
      fields.push('ping_interval = ?');
      values.push(updates.ping_interval);
    }
    if (updates.last_ping_at !== undefined) {
      fields.push('last_ping_at = ?');
      values.push(updates.last_ping_at);
    }

    values.push(id);
    const stmt = this.db.prepare(`UPDATE tasks SET ${fields.join(', ')} WHERE id = ?`);
    stmt.run(...values);

    return this.getTask(id);
  }

  toggleTaskComplete(id: string): Task | undefined {
    const task = this.getTask(id);
    if (!task) return undefined;

    const now = Date.now();
    const stmt = this.db.prepare('UPDATE tasks SET completed = ?, updated_at = ? WHERE id = ?');
    stmt.run(task.completed ? 0 : 1, now, id);

    return this.getTask(id);
  }

  deleteTask(id: string): boolean {
    // Delete subtasks first
    const deleteSubtasksStmt = this.db.prepare('DELETE FROM tasks WHERE parent_id = ?');
    deleteSubtasksStmt.run(id);

    const stmt = this.db.prepare('DELETE FROM tasks WHERE id = ?');
    const result = stmt.run(id);
    return result.changes > 0;
  }

  searchTasks(query: string): Task[] {
    const stmt = this.db.prepare(`
      SELECT * FROM tasks
      WHERE content LIKE ?
      ORDER BY completed ASC, created_at DESC
      LIMIT 50
    `);
    const rows = stmt.all(`%${query}%`) as TaskRow[];
    return rows.map(rowToTask);
  }

  close(): void {
    this.db.close();
  }
}
