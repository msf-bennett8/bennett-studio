import { useState } from 'react';
import { Shield, Database } from 'lucide-react';

export function PrivacySettings() {
  const [telemetry, setTelemetry] = useState(false);

  return (
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
  );
}
