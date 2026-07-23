import { useThemeStore } from '../../stores/themeStore';
import { getAllThemes } from '../../theme';
import { Palette } from 'lucide-react';

export function AppearanceSettings() {
  const { theme, setTheme } = useThemeStore();
  const themes = getAllThemes();

  return (
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
  );
}
