import { useState } from 'react';
import { Minus, Square, X, Terminal } from 'lucide-react';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  const handleMinimize = () => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.window.appWindow.minimize();
    }
  };

  const handleMaximize = () => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.window.appWindow.toggleMaximize();
      setIsMaximized(!isMaximized);
    }
  };

  const handleClose = () => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.window.appWindow.close();
    }
  };

  return (
    <div 
      className="h-10 flex items-center justify-between px-4 select-none"
      style={{ 
        backgroundColor: 'var(--bgSecondary)', 
        borderBottom: '1px solid var(--borderDefault)',
        WebkitAppRegion: 'drag' as any,
      }}
      data-tauri-drag-region
    >
      <div className="flex items-center gap-2" style={{ WebkitAppRegion: 'no-drag' as any }}>
        <Terminal size={16} style={{ color: 'var(--accentPrimary)' }} />
        <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>
          Bennett Studio
        </span>
        <span className="text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
          Desktop
        </span>
      </div>

      <div className="flex items-center gap-2" style={{ WebkitAppRegion: 'no-drag' as any }}>
        <button 
          onClick={handleMinimize}
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-all hover:bg-white/10"
          style={{ color: 'var(--textSecondary)' }}
        >
          <Minus size={14} />
        </button>
        <button 
          onClick={handleMaximize}
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-all hover:bg-white/10"
          style={{ color: 'var(--textSecondary)' }}
        >
          <Square size={14} />
        </button>
        <button 
          onClick={handleClose}
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-all hover:bg-red-500/20"
          style={{ color: 'var(--textSecondary)' }}
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}

