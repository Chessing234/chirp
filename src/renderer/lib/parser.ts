import type { ParsedTask } from '../../shared/types';

interface TimePattern {
  regex: RegExp;
  handler: (match: RegExpMatchArray, baseDate: Date) => Date;
}

const TIME_PATTERNS: TimePattern[] = [
  // "tomorrow 5pm"
  {
    regex: /\btomorrow\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b/i,
    handler: (match, base) => {
      const date = new Date(base);
      date.setDate(date.getDate() + 1);
      let hours = parseInt(match[1]);
      const minutes = match[2] ? parseInt(match[2]) : 0;
      const period = match[3]?.toLowerCase();

      if (period === 'pm' && hours !== 12) hours += 12;
      if (period === 'am' && hours === 12) hours = 0;
      if (!period && hours < 8) hours += 12; // Assume PM for low numbers without period

      date.setHours(hours, minutes, 0, 0);
      return date;
    },
  },
  // "tomorrow"
  {
    regex: /\btomorrow\b/i,
    handler: (_, base) => {
      const date = new Date(base);
      date.setDate(date.getDate() + 1);
      date.setHours(9, 0, 0, 0); // Default 9 AM
      return date;
    },
  },
  // "today 5pm"
  {
    regex: /\btoday\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b/i,
    handler: (match, base) => {
      const date = new Date(base);
      let hours = parseInt(match[1]);
      const minutes = match[2] ? parseInt(match[2]) : 0;
      const period = match[3]?.toLowerCase();

      if (period === 'pm' && hours !== 12) hours += 12;
      if (period === 'am' && hours === 12) hours = 0;
      if (!period && hours < 8) hours += 12;

      date.setHours(hours, minutes, 0, 0);
      return date;
    },
  },
  // "at 5pm" or "5pm"
  {
    regex: /\b(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b/i,
    handler: (match, base) => {
      const date = new Date(base);
      let hours = parseInt(match[1]);
      const minutes = match[2] ? parseInt(match[2]) : 0;
      const period = match[3].toLowerCase();

      if (period === 'pm' && hours !== 12) hours += 12;
      if (period === 'am' && hours === 12) hours = 0;

      // If time has passed, schedule for tomorrow
      if (date.getHours() > hours || (date.getHours() === hours && date.getMinutes() >= minutes)) {
        date.setDate(date.getDate() + 1);
      }

      date.setHours(hours, minutes, 0, 0);
      return date;
    },
  },
  // "in 30m" or "in 2h"
  {
    regex: /\bin\s+(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b/i,
    handler: (match, base) => {
      const date = new Date(base);
      const amount = parseInt(match[1]);
      const unit = match[2].toLowerCase();

      if (unit.startsWith('m')) {
        date.setMinutes(date.getMinutes() + amount);
      } else {
        date.setHours(date.getHours() + amount);
      }
      return date;
    },
  },
  // "next week"
  {
    regex: /\bnext\s+week\b/i,
    handler: (_, base) => {
      const date = new Date(base);
      date.setDate(date.getDate() + 7);
      date.setHours(9, 0, 0, 0);
      return date;
    },
  },
  // Day names: "monday", "tuesday", etc.
  {
    regex: /\b(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b/i,
    handler: (match, base) => {
      const days = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];
      const targetDay = days.indexOf(match[1].toLowerCase());
      const date = new Date(base);
      const currentDay = date.getDay();
      let daysToAdd = targetDay - currentDay;
      if (daysToAdd <= 0) daysToAdd += 7;
      date.setDate(date.getDate() + daysToAdd);
      date.setHours(9, 0, 0, 0);
      return date;
    },
  },
];

const PING_PATTERNS = [
  // "ping every 30m" or "ping 30m"
  {
    regex: /\bping\s+(?:every\s+)?(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b/i,
    handler: (match: RegExpMatchArray): number => {
      const amount = parseInt(match[1]);
      const unit = match[2].toLowerCase();
      return unit.startsWith('m') ? amount : amount * 60;
    },
  },
  // "every 2 hours"
  {
    regex: /\bevery\s+(\d+)\s*(m|min|mins|minutes?|h|hr|hrs|hours?)\b/i,
    handler: (match: RegExpMatchArray): number => {
      const amount = parseInt(match[1]);
      const unit = match[2].toLowerCase();
      return unit.startsWith('m') ? amount : amount * 60;
    },
  },
];

export function parseTaskInput(input: string): ParsedTask {
  const now = new Date();
  let content = input.trim();
  let due_at: Date | undefined;
  let ping_interval: number | undefined;

  // Extract time
  for (const pattern of TIME_PATTERNS) {
    const match = content.match(pattern.regex);
    if (match) {
      due_at = pattern.handler(match, now);
      content = content.replace(pattern.regex, '').trim();
      break;
    }
  }

  // Extract ping interval
  for (const pattern of PING_PATTERNS) {
    const match = content.match(pattern.regex);
    if (match) {
      ping_interval = pattern.handler(match);
      content = content.replace(pattern.regex, '').trim();
      break;
    }
  }

  // Clean up extra spaces
  content = content.replace(/\s+/g, ' ').trim();

  return {
    content,
    due_at,
    ping_interval,
  };
}

export function formatDueDate(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);

  const isToday = date.toDateString() === now.toDateString();
  const isTomorrow = date.toDateString() === tomorrow.toDateString();

  const timeStr = date.toLocaleTimeString('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
  });

  if (isToday) {
    return `Today ${timeStr}`;
  } else if (isTomorrow) {
    return `Tomorrow ${timeStr}`;
  } else {
    const dateStr = date.toLocaleDateString('en-US', {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
    });
    return `${dateStr} ${timeStr}`;
  }
}

export function formatPingInterval(minutes: number): string {
  if (minutes < 60) {
    return `${minutes}m`;
  } else {
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
  }
}

export function getRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;
  return new Date(timestamp).toLocaleDateString();
}
