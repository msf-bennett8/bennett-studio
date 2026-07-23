import { useLocation, useNavigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { Database, Search, Table2, Share2, Settings, Home, Terminal, Cpu, Rows3, Globe, StickyNote, Palette, Bell, Shield, KeyRound, ArrowLeft, Sliders, Users, Activity, BookOpen } from 'lucide-react';
import { api } from '../../services/api';
import { useUIPreferencesStore } from '../../stores/uiPreferencesStore';

const navItems = [
  { icon: Home, label: 'Home', path: '/' },
  { icon: Database, label: 'Databases', path: '/databases' },
  { icon: Search, label: 'Query', path: '/query' },
  { icon: Table2, label: 'Schema', path: '/schema' },
  { icon: Rows3, label: 'Data', path: '/data' },
  { icon: Share2, label: 'Share', path: '/share' },
  { icon: Globe, label: 'Remote', path: '/remote-query' },
  { icon: StickyNote, label: 'Notes', path: '/notes' },
  { icon: Settings, label: 'Settings', path: '/settings' },
];

const settingsNavItems = [
  { icon: Sliders, label: 'General', path: '/settings/general' },
  { icon: Palette, label: 'Appearance', path: '/settings/appearance' },
  { icon: Bell, label: 'Notifications', path: '/settings/notifications' },
  { icon: Shield, label: 'Privacy & Security', path: '/settings/privacy' },
  { icon: KeyRound, label: 'API Keys', path: '/settings/api-keys' },
  { icon: Users, label: 'Guests', path: '/settings/guests' },
  { icon: Activity, label: 'Activity', path: '/settings/activity' },
  { icon: BookOpen, label: 'Resources', path: '/settings/resources' },
];

export function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();
  const [engineOnline, setEngineOnline] = useState(true);
  const isSettingsSection = location.pathname.startsWith('/settings');
  const { compactSidebar } = useUIPreferencesStore();
  const navButtonPadding = compactSidebar ? 'px-4 py-2' : 'px-4 py-3';

  // Real-time engine health polling
  useEffect(() => {
    let mounted = true;

    const checkHealth = async () => {
      try {
        await api.health();
        if (mounted) setEngineOnline(true);
      } catch {
        if (mounted) setEngineOnline(false);
      }
    };

    checkHealth();
    const interval = setInterval(checkHealth, 5000); // Check every 5s

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

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
        {isSettingsSection ? (
          <>
            <button onClick={() => navigate('/')} className="w-full flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium transition-all mb-2"
              style={{ color: 'var(--textSecondary)' }}>
              <ArrowLeft size={18} />
              Back to Studio
            </button>
            <div className="h-px my-2" style={{ backgroundColor: 'var(--borderDefault)' }} />
            {settingsNavItems.map((item) => {
              const Icon = item.icon;
              const isActive = location.pathname === item.path;
              return (
                <button key={item.path} onClick={() => navigate(item.path)} className={`w-full flex items-center gap-3 ${navButtonPadding} rounded-xl text-sm font-medium transition-all`}
                  style={{ backgroundColor: isActive ? 'var(--surfaceActive)' : 'transparent', color: isActive ? 'var(--accentPrimary)' : 'var(--textSecondary)', borderRight: isActive ? '3px solid var(--accentPrimary)' : '3px solid transparent' }}>
                  <Icon size={18} />
                  {item.label}
                </button>
              );
            })}
          </>
        ) : (
          navItems.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname === item.path;
            return (
              <button key={item.path} onClick={() => navigate(item.path)} className={`w-full flex items-center gap-3 ${navButtonPadding} rounded-xl text-sm font-medium transition-all`}
                style={{ backgroundColor: isActive ? 'var(--surfaceActive)' : 'transparent', color: isActive ? 'var(--accentPrimary)' : 'var(--textSecondary)', borderRight: isActive ? '3px solid var(--accentPrimary)' : '3px solid transparent' }}>
                <Icon size={18} />
                {item.label}
              </button>
            );
          })
        )}
      </nav>

      <div className="p-4 border-t space-y-2" style={{ borderColor: 'var(--borderDefault)' }}>
        <div className="flex items-center gap-3 px-4 py-3 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)' }}>
          <div
            className="w-2 h-2 rounded-full"
            style={{
              backgroundColor: engineOnline ? 'var(--accentSuccess)' : 'var(--accentError)',
              animation: 'blink-dot 3s steps(1) infinite',
            }}
          />
          <span className="text-xs" style={{ color: 'var(--textSecondary)' }}>
            {engineOnline ? 'Engine Online' : 'Engine Offline'}
          </span>
        </div>
        <div className="flex items-center gap-3 px-4 py-2 rounded-xl" style={{ backgroundColor: 'var(--bgTertiary)' }}>
          <Cpu size={14} style={{ color: 'var(--textMuted)' }} />
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>Rust v1.78</span>
        </div>
      </div>
    </aside>
  );
}
