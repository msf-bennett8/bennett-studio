import { useThemeStore } from '../../stores/themeStore';
import { getAllThemes } from '../../theme';

export function ThemeSwitcher() {
  const { theme, setTheme } = useThemeStore();
  const themes = getAllThemes();

  return (
    <div className="relative group">
      <button
        className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium"
        style={{ 
          backgroundColor: 'var(--surfaceDefault)',
          color: 'var(--textPrimary)',
          border: '1px solid var(--borderDefault)'
        }}
      >
        <span 
          className="w-3 h-3 rounded-full"
          style={{ backgroundColor: 'var(--accentPrimary)' }}
        />
        {themes.find(t => t.id === theme)?.name || 'Theme'}
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      <div 
        className="absolute right-0 mt-2 w-56 rounded-xl shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50"
        style={{ 
          backgroundColor: 'var(--bgElevated)',
          border: '1px solid var(--borderDefault)'
        }}
      >
        <div className="p-2">
          {themes.map((t) => (
            <button
              key={t.id}
              onClick={() => setTheme(t.id)}
              className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-all"
              style={{
                backgroundColor: theme === t.id ? 'var(--surfaceActive)' : 'transparent',
                color: 'var(--textPrimary)',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = 'var(--surfaceHover)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = theme === t.id ? 'var(--surfaceActive)' : 'transparent';
              }}
            >
              <span 
                className="w-3 h-3 rounded-full flex-shrink-0"
                style={{ 
                  backgroundColor: t.id === 'terminal' ? '#00ff88' : 
                    t.id === 'light' ? '#1a1a1a' :
                    t.id === 'ocean' ? '#00d4ff' :
                    t.id === 'midnight' ? '#58a6ff' : '#00d4aa'
                }}
              />
              <div>
                <div className="text-sm font-medium">{t.name}</div>
                <div className="text-xs" style={{ color: 'var(--textTertiary)' }}>
                  {t.description}
                </div>
              </div>
              {theme === t.id && (
                <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
              )}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
