import { powerMonitor } from 'electron';
import type { Task, List, ListReminder } from '../shared/types';
import type { Database } from './database';

interface ReminderCallback {
  onTaskReminder: (task: Task) => void;
  onListReminder: (list: List, pendingCount: number) => void;
}

export class ReminderEngine {
  private db: Database;
  private callbacks: ReminderCallback;
  private intervalId: NodeJS.Timeout | null = null;
  private activityIntervalId: NodeJS.Timeout | null = null;
  private snoozedTasks: Map<string, number> = new Map();
  private snoozedLists: Map<string, number> = new Map();
  private lastListPings: Map<string, number> = new Map();
  private isUserActive: boolean = true;
  private isStopped: boolean = false;
  private readonly CHECK_INTERVAL = 30000; // 30 seconds
  private readonly ACTIVITY_IDLE_THRESHOLD = 300; // 5 minutes in seconds

  constructor(db: Database, callbacks: ReminderCallback) {
    this.db = db;
    this.callbacks = callbacks;
  }

  private setupActivityMonitor(): void {
    if (this.activityIntervalId) return;

    // Check user activity state periodically
    this.activityIntervalId = setInterval(() => {
      if (this.isStopped) return;
      try {
        const idleTime = powerMonitor.getSystemIdleTime();
        this.isUserActive = idleTime < this.ACTIVITY_IDLE_THRESHOLD;
      } catch {
        // powerMonitor may not be available, default to active
        this.isUserActive = true;
      }
    }, 10000); // Check every 10 seconds
  }

  start(): void {
    if (this.intervalId) return;
    this.isStopped = false;

    this.setupActivityMonitor();

    this.intervalId = setInterval(() => {
      if (!this.isStopped) {
        this.checkReminders();
      }
    }, this.CHECK_INTERVAL);

    // Run immediately on start
    this.checkReminders();
  }

  stop(): void {
    this.isStopped = true;

    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }

    if (this.activityIntervalId) {
      clearInterval(this.activityIntervalId);
      this.activityIntervalId = null;
    }
  }

  snoozeTask(taskId: string, minutes: number): void {
    const snoozeUntil = Date.now() + minutes * 60 * 1000;
    this.snoozedTasks.set(taskId, snoozeUntil);
  }

  snoozeList(listId: string, minutes: number): void {
    const snoozeUntil = Date.now() + minutes * 60 * 1000;
    this.snoozedLists.set(listId, snoozeUntil);
  }

  private checkReminders(): void {
    this.checkTaskReminders();
    this.checkListReminders();
  }

  private checkTaskReminders(): void {
    const now = Date.now();
    const tasks = this.db.getPendingTasksWithReminders();

    for (const task of tasks) {
      // Skip snoozed tasks
      const snoozeUntil = this.snoozedTasks.get(task.id);
      if (snoozeUntil && now < snoozeUntil) {
        continue;
      }

      // Clear expired snooze
      if (snoozeUntil && now >= snoozeUntil) {
        this.snoozedTasks.delete(task.id);
      }

      let shouldNotify = false;

      // Check due_at
      if (task.due_at && now >= task.due_at) {
        // Only notify once for due tasks (within 5 minutes of due time)
        const timeSinceDue = now - task.due_at;
        if (timeSinceDue < 5 * 60 * 1000) {
          shouldNotify = true;
        }
      }

      // Check ping interval
      if (task.ping_interval) {
        const pingIntervalMs = task.ping_interval * 60 * 1000;
        const lastPing = task.last_ping_at || task.created_at;
        const timeSinceLastPing = now - lastPing;

        if (timeSinceLastPing >= pingIntervalMs) {
          shouldNotify = true;
        }
      }

      if (shouldNotify) {
        this.callbacks.onTaskReminder(task);
      }
    }
  }

  private checkListReminders(): void {
    const now = Date.now();
    const lists = this.db.getListsWithReminders();

    for (const list of lists) {
      if (!list.reminder || !list.reminder.enabled) continue;

      // Skip snoozed lists
      const snoozeUntil = this.snoozedLists.get(list.id);
      if (snoozeUntil && now < snoozeUntil) {
        continue;
      }

      // Clear expired snooze
      if (snoozeUntil && now >= snoozeUntil) {
        this.snoozedLists.delete(list.id);
      }

      // Check if user needs to be active for this reminder
      if (list.reminder.activityOnly && !this.isUserActive) {
        continue;
      }

      const shouldNotify = this.shouldNotifyForList(list, list.reminder, now);

      if (shouldNotify) {
        const tasks = this.db.getTasksByList(list.id);
        const pendingCount = tasks.filter(t => !t.completed).length;

        if (pendingCount > 0) {
          this.callbacks.onListReminder(list, pendingCount);
          this.lastListPings.set(list.id, now);
        }
      }
    }
  }

  private shouldNotifyForList(list: List, reminder: ListReminder, now: number): boolean {
    const lastPing = this.lastListPings.get(list.id) || 0;

    switch (reminder.type) {
      case 'interval': {
        if (!reminder.interval) return false;
        const intervalMs = reminder.interval * 60 * 1000;
        return (now - lastPing) >= intervalMs;
      }

      case 'daily': {
        if (!reminder.time) return false;
        return this.checkDailyReminder(reminder.time, lastPing, now);
      }

      case 'weekly': {
        if (!reminder.time || !reminder.days || reminder.days.length === 0) return false;
        return this.checkWeeklyReminder(reminder.time, reminder.days, lastPing, now);
      }

      case 'activity': {
        // Activity-based: remind every hour while user is active
        const intervalMs = (reminder.interval || 60) * 60 * 1000;
        return this.isUserActive && (now - lastPing) >= intervalMs;
      }

      default:
        return false;
    }
  }

  private checkDailyReminder(time: string, lastPing: number, now: number): boolean {
    const [hours, minutes] = time.split(':').map(Number);
    const today = new Date(now);

    // Create target time for today
    const targetTime = new Date(today);
    targetTime.setHours(hours, minutes, 0, 0);

    // Check if we're within 2 minutes of the target time
    const diff = now - targetTime.getTime();
    const isWithinWindow = diff >= 0 && diff < 2 * 60 * 1000;

    // Check if we already pinged today
    const lastPingDate = new Date(lastPing);
    const alreadyPingedToday = lastPingDate.toDateString() === today.toDateString() &&
                               lastPing >= targetTime.getTime();

    return isWithinWindow && !alreadyPingedToday;
  }

  private checkWeeklyReminder(time: string, days: number[], lastPing: number, now: number): boolean {
    const today = new Date(now);
    const currentDay = today.getDay(); // 0 = Sunday, 6 = Saturday

    // Check if today is one of the reminder days
    if (!days.includes(currentDay)) {
      return false;
    }

    // Use daily reminder logic for the time check
    return this.checkDailyReminder(time, lastPing, now);
  }

  // Get user activity status
  isActive(): boolean {
    return this.isUserActive;
  }
}
