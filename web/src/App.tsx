import { useEffect } from 'react';
import { useThemeStore } from './stores/themeStore';
import { ThemeSwitcher } from './components/ui/ThemeSwitcher';

function App() {
  const { theme, colors } = useThemeStore();

  useEffect(() => {
    // Apply CSS variables when theme changes
    const root = document.documentElement;
    Object.entries(colors).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value);
    });
    root.setAttribute('data-theme', theme);
  }, [theme, colors]);

  return (
    <div 
      className="min-h-screen"
      style={{ backgroundColor: 'var(--bgPrimary)', color: 'var(--textPrimary)' }}
    >
      {/* Header */}
      <header 
        className="border-b px-6 py-4 flex items-center justify-between"
        style={{ 
          backgroundColor: 'var(--bgSecondary)',
          borderColor: 'var(--borderDefault)'
        }}
      >
        <div className="flex items-center gap-3">
          <div 
            className="w-8 h-8 rounded-lg flex items-center justify-center font-bold text-lg"
            style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}
          >
            B
          </div>
          <h1 className="text-xl font-bold">Bennett Studio</h1>
          <span 
            className="text-xs px-2 py-1 rounded-full"
            style={{ 
              backgroundColor: 'var(--surfaceActive)',
              color: 'var(--textTertiary)'
            }}
          >
            Web
          </span>
        </div>
        
        <div className="flex items-center gap-4">
          <ThemeSwitcher />
        </div>
      </header>

      {/* Main Content */}
      <main className="p-8">
        <div className="max-w-4xl mx-auto space-y-8">
          
          {/* Welcome Card */}
          <div 
            className="card"
            style={{ 
              backgroundColor: 'var(--surfaceDefault)',
              borderColor: 'var(--borderDefault)'
            }}
          >
            <h2 className="text-2xl font-bold mb-2">Welcome to Bennett Studio</h2>
            <p style={{ color: 'var(--textSecondary)' }}>
              Connect to shared databases or join a session via link.
            </p>
            
            <div className="mt-6 flex gap-4">
              <button className="btn-primary">
                Connect to Database
              </button>
              <button className="btn-secondary">
                Join Shared Session
              </button>
            </div>
          </div>

          {/* Theme Preview */}
          <div 
            className="card"
            style={{ 
              backgroundColor: 'var(--surfaceDefault)',
              borderColor: 'var(--borderDefault)'
            }}
          >
            <h3 className="text-lg font-semibold mb-4">Current Theme: {theme}</h3>
            
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="space-y-2">
                <div className="text-sm" style={{ color: 'var(--textTertiary)' }}>Backgrounds</div>
                <div className="flex gap-2">
                  <div className="w-8 h-8 rounded" style={{ backgroundColor: 'var(--bgPrimary)' }} />
                  <div className="w-8 h-8 rounded" style={{ backgroundColor: 'var(--bgSecondary)' }} />
                  <div className="w-8 h-8 rounded" style={{ backgroundColor: 'var(--bgTertiary)' }} />
                </div>
              </div>
              
              <div className="space-y-2">
                <div className="text-sm" style={{ color: 'var(--textTertiary)' }}>Accents</div>
                <div className="flex gap-2">
                  <div className="w-8 h-8 rounded" style={{ backgroundColor: 'var(--accentPrimary)' }} />
                  <div className="w-8 h-8 rounded" style={{ backgroundColor: 'var(--accentSecondary)' }} />
                  <div className="w-8 h-8 rounded" style={{ backgroundColor: 'var(--accentSuccess)' }} />
                </div>
              </div>
              
              <div className="space-y-2">
                <div className="text-sm" style={{ color: 'var(--textTertiary)' }}>Text</div>
                <div className="flex flex-col gap-1">
                  <span style={{ color: 'var(--textPrimary)' }}>Primary</span>
                  <span style={{ color: 'var(--textSecondary)' }}>Secondary</span>
                  <span style={{ color: 'var(--textMuted)' }}>Muted</span>
                </div>
              </div>
              
              <div className="space-y-2">
                <div className="text-sm" style={{ color: 'var(--textTertiary)' }}>Terminal</div>
                <div className="font-mono text-sm" style={{ color: 'var(--terminalGreen)' }}>
                  $ bennett --version
                </div>
                <div className="font-mono text-sm" style={{ color: 'var(--terminalGreenDim)' }}>
                  v0.1.0
                </div>
              </div>
            </div>
          </div>

          {/* Syntax Preview */}
          <div 
            className="card"
            style={{ 
              backgroundColor: 'var(--bgSecondary)',
              borderColor: 'var(--borderDefault)'
            }}
          >
            <h3 className="text-lg font-semibold mb-4">SQL Syntax Preview</h3>
            <pre className="sql-editor p-4 rounded-lg overflow-x-auto">
              <code>
                <span className="syntax-keyword">SELECT</span>{' '}
                <span className="syntax-variable">id</span>,{' '}
                <span className="syntax-variable">name</span>,{' '}
                <span className="syntax-variable">email</span>{' '}\n
                <span className="syntax-keyword">FROM</span>{' '}
                <span className="syntax-type">users</span>{' '}\n
                <span className="syntax-keyword">WHERE</span>{' '}
                <span className="syntax-variable">created_at</span>{' '}
                <span className="syntax-operator">&gt;</span>{' '}
                <span className="syntax-string">'2024-01-01'</span>{' '}\n
                <span className="syntax-keyword">AND</span>{' '}
                <span className="syntax-variable">status</span>{' '}
                <span className="syntax-operator">=</span>{' '}
                <span className="syntax-string">'active'</span>;{' '}\n
                <span className="syntax-comment">-- Filter active users</span>
              </code>
            </pre>
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
