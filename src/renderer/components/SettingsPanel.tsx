import { useState, useEffect } from 'react';
import { useStore } from '../lib/store';
import { getApi } from '../lib/tauri-api';

interface SettingsPanelProps {
  onBack: () => void;
}

export function SettingsPanel({ onBack }: SettingsPanelProps) {
  const { settings, setSettings } = useStore();
  const [localSettings, setLocalSettings] = useState(settings);

  useEffect(() => {
    setLocalSettings(settings);
  }, [settings]);

  const updateSetting = async (key: string, value: unknown) => {
    setLocalSettings((prev) => ({ ...prev, [key]: value }));
    const newSettings = await getApi().app.setSettings(key, value);
    setSettings(newSettings);
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' || e.key === 'Backspace') {
        e.preventDefault();
        onBack();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onBack]);

  return (
    <div className="animate-slide-up">
      {/* Header */}
      <div className="px-4 py-3 border-b border-ping-border/50 flex items-center gap-3">
        <button
          onClick={onBack}
          className="p-1 rounded hover:bg-ping-elevated transition-colors"
        >
          <svg
            className="w-5 h-5 text-ping-muted"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
        </button>
        <h2 className="text-sm font-semibold text-ping-text">Settings</h2>
      </div>

      {/* Settings content */}
      <div className="p-4 space-y-6">
        {/* Launch on Startup */}
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm font-medium text-ping-text">Launch on startup</div>
            <div className="text-xs text-ping-muted mt-0.5">
              Start Chirp when you log in
            </div>
          </div>
          <button
            onClick={() => updateSetting('launchOnStartup', !localSettings.launchOnStartup)}
            className={`
              relative w-10 h-6 rounded-full transition-colors duration-200
              ${localSettings.launchOnStartup ? 'bg-ping-accent' : 'bg-ping-border'}
            `}
          >
            <div
              className={`
                absolute top-1 w-4 h-4 rounded-full bg-white
                transition-transform duration-200
                ${localSettings.launchOnStartup ? 'left-5' : 'left-1'}
              `}
            />
          </button>
        </div>

        {/* Sound */}
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm font-medium text-ping-text">Notification sounds</div>
            <div className="text-xs text-ping-muted mt-0.5">
              Play sounds for reminders
            </div>
          </div>
          <button
            onClick={() => updateSetting('soundEnabled', !localSettings.soundEnabled)}
            className={`
              relative w-10 h-6 rounded-full transition-colors duration-200
              ${localSettings.soundEnabled ? 'bg-ping-accent' : 'bg-ping-border'}
            `}
          >
            <div
              className={`
                absolute top-1 w-4 h-4 rounded-full bg-white
                transition-transform duration-200
                ${localSettings.soundEnabled ? 'left-5' : 'left-1'}
              `}
            />
          </button>
        </div>

        {/* Global Shortcut */}
        <div>
          <div className="text-sm font-medium text-ping-text">Global shortcut</div>
          <div className="text-xs text-ping-muted mt-0.5 mb-2">
            Press to summon Chirp from anywhere
          </div>
          <div className="flex items-center gap-2">
            <div className="flex-1 px-3 py-2 bg-ping-elevated border border-ping-border rounded-lg">
              <code className="text-sm text-ping-text font-mono">
                {localSettings.globalShortcut.replace('CommandOrControl', '⌘/Ctrl')}
              </code>
            </div>
          </div>
        </div>

        {/* Keyboard shortcuts reference */}
        <div>
          <div className="text-sm font-medium text-ping-text mb-3">Keyboard shortcuts</div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-ping-muted">Open Chirp</span>
              <span className="kbd">⌘⇧P</span>
            </div>
            <div className="flex justify-between">
              <span className="text-ping-muted">Close</span>
              <span className="kbd">Esc</span>
            </div>
            <div className="flex justify-between">
              <span className="text-ping-muted">Navigate</span>
              <span className="kbd">↑↓</span>
            </div>
            <div className="flex justify-between">
              <span className="text-ping-muted">Toggle task</span>
              <span className="kbd">⌘↵</span>
            </div>
            <div className="flex justify-between">
              <span className="text-ping-muted">Switch list</span>
              <span className="kbd">Tab</span>
            </div>
            <div className="flex justify-between">
              <span className="text-ping-muted">Add task</span>
              <span className="kbd">↵</span>
            </div>
          </div>
        </div>

        {/* Version */}
        <div className="pt-4 border-t border-ping-border/50 text-center">
          <div className="text-xs text-ping-muted">
            Chirp v1.0.0
          </div>
          <div className="text-xs text-ping-muted/50 mt-1">
            Keyboard-first task companion
          </div>
        </div>
      </div>
    </div>
  );
}
