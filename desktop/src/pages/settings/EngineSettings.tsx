import { Terminal } from 'lucide-react';

export function EngineSettings() {
  return (
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
  );
}
