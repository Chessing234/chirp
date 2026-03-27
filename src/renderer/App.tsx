import { useEffect } from 'react';
import { useStore } from './lib/store';
import { CommandPalette } from './components/CommandPalette';
import { useEscapeToHide } from './hooks/useKeyboard';

export default function App() {
  const loadData = useStore((s) => s.loadData);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEscapeToHide();

  return (
    <div className="h-full w-full bg-[#111111]">
      <CommandPalette />
    </div>
  );
}
