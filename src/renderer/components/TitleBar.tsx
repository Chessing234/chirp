import { useState, useEffect } from 'react';
import { getApi } from '../lib/tauri-api';

const api = getApi();

export function TitleBar() {
  const [mode, setMode] = useState<'overlay' | 'full'>('full');

  useEffect(() => {
    // Get initial mode
    api.app.getMode().then(setMode).catch(() => {
      // Default to full mode if unable to get mode
    });
  }, []);

  const handleClose = () => {
    api.app.close();
  };

  const handleMinimize = () => {
    api.app.minimize();
  };

  // In overlay mode, show minimal controls
  if (mode === 'overlay') {
    return (
      <div className="flex items-center justify-between px-4 py-2 drag-region" data-tauri-drag-region>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-ping-accent" />
          <span className="text-xs font-medium text-ping-muted">Chirp</span>
        </div>
        <div className="flex items-center gap-1 no-drag">
          <span className="text-xs text-ping-muted/60 mr-2">Press Esc to close</span>
        </div>
      </div>
    );
  }

  // Full mode - show window controls
  return (
    <div className="flex items-center justify-between px-4 py-3 border-b border-ping-border/30 drag-region" data-tauri-drag-region>
      <div className="flex items-center gap-3 no-drag">
        {/* macOS-style traffic lights */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleClose}
            className="w-3 h-3 rounded-full bg-[#ff5f57] hover:bg-[#ff5f57]/80 transition-colors group relative"
            title="Close"
          >
            <svg
              className="w-3 h-3 absolute inset-0 opacity-0 group-hover:opacity-100 text-[#4c0002]"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M3 3l6 6M9 3l-6 6" />
            </svg>
          </button>
          <button
            onClick={handleMinimize}
            className="w-3 h-3 rounded-full bg-[#febc2e] hover:bg-[#febc2e]/80 transition-colors group relative"
            title="Minimize"
          >
            <svg
              className="w-3 h-3 absolute inset-0 opacity-0 group-hover:opacity-100 text-[#995700]"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M2 6h8" />
            </svg>
          </button>
          <div className="w-3 h-3 rounded-full bg-[#28c840] opacity-50" title="Maximize (disabled)" />
        </div>
      </div>

      <div className="flex items-center gap-2">
        <div className="w-2 h-2 rounded-full bg-ping-accent animate-pulse" />
        <span className="text-sm font-semibold text-ping-text">Chirp</span>
      </div>

      <div className="flex items-center gap-2 text-xs text-ping-muted no-drag">
        <span className="kbd">⌘E</span>
        <span>quick access</span>
      </div>
    </div>
  );
}
