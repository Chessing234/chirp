import { useState, useRef, useEffect, useCallback } from 'react';
import { useStore } from '../lib/store';
import { getApi } from '../lib/tauri-api';
import { parseTaskInput, formatDueDate, formatPingInterval } from '../lib/parser';
import { useFuzzySearch } from '../hooks/useFuzzySearch';
import { useKeyboard } from '../hooks/useKeyboard';
import { TaskItem } from './TaskItem';
import { ListSelector } from './ListSelector';
import { SettingsPanel } from './SettingsPanel';
import { ListReminderSettings } from './ListReminderSettings';
import { TitleBar } from './TitleBar';
import type { List } from '../../shared/types';

export function CommandPalette() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [inputValue, setInputValue] = useState('');
  const [showListSelector, setShowListSelector] = useState(false);
  const [editingReminderList, setEditingReminderList] = useState<List | null>(null);

  const {
    lists,
    tasks,
    selectedListId,
    selectedTaskIndex,
    view,
    setSelectedListId,
    setSelectedTaskIndex,
    setView,
    setInputFocused,
    createTask,
    toggleTaskComplete,
    deleteTask,
    deleteList,
  } = useStore();

  // Get tasks for current list
  const currentListTasks = tasks.filter((t) => t.list_id === selectedListId && !t.parent_id);
  const pendingTasks = currentListTasks.filter((t) => !t.completed);
  const completedTasks = currentListTasks.filter((t) => t.completed);

  // Apply fuzzy search
  const searchQuery = inputValue.startsWith('/') ? '' : inputValue;
  const filteredPending = useFuzzySearch(pendingTasks, searchQuery);
  const filteredCompleted = useFuzzySearch(completedTasks, searchQuery);
  const allFiltered = [...filteredPending, ...filteredCompleted];

  const currentList = lists.find((l) => l.id === selectedListId);

  // Parse current input for preview
  const parsedInput = parseTaskInput(inputValue);
  const hasTimeInfo = parsedInput.due_at || parsedInput.ping_interval;

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Handle commands
  const handleCommand = useCallback(
    (cmd: string) => {
      const parts = cmd.slice(1).split(' ');
      const command = parts[0].toLowerCase();

      switch (command) {
        case 'list':
        case 'lists':
          setShowListSelector(true);
          setInputValue('');
          break;
        case 'settings':
        case 'config':
          setView('settings');
          setInputValue('');
          break;
        case 'new':
          // Create new list
          const listName = parts.slice(1).join(' ');
          if (listName) {
            useStore.getState().createList(listName);
            setInputValue('');
          }
          break;
        case 'clear':
          // Clear completed tasks
          completedTasks.forEach((t) => deleteTask(t.id));
          setInputValue('');
          break;
        case 'quit':
        case 'exit':
          getApi().app.quit();
          break;
      }
    },
    [setView, completedTasks, deleteTask]
  );

  // Submit handler
  const handleSubmit = useCallback(async () => {
    const trimmed = inputValue.trim();
    if (!trimmed) return;

    // Handle commands
    if (trimmed.startsWith('/')) {
      handleCommand(trimmed);
      return;
    }

    // Create task
    if (selectedListId && parsedInput.content) {
      await createTask({
        list_id: selectedListId,
        content: parsedInput.content,
        due_at: parsedInput.due_at?.getTime(),
        ping_interval: parsedInput.ping_interval,
      });
      setInputValue('');
    }
  }, [inputValue, selectedListId, parsedInput, createTask, handleCommand]);

  // Keyboard navigation
  useKeyboard({
    onEnter: () => {
      if (!inputValue && allFiltered.length > 0 && selectedTaskIndex < allFiltered.length) {
        // Toggle selected task
        const task = allFiltered[selectedTaskIndex];
        toggleTaskComplete(task.id);
      } else {
        handleSubmit();
      }
    },
    onArrowUp: () => {
      setSelectedTaskIndex(Math.max(0, selectedTaskIndex - 1));
    },
    onArrowDown: () => {
      setSelectedTaskIndex(Math.min(allFiltered.length - 1, selectedTaskIndex + 1));
    },
    onTab: () => {
      setShowListSelector(true);
    },
    onCmdEnter: () => {
      if (allFiltered.length > 0 && selectedTaskIndex < allFiltered.length) {
        toggleTaskComplete(allFiltered[selectedTaskIndex].id);
      }
    },
  });

  // Settings view
  if (view === 'settings') {
    return (
      <div className="w-full h-full glass overflow-hidden animate-in">
        <TitleBar />
        <SettingsPanel onBack={() => setView('command')} />
      </div>
    );
  }

  // List reminder settings view
  if (editingReminderList) {
    return (
      <div className="w-full h-full glass overflow-hidden animate-in flex flex-col">
        <TitleBar />
        <div className="flex-1 overflow-hidden">
          <ListReminderSettings
            list={editingReminderList}
            onBack={() => setEditingReminderList(null)}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="w-full h-full glass overflow-hidden animate-in">
      {/* Title bar with window controls */}
      <TitleBar />

      {/* Header with list selector */}
      <div className="px-4 pt-3 pb-2 border-b border-ping-border/50 drag-region">
        <div className="flex items-center justify-between no-drag">
          <button
            onClick={() => setShowListSelector(!showListSelector)}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg hover:bg-ping-elevated transition-colors"
          >
            <div
              className="w-2 h-2 rounded-full"
              style={{ backgroundColor: currentList?.color || '#4a9f6e' }}
            />
            <span className="text-sm font-medium text-ping-text">
              {currentList?.name || 'Select List'}
            </span>
            <svg
              className="w-4 h-4 text-ping-muted"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </button>

          <div className="flex items-center gap-2 text-xs text-ping-muted">
            <span className="kbd">Tab</span>
            <span>lists</span>
            <span className="ml-2 kbd">Esc</span>
            <span>close</span>
          </div>
        </div>
      </div>

      {/* List selector dropdown */}
      {showListSelector && (
        <ListSelector
          lists={lists}
          selectedId={selectedListId}
          onSelect={(id) => {
            setSelectedListId(id);
            setShowListSelector(false);
            inputRef.current?.focus();
          }}
          onClose={() => {
            setShowListSelector(false);
            inputRef.current?.focus();
          }}
          onCreateList={async (name) => {
            const list = await useStore.getState().createList(name, '#4a9f6e');
            setSelectedListId(list.id);
            setShowListSelector(false);
            // Auto-open reminder settings for newly created list
            setEditingReminderList(list);
          }}
          onDeleteList={async (id) => {
            await deleteList(id);
            setShowListSelector(false);
            inputRef.current?.focus();
          }}
          onConfigureReminder={(list) => {
            setShowListSelector(false);
            setEditingReminderList(list);
          }}
        />
      )}

      {/* Input */}
      <div className="px-4 py-3">
        <div className="relative">
          <input
            ref={inputRef}
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onFocus={() => setInputFocused(true)}
            onBlur={() => setInputFocused(false)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
                e.preventDefault();
                handleSubmit();
              }
            }}
            placeholder="Add task... (try: finish essay tomorrow 5pm ping 2h)"
            className="w-full px-4 py-3 bg-ping-elevated border border-ping-border rounded-xl
                     text-ping-text placeholder-ping-muted font-mono text-sm
                     focus:outline-none focus:border-ping-accent/50 focus:ring-1 focus:ring-ping-accent/20
                     transition-all duration-150"
          />

          {/* Input hints */}
          {hasTimeInfo && inputValue && (
            <div className="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-2">
              {parsedInput.due_at && (
                <span className="text-xs px-2 py-1 bg-ping-accent/10 text-ping-accent rounded">
                  {formatDueDate(parsedInput.due_at.getTime())}
                </span>
              )}
              {parsedInput.ping_interval && (
                <span className="text-xs px-2 py-1 bg-amber-500/10 text-amber-400 rounded flex items-center gap-1">
                  <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6z" />
                  </svg>
                  {formatPingInterval(parsedInput.ping_interval)}
                </span>
              )}
            </div>
          )}
        </div>

        {/* Commands hint */}
        {inputValue.startsWith('/') && (
          <div className="mt-2 px-1 text-xs text-ping-muted">
            <span className="text-ping-accent">/list</span> switch list •{' '}
            <span className="text-ping-accent">/new</span> create list •{' '}
            <span className="text-ping-accent">/settings</span> •{' '}
            <span className="text-ping-accent">/clear</span> completed
          </div>
        )}
      </div>

      {/* Task list */}
      <div className="max-h-[320px] overflow-y-auto">
        {/* Pending tasks */}
        {filteredPending.length > 0 && (
          <div className="px-2 pb-2">
            {filteredPending.map((task, idx) => (
              <TaskItem
                key={task.id}
                task={task}
                isSelected={selectedTaskIndex === idx}
                matches={task.matches}
                onToggle={() => toggleTaskComplete(task.id)}
                onDelete={() => deleteTask(task.id)}
                onSelect={() => setSelectedTaskIndex(idx)}
              />
            ))}
          </div>
        )}

        {/* Completed tasks */}
        {filteredCompleted.length > 0 && (
          <div className="px-2 pb-2">
            <div className="px-3 py-2 text-xs text-ping-muted font-medium">
              Completed ({filteredCompleted.length})
            </div>
            {filteredCompleted.map((task, idx) => (
              <TaskItem
                key={task.id}
                task={task}
                isSelected={selectedTaskIndex === filteredPending.length + idx}
                matches={task.matches}
                onToggle={() => toggleTaskComplete(task.id)}
                onDelete={() => deleteTask(task.id)}
                onSelect={() => setSelectedTaskIndex(filteredPending.length + idx)}
              />
            ))}
          </div>
        )}

        {/* Empty state */}
        {allFiltered.length === 0 && !inputValue && (
          <div className="px-4 py-8 text-center">
            <div className="text-ping-muted text-sm">
              No tasks yet. Start typing to add one!
            </div>
            <div className="mt-2 text-xs text-ping-muted/70">
              Try: "Call mom tomorrow 5pm ping 2h"
            </div>
          </div>
        )}

        {/* No results */}
        {allFiltered.length === 0 && inputValue && !inputValue.startsWith('/') && (
          <div className="px-4 py-6 text-center text-ping-muted text-sm">
            Press <span className="kbd">Enter</span> to create task
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-2 border-t border-ping-border/50 flex items-center justify-between text-xs text-ping-muted">
        <div className="flex items-center gap-4">
          <span>
            {pendingTasks.length} pending
            {completedTasks.length > 0 && ` • ${completedTasks.length} done`}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="kbd">↑↓</span>
          <span>navigate</span>
          <span className="kbd">⌘↵</span>
          <span>toggle</span>
        </div>
      </div>
    </div>
  );
}
