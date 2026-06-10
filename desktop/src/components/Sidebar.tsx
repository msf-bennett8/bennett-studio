import { useLocation, useNavigate } from 'react-router-dom';
import { Database, Search, Table2, Share2, Settings, Home, Terminal, Cpu } from 'lucide-react';

const navItems = [
  { icon: Home, label: 'Home', path: '/' },
  { icon: Database, label: 'Databases', path: '/databases' },
  { icon: Search, label: 'Query', path: '/query' },
  { icon: Table2, label: 'Schema', path: '/schema' },
  { icon: Share2, label: 'Share', path: '/share' },
  { icon: Settings, label: 'Settings', path: '/settings' },
];

export function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <aside className="w-64 flex flex-col border-r" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
      <div className="p-6 flex items-center gap-3">
        <div className="w-10 h-10 rounded-xl flex items-center justify-center font-bold text-xl" style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}>
          <Terminal size={20} />
        </div>
        <div>
          <h1 className="font-bold text-lg" style={{ color: 'var(--textPrimary)' }}>Bennett</h1>
          <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Studio Desktop</p>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 space-y-1">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.path;
          return (
            <button key={item.path} onClick={() => navigate(item.path)} className="w-full flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium transition-all"
              style={{ backgroundColor: isActive ? 'var(--surfaceActive)' : 'transparent', color: isActive ? 'var(--accentPrimary)' : 'var(--textSecondary)', borderRight: isActive ? '3px solid var(--accentPrimary)' : '3px solid transparent' }}>
              <Icon size={18} />
              {item.label}
            </button>
          );
        })}
      </nav>

      <div className="p-4 border-t space-y-2" style={{ borderColor: 'var(--borderDefault)' }}>
        <div className="flex items-center gap-3 px-4 py-3 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)' }}>
          <div className="w-2 h-2 rounded-full" style={{ backgroundColor: 'var(--accentSuccess)' }} />
          <span className="text-xs" style={{ color: 'var(--textSecondary)' }}>Engine Online</span>
        </div>
        <div className="flex items-center gap-3 px-4 py-2 rounded-xl" style={{ backgroundColor: 'var(--bgTertiary)' }}>
          <Cpu size={14} style={{ color: 'var(--textMuted)' }} />
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>Rust v1.78</span>
        </div>
      </div>
    </aside>
  );
}

