import { useState, useEffect } from 'react';
import type { List, ListReminder, ReminderType } from '../../shared/types';
import { useStore } from '../lib/store';

interface ListReminderSettingsProps {
  list: List;
  onBack: () => void;
}

const DAYS = [
  { value: 0, label: 'Sun' },
  { value: 1, label: 'Mon' },
  { value: 2, label: 'Tue' },
  { value: 3, label: 'Wed' },
  { value: 4, label: 'Thu' },
  { value: 5, label: 'Fri' },
  { value: 6, label: 'Sat' },
];

const REMINDER_TYPES: { value: ReminderType; label: string; description: string }[] = [
  { value: 'none', label: 'No reminders', description: 'Disable reminders for this list' },
  { value: 'interval', label: 'Every X hours', description: 'Remind at regular intervals' },
  { value: 'daily', label: 'Daily at time', description: 'Remind once per day at a specific time' },
  { value: 'weekly', label: 'Weekly on days', description: 'Remind on specific days of the week' },
  { value: 'activity', label: 'While active', description: 'Remind only when using the laptop' },
];

// Quick preset options for convenience
const QUICK_PRESETS = [
  { value: 5, label: '5m' },
  { value: 15, label: '15m' },
  { value: 30, label: '30m' },
  { value: 60, label: '1h' },
  { value: 120, label: '2h' },
  { value: 240, label: '4h' },
];

// Format minutes to human readable string
function formatInterval(minutes: number | undefined): string {
  const mins = minutes || 60;
  if (mins < 60) {
    return `${mins} min`;
  }
  const hours = Math.floor(mins / 60);
  const remainder = mins % 60;
  if (remainder === 0) {
    return `${hours} hour${hours > 1 ? 's' : ''}`;
  }
  return `${hours}h ${remainder}m`;
}

export function ListReminderSettings({ list, onBack }: ListReminderSettingsProps) {
  const updateList = useStore((s) => s.updateList);

  const [reminder, setReminder] = useState<ListReminder>(() => {
    return list.reminder || {
      type: 'none',
      enabled: false,
    };
  });

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onBack();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onBack]);

  const handleSave = async () => {
    const updatedReminder: ListReminder = {
      ...reminder,
      enabled: reminder.type !== 'none',
    };

    await updateList(list.id, { reminder: updatedReminder });
    onBack();
  };

  const handleTypeChange = (type: ReminderType) => {
    setReminder((prev) => ({
      ...prev,
      type,
      enabled: type !== 'none',
      // Set defaults for each type
      interval: type === 'interval' ? 60 : type === 'activity' ? 60 : prev.interval,
      time: type === 'daily' || type === 'weekly' ? prev.time || '09:00' : prev.time,
      days: type === 'weekly' ? prev.days || [1, 2, 3, 4, 5] : prev.days,
    }));
  };

  const toggleDay = (day: number) => {
    setReminder((prev) => {
      const currentDays = prev.days || [];
      const newDays = currentDays.includes(day)
        ? currentDays.filter((d) => d !== day)
        : [...currentDays, day].sort();
      return { ...prev, days: newDays };
    });
  };

  return (
    <div className="animate-slide-up flex flex-col h-full">
      {/* Header */}
      <div className="px-4 py-3 border-b border-ping-border/50 flex items-center gap-3 flex-shrink-0">
        <button
          onClick={onBack}
          className="p-1 rounded hover:bg-ping-elevated transition-colors"
        >
          <svg className="w-5 h-5 text-ping-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <div className="flex items-center gap-2">
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: list.color || '#4a9f6e' }}
          />
          <h2 className="text-sm font-semibold text-ping-text">{list.name} - Reminders</h2>
        </div>
      </div>

      {/* Content */}
      <div className="p-4 space-y-6 flex-1 overflow-y-auto">
        {/* Reminder Type */}
        <div>
          <div className="text-sm font-medium text-ping-text mb-3">Reminder Type</div>
          <div className="space-y-2">
            {REMINDER_TYPES.map((type) => (
              <button
                key={type.value}
                onClick={() => handleTypeChange(type.value)}
                className={`
                  w-full flex items-start gap-3 px-3 py-2.5 rounded-lg text-left transition-colors
                  ${reminder.type === type.value
                    ? 'bg-ping-accent/10 border border-ping-accent/30'
                    : 'bg-ping-elevated border border-ping-border hover:border-ping-accent/20'}
                `}
              >
                <div className={`
                  w-4 h-4 rounded-full border-2 flex-shrink-0 mt-0.5
                  ${reminder.type === type.value
                    ? 'border-ping-accent bg-ping-accent'
                    : 'border-ping-muted'}
                `}>
                  {reminder.type === type.value && (
                    <svg className="w-full h-full text-ping-bg" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                    </svg>
                  )}
                </div>
                <div>
                  <div className="text-sm font-medium text-ping-text">{type.label}</div>
                  <div className="text-xs text-ping-muted mt-0.5">{type.description}</div>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Interval Setting */}
        {(reminder.type === 'interval' || reminder.type === 'activity') && (
          <div>
            <div className="text-sm font-medium text-ping-text mb-2">
              {reminder.type === 'activity' ? 'Remind every' : 'Interval'}
            </div>

            {/* Current value display */}
            <div className="text-center mb-3">
              <span className="text-2xl font-bold text-ping-accent">
                {formatInterval(reminder.interval || 60)}
              </span>
            </div>

            {/* Range slider */}
            <div className="px-1 mb-4">
              <input
                type="range"
                min="1"
                max="720"
                value={reminder.interval || 60}
                onChange={(e) => {
                  const val = parseInt(e.target.value, 10);
                  if (!isNaN(val)) {
                    setReminder((prev) => ({ ...prev, interval: val }));
                  }
                }}
                className="w-full h-2 bg-ping-border rounded-lg appearance-none cursor-pointer
                         [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-5
                         [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:rounded-full
                         [&::-webkit-slider-thumb]:bg-ping-accent [&::-webkit-slider-thumb]:cursor-pointer
                         [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:border-2
                         [&::-webkit-slider-thumb]:border-ping-bg
                         [&::-moz-range-thumb]:w-5 [&::-moz-range-thumb]:h-5
                         [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-ping-accent
                         [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:border-2
                         [&::-moz-range-thumb]:border-ping-bg"
              />
              <div className="flex justify-between text-xs text-ping-muted mt-1">
                <span>1 min</span>
                <span>12 hours</span>
              </div>
            </div>

            {/* Quick presets */}
            <div className="flex gap-2 flex-wrap">
              {QUICK_PRESETS.map((preset) => (
                <button
                  key={preset.value}
                  onClick={() => setReminder((prev) => ({ ...prev, interval: preset.value }))}
                  className={`
                    px-3 py-1.5 rounded-lg text-xs font-medium transition-colors
                    ${reminder.interval === preset.value
                      ? 'bg-ping-accent text-ping-bg'
                      : 'bg-ping-elevated text-ping-muted hover:text-ping-text hover:bg-ping-border'}
                  `}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Time Setting */}
        {(reminder.type === 'daily' || reminder.type === 'weekly') && (
          <div>
            <div className="text-sm font-medium text-ping-text mb-3">Time</div>
            <input
              type="time"
              value={reminder.time || '09:00'}
              onChange={(e) => setReminder((prev) => ({ ...prev, time: e.target.value }))}
              className="w-full px-4 py-2.5 bg-ping-elevated border border-ping-border rounded-lg
                       text-ping-text font-mono text-sm
                       focus:outline-none focus:border-ping-accent/50"
            />
          </div>
        )}

        {/* Days of Week */}
        {reminder.type === 'weekly' && (
          <div>
            <div className="text-sm font-medium text-ping-text mb-3">Days</div>
            <div className="flex gap-2">
              {DAYS.map((day) => {
                const isSelected = reminder.days?.includes(day.value);
                return (
                  <button
                    key={day.value}
                    onClick={() => toggleDay(day.value)}
                    className={`
                      flex-1 py-2 rounded-lg text-sm font-medium transition-colors
                      ${isSelected
                        ? 'bg-ping-accent text-ping-bg'
                        : 'bg-ping-elevated text-ping-muted hover:text-ping-text hover:bg-ping-border'}
                    `}
                  >
                    {day.label}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Activity Only Toggle (for interval type) */}
        {reminder.type === 'interval' && (
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-ping-text">Only when active</div>
              <div className="text-xs text-ping-muted mt-0.5">
                Pause reminders when laptop is idle
              </div>
            </div>
            <button
              onClick={() => setReminder((prev) => ({ ...prev, activityOnly: !prev.activityOnly }))}
              className={`
                relative w-10 h-6 rounded-full transition-colors duration-200
                ${reminder.activityOnly ? 'bg-ping-accent' : 'bg-ping-border'}
              `}
            >
              <div
                className={`
                  absolute top-1 w-4 h-4 rounded-full bg-white
                  transition-transform duration-200
                  ${reminder.activityOnly ? 'left-5' : 'left-1'}
                `}
              />
            </button>
          </div>
        )}

        {/* Summary */}
        {reminder.type !== 'none' && (
          <div className="p-3 bg-ping-elevated rounded-lg border border-ping-border">
            <div className="text-xs text-ping-muted mb-1">Summary</div>
            <div className="text-sm text-ping-text">
              {reminder.type === 'interval' && (
                <>
                  Remind every {formatInterval(reminder.interval || 60)}
                  {reminder.activityOnly && ' while laptop is active'}
                </>
              )}
              {reminder.type === 'daily' && (
                <>Remind daily at {reminder.time || '9:00 AM'}</>
              )}
              {reminder.type === 'weekly' && (
                <>
                  Remind on {reminder.days?.map((d) => DAYS.find((day) => day.value === d)?.label).join(', ')} at {reminder.time || '9:00 AM'}
                </>
              )}
              {reminder.type === 'activity' && (
                <>
                  Remind every {formatInterval(reminder.interval || 60)} while actively using laptop
                </>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-ping-border/50 flex items-center justify-end gap-3 flex-shrink-0">
        <button
          onClick={onBack}
          className="px-4 py-2 text-sm text-ping-muted hover:text-ping-text transition-colors"
        >
          Cancel
        </button>
        <button
          onClick={handleSave}
          className="px-4 py-2 bg-ping-accent text-ping-bg rounded-lg text-sm font-medium
                   hover:bg-ping-accent/90 transition-colors"
        >
          Save Changes
        </button>
      </div>
    </div>
  );
}
