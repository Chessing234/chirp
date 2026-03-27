import { useState, useRef, useEffect } from 'react';
import type { List } from '../../shared/types';

interface ListSelectorProps {
  lists: List[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onClose: () => void;
  onCreateList: (name: string) => Promise<void>;
  onDeleteList?: (id: string) => Promise<void>;
  onConfigureReminder?: (list: List) => void;
}

const COLORS = [
  '#4a9f6e', // green
  '#5a8dee', // blue
  '#e55050', // red
  '#f59e0b', // amber
  '#8b5cf6', // purple
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#84cc16', // lime
];

export function ListSelector({
  lists,
  selectedId,
  onSelect,
  onClose,
  onCreateList,
  onDeleteList,
  onConfigureReminder,
}: ListSelectorProps) {
  const [isCreating, setIsCreating] = useState(false);
  const [newListName, setNewListName] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(
    lists.findIndex((l) => l.id === selectedId)
  );
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isCreating) {
      inputRef.current?.focus();
    }
  }, [isCreating]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isCreating) {
        if (e.key === 'Escape') {
          setIsCreating(false);
          setNewListName('');
        } else if (e.key === 'Enter' && newListName.trim()) {
          handleCreate();
        }
        return;
      }

      switch (e.key) {
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex((i) => Math.max(0, i - 1));
          break;
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex((i) => Math.min(lists.length - 1, i + 1));
          break;
        case 'Enter':
          e.preventDefault();
          if (selectedIndex >= 0 && selectedIndex < lists.length) {
            onSelect(lists[selectedIndex].id);
          }
          break;
        case 'Escape':
        case 'Tab':
          e.preventDefault();
          onClose();
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isCreating, newListName, selectedIndex, lists, onSelect, onClose]);

  const handleCreate = async () => {
    if (newListName.trim()) {
      await onCreateList(newListName.trim());
      setNewListName('');
      setIsCreating(false);
    }
  };

  const handleDeleteClick = (e: React.MouseEvent, listId: string, listName: string) => {
    e.stopPropagation();
    e.preventDefault();
    console.log('Delete clicked for:', listId, listName);
    if (onDeleteList) {
      onDeleteList(listId)
        .then(() => console.log('Delete success'))
        .catch((err) => console.error('Delete error:', err));
    }
  };

  return (
    <div className="border-b border-ping-border/50 animate-slide-up">
      <div className="p-2 max-h-[200px] overflow-y-auto">
        {lists.map((list, idx) => (
          <div
            key={list.id}
            onMouseEnter={() => setSelectedIndex(idx)}
            className={`
              relative flex items-center px-3 py-2 rounded-lg
              transition-colors duration-100
              ${selectedIndex === idx ? 'bg-ping-accent/10' : 'hover:bg-ping-elevated'}
            `}
          >
            {/* List name - clickable to select */}
            <div
              onClick={() => onSelect(list.id)}
              className={`
                flex-1 flex items-center gap-3 cursor-pointer
                ${list.id === selectedId ? 'text-ping-accent' : 'text-ping-text'}
              `}
            >
              <div
                className="w-3 h-3 rounded-full flex-shrink-0"
                style={{ backgroundColor: list.color || '#4a9f6e' }}
              />
              <span className="text-sm font-medium truncate">{list.name}</span>
              {list.reminder?.enabled && (
                <svg className="w-3.5 h-3.5 text-amber-400 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6z" />
                </svg>
              )}
              {list.id === selectedId && (
                <svg className="w-4 h-4 text-ping-accent flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
              )}
            </div>

            {/* Action buttons - separate from the select area */}
            <div className="flex items-center gap-1 ml-2 flex-shrink-0">
              {onConfigureReminder && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onConfigureReminder(list);
                  }}
                  className="p-1.5 rounded hover:bg-ping-border/50 text-ping-muted hover:text-ping-text transition-colors"
                  title="Configure reminders"
                >
                  <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                </button>
              )}
              {onDeleteList && (
                <button
                  type="button"
                  onClick={(e) => handleDeleteClick(e, list.id, list.name)}
                  className="p-1.5 rounded bg-red-500/20 text-red-400 hover:bg-red-500/40 hover:text-red-300 transition-colors z-10"
                  title="Delete list"
                >
                  <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              )}
            </div>
          </div>
        ))}

        {/* Create new list */}
        {isCreating ? (
          <div className="flex items-center gap-2 px-3 py-2">
            <div
              className="w-3 h-3 rounded-full flex-shrink-0"
              style={{ backgroundColor: COLORS[lists.length % COLORS.length] }}
            />
            <input
              ref={inputRef}
              type="text"
              value={newListName}
              onChange={(e) => setNewListName(e.target.value)}
              placeholder="List name..."
              className="flex-1 bg-transparent text-sm text-ping-text placeholder-ping-muted focus:outline-none"
            />
            <button
              onClick={handleCreate}
              disabled={!newListName.trim()}
              className="text-xs px-2 py-1 rounded bg-ping-accent text-ping-bg disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Add
            </button>
          </div>
        ) : (
          <button
            onClick={() => setIsCreating(true)}
            className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left text-ping-muted hover:text-ping-text hover:bg-ping-elevated transition-colors duration-100"
          >
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            <span className="text-sm">New list</span>
          </button>
        )}
      </div>
    </div>
  );
}
