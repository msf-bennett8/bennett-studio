import { useThemeStore } from '../../stores/themeStore';
import { useUIPreferencesStore } from '../../stores/uiPreferencesStore';
import { getAllThemes } from '../../theme';
import { Palette, Type, Zap } from 'lucide-react';

export function AppearanceSettings() {
  const { theme, setTheme } = useThemeStore();
  const { fontScale, setFontScale, reduceMotion, setReduceMotion } = useUIPreferencesStore();
  const themes = getAllThemes();

  return (
    <div className="space-y-6">
      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-3 mb-4">
          <Palette size={20} style={{ color: 'var(--accentPrimary)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Theme</h2>
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
          <Type size={20} style={{ color: 'var(--accentInfo)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Text Size</h2>
        </div>
        <p className="text-xs mb-3" style={{ color: 'var(--textMuted)' }}>
          Scales text across the entire app. Current: {fontScale}%
        </p>
        <div className="flex items-center gap-3">
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>A</span>
          <input
            type="range"
            min={90}
            max={130}
            step={5}
            value={fontScale}
            onChange={(e) => setFontScale(Number(e.target.value))}
            className="flex-1"
          />
          <span className="text-lg" style={{ color: 'var(--textMuted)' }}>A</span>
        </div>
      </div>

      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-3 mb-4">
          <Zap size={20} style={{ color: 'var(--accentWarning)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Motion</h2>
        </div>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Reduce motion</p>
            <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Disables animations and transitions throughout the app.</p>
          </div>
          <button onClick={() => setReduceMotion(!reduceMotion)} className="w-12 h-6 rounded-full transition-all relative" style={{ backgroundColor: reduceMotion ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}>
            <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: reduceMotion ? '26px' : '2px' }} />
          </button>
        </div>
      </div>
    </div>
  );
}
