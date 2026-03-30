import { useEffect, useCallback } from 'react';
import { useStore } from '../lib/store';
import { getApi } from '../lib/tauri-api';

interface KeyboardOptions {
  onEscape?: () => void;
  onEnter?: () => void;
  onArrowUp?: () => void;
  onArrowDown?: () => void;
  onTab?: () => void;
  onBackspace?: () => void;
  onCmdEnter?: () => void;
  enabled?: boolean;
}

export function useKeyboard(options: KeyboardOptions = {}) {
  const {
    onEscape,
    onEnter,
    onArrowUp,
    onArrowDown,
    onTab,
    onBackspace,
    onCmdEnter,
    enabled = true,
  } = options;

  const isInputFocused = useStore((s) => s.isInputFocused);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!enabled) return;

      // Handle Escape globally
      if (e.key === 'Escape') {
        e.preventDefault();
        onEscape?.();
        return;
      }

      // Handle Cmd/Ctrl + Enter
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        onCmdEnter?.();
        return;
      }

      // Skip navigation keys when typing in input
      if (isInputFocused) {
        // Still allow Tab to switch context
        if (e.key === 'Tab') {
          e.preventDefault();
          onTab?.();
        }
        return;
      }

      switch (e.key) {
        case 'Enter':
          e.preventDefault();
          onEnter?.();
          break;
        case 'ArrowUp':
          e.preventDefault();
          onArrowUp?.();
          break;
        case 'ArrowDown':
          e.preventDefault();
          onArrowDown?.();
          break;
        case 'Tab':
          e.preventDefault();
          onTab?.();
          break;
        case 'Backspace':
          e.preventDefault();
          onBackspace?.();
          break;
      }
    },
    [
      enabled,
      isInputFocused,
      onEscape,
      onEnter,
      onArrowUp,
      onArrowDown,
      onTab,
      onBackspace,
      onCmdEnter,
    ]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

export function useEscapeToHide() {
  const handleEscape = useCallback(() => {
    getApi().app.hide();
  }, []);

  useKeyboard({
    onEscape: handleEscape,
  });
}
