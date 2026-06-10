import { useNavigate } from 'react-router-dom';
import { Database, Share2, Terminal, Zap, Shield, Globe, HardDrive, Wifi } from 'lucide-react';

export function HomePage() {
  const navigate = useNavigate();

  const features = [
    { icon: Database, title: 'Local Databases', description: 'Install PostgreSQL, MySQL, MariaDB, SQLite with one click. Docker-powered, zero config.', color: 'var(--accentPrimary)' },
    { icon: Share2, title: 'Secure Sharing', description: 'Share database access via secure tunnels. No firewall holes, UUID-based URLs.', color: 'var(--accentSecondary)' },
    { icon: Terminal, title: 'SQL Editor', description: 'Write queries with Monaco Editor, syntax highlighting, autocomplete, and real-time results.', color: 'var(--accentWarning)' },
    { icon: Zap, title: 'Native Performance', description: 'Built with Rust + Tauri. Direct system integration, native notifications, global hotkeys.', color: 'var(--accentSuccess)' },
    { icon: Shield, title: 'Enterprise Security', description: 'Schema-aware permissions, credential vaulting, audit logging, end-to-end encryption.', color: 'var(--accentError)' },
    { icon: Globe, title: 'Multi-Client Sync', description: 'Desktop, web, CLI, VS Code — all share the same headless engine via gRPC/WebSocket.', color: 'var(--accentInfo)' },
  ];

  const nativeFeatures = [
    { icon: HardDrive, label: 'Docker Runtime', value: 'Active', status: 'running' },
    { icon: Wifi, label: 'Relay Connection', value: 'Connected', status: 'active' },
    { icon: Terminal, label: 'Engine Process', value: 'PID 4521', status: 'running' },
  ];

  return (
    <div className="p-8 max-w-6xl mx-auto">
      {/* Native Status Bar */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        {nativeFeatures.map((feature, index) => (
          <div key={index} className="card p-4 rounded-xl flex items-center gap-3" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
            <feature.icon size={20} style={{ color: feature.status === 'running' ? 'var(--accentSuccess)' : 'var(--accentPrimary)' }} />
            <div>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>{feature.label}</p>
              <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{feature.value}</p>
            </div>
          </div>
        ))}
      </div>

      {/* Hero */}
      <div className="text-center mb-16">
        <h1 className="text-5xl font-bold mb-4" style={{ color: 'var(--textPrimary)' }}>Bennett Studio</h1>
        <p className="text-xl mb-2" style={{ color: 'var(--textSecondary)' }}>The Database Workspace for Modern Developers</p>
        <p className="text-sm mb-8" style={{ color: 'var(--textMuted)' }}>林深时见鹿，海深时见鲸，情深时见你</p>
        <div className="flex justify-center gap-4">
          <button onClick={() => navigate('/databases')} className="btn-primary px-6 py-3 rounded-xl font-medium">
            <Database size={18} className="inline mr-2" />Add Database
          </button>
          <button onClick={() => navigate('/share')} className="btn-secondary px-6 py-3 rounded-xl font-medium">
            <Share2 size={18} className="inline mr-2" />Share Access
          </button>
        </div>
      </div>

      {/* Features Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {features.map((feature, index) => {
          const Icon = feature.icon;
          return (
            <div key={index} className="card p-6 rounded-xl transition-all hover:scale-[1.02]" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
              <div className="w-12 h-12 rounded-xl flex items-center justify-center mb-4" style={{ backgroundColor: `${feature.color}20` }}>
                <Icon size={24} style={{ color: feature.color }} />
              </div>
              <h3 className="text-lg font-semibold mb-2" style={{ color: 'var(--textPrimary)' }}>{feature.title}</h3>
              <p className="text-sm leading-relaxed" style={{ color: 'var(--textSecondary)' }}>{feature.description}</p>
            </div>
          );
        })}
      </div>

      {/* Quick Stats */}
      <div className="mt-12 grid grid-cols-2 md:grid-cols-4 gap-4 p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        {[
          { label: 'Active Databases', value: '0', color: 'var(--accentPrimary)' },
          { label: 'Active Shares', value: '0', color: 'var(--accentSecondary)' },
          { label: 'Queries Today', value: '0', color: 'var(--accentWarning)' },
          { label: 'Connected Peers', value: '0', color: 'var(--accentSuccess)' },
        ].map((stat, index) => (
          <div key={index} className="text-center">
            <div className="text-3xl font-bold mb-1" style={{ color: stat.color }}>{stat.value}</div>
            <div className="text-xs" style={{ color: 'var(--textMuted)' }}>{stat.label}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

