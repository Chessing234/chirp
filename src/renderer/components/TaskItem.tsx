import { memo } from 'react';
import type { Task } from '../../shared/types';
import { formatDueDate, formatPingInterval, getRelativeTime } from '../lib/parser';

interface TaskItemProps {
  task: Task;
  isSelected: boolean;
  matches: number[];
  onToggle: () => void;
  onDelete: () => void;
  onSelect: () => void;
}

export const TaskItem = memo(function TaskItem({
  task,
  isSelected,
  matches,
  onToggle,
  onDelete,
  onSelect,
}: TaskItemProps) {
  const isOverdue = task.due_at && !task.completed && task.due_at < Date.now();

  // Render highlighted text
  const renderContent = () => {
    if (matches.length === 0) {
      return <span>{task.content}</span>;
    }

    const result: React.ReactNode[] = [];
    let lastIndex = 0;

    for (const idx of matches) {
      if (idx > lastIndex) {
        result.push(
          <span key={`t-${lastIndex}`}>{task.content.slice(lastIndex, idx)}</span>
        );
      }
      result.push(
        <span key={`m-${idx}`} className="bg-ping-accent/30 text-ping-text">
          {task.content[idx]}
        </span>
      );
      lastIndex = idx + 1;
    }

    if (lastIndex < task.content.length) {
      result.push(
        <span key={`t-${lastIndex}`}>{task.content.slice(lastIndex)}</span>
      );
    }

    return result;
  };

  return (
    <div
      onClick={onSelect}
      onDoubleClick={onToggle}
      className={`
        group flex items-start gap-3 px-3 py-2.5 mx-1 rounded-lg cursor-pointer
        transition-all duration-150
        ${isSelected ? 'bg-ping-accent/10' : 'hover:bg-ping-elevated'}
        ${task.completed ? 'opacity-50' : ''}
      `}
    >
      {/* Checkbox */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onToggle();
        }}
        className={`
          mt-0.5 w-4 h-4 rounded border-2 flex items-center justify-center
          transition-all duration-150 flex-shrink-0
          ${
            task.completed
              ? 'bg-ping-accent border-ping-accent'
              : 'border-ping-border hover:border-ping-accent/50'
          }
        `}
      >
        {task.completed && (
          <svg className="w-3 h-3 text-ping-bg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
          </svg>
        )}
      </button>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div
          className={`
            font-mono text-sm leading-relaxed
            ${task.completed ? 'line-through text-ping-muted' : 'text-ping-text'}
          `}
        >
          {renderContent()}
        </div>

        {/* Meta info */}
        {(task.due_at || task.ping_interval) && !task.completed && (
          <div className="flex items-center gap-2 mt-1">
            {task.due_at && (
              <span
                className={`
                  text-xs px-1.5 py-0.5 rounded flex items-center gap-1
                  ${isOverdue ? 'bg-ping-danger/10 text-ping-danger' : 'bg-ping-elevated text-ping-muted'}
                `}
              >
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
                {formatDueDate(task.due_at)}
              </span>
            )}
            {task.ping_interval && (
              <span className="text-xs px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-400 flex items-center gap-1">
                <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6z" />
                </svg>
                every {formatPingInterval(task.ping_interval)}
              </span>
            )}
          </div>
        )}

        {/* Created time for completed tasks */}
        {task.completed && (
          <div className="text-xs text-ping-muted/50 mt-0.5">
            {getRelativeTime(task.updated_at)}
          </div>
        )}
      </div>

      {/* Delete button */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
        className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-ping-danger/10 text-ping-muted hover:text-ping-danger transition-all"
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
          />
        </svg>
      </button>
    </div>
  );
});
