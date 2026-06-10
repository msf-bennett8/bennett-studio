import { useState } from 'react';
import { useThemeStore } from '../stores/themeStore';
import { getAllThemes } from '../theme';
import { Palette, Bell, Shield, Database, Terminal } from 'lucide-react';

export function SettingsPage() {
  const { theme, setTheme } = useThemeStore();
  const [notifications, setNotifications] = useState(true);
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [telemetry, setTelemetry] = useState(false);
  const themes = getAllThemes();

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <h1 className="text-3xl font-bold mb-8" style={{ color: 'var(--textPrimary)' }}>Settings</h1>

      <div className="space-y-6">
        <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center gap-3 mb-4">
            <Palette size={20} style={{ color: 'var(--accentPrimary)' }} />
            <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Appearance</h2>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
            {themes.map((t) => (
              <button key={t.id} onClick={() => setTheme(t.id)} className="p-4 rounded-xl text-left transition-all"
                style={{ backgroundColor: theme === t.id ? 'var(--surfaceActive)' : 'var(--bgTertiary)', border: theme === t.id ? '2px solid var(--accentPrimary)' : '2px solid transparent' }}>
                <div className="w-full h-8 rounded-lg mb-2" style={{ backgroundColor: t.id === 'terminal' ? '#000' : t.id === 'light' ? '#f5f5f5' : t.id === 'ocean' ? '#0a1628' : t.id === 'midnight' ? '#0d1117' : '#0a0a0a', border: '1px solid var(--borderDefault)' }} />
                <div className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{t.name}</div>
                <div className="text-xs" style={{ color: 'var(--textMuted)' }}>{t.description}</div>
              </button>
            ))}
          </div>
        </div>

        <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center gap-3 mb-4">
            <Bell size={20} style={{ color: 'var(--accentWarning)' }} />
            <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Notifications</h2>
          </div>
          <div className="space-y-4">
            {[
              { label: 'Enable Notifications', desc: 'Get alerts for query completions and share requests', state: notifications, setState: setNotifications },
              { label: 'Auto-update Check', desc: 'Automatically check for new versions', state: autoUpdate, setState: setAutoUpdate },
            ].map((item, i) => (
              <div key={i} className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{item.label}</p>
                  <p className="text-xs" style={{ color: 'var(--textMuted)' }}>{item.desc}</p>
                </div>
                <button onClick={() => item.setState(!item.state)} className="w-12 h-6 rounded-full transition-all relative" style={{ backgroundColor: item.state ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}>
                  <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: item.state ? '26px' : '2px' }} />
                </button>
              </div>
            ))}
          </div>
        </div>

        <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center gap-3 mb-4">
            <Shield size={20} style={{ color: 'var(--accentError)' }} />
            <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Privacy & Security</h2>
          </div>
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Telemetry</p>
                <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Send anonymous usage data to improve the product</p>
              </div>
              <button onClick={() => setTelemetry(!telemetry)} className="w-12 h-6 rounded-full transition-all relative" style={{ backgroundColor: telemetry ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}>
                <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: telemetry ? '26px' : '2px' }} />
              </button>
            </div>
            <div className="p-4 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              <div className="flex items-center gap-2 mb-2">
                <Database size={16} style={{ color: 'var(--accentPrimary)' }} />
                <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Data Residency</span>
              </div>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>All database data stays on your local machine. No data is sent to our servers unless you explicitly create a share tunnel.</p>
            </div>
          </div>
        </div>

        <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center gap-3 mb-4">
            <Terminal size={20} style={{ color: 'var(--accentInfo)' }} />
            <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Engine Info</h2>
          </div>
          <div className="grid grid-cols-2 gap-4">
            {[
              { label: 'Version', value: 'v0.1.0-alpha' },
              { label: 'Runtime', value: 'Rust + Tokio' },
              { label: 'Protocol', value: 'gRPC + WebSocket' },
              { label: 'License', value: 'MIT + Commercial' },
            ].map((item, i) => (
              <div key={i} className="p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                <p className="text-xs" style={{ color: 'var(--textMuted)' }}>{item.label}</p>
                <p className="text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{item.value}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

