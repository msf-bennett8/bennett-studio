import { useEffect, useState } from 'react';
import { Sliders, Copy, Trash2, Terminal } from 'lucide-react';
import { settingsApi, EngineInfo } from '../../services/settingsApi';

const NAME_KEY = 'bennett_engine_name';

export function GeneralSettings() {
  const [info, setInfo] = useState<EngineInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState(() => localStorage.getItem(NAME_KEY) || 'My Bennett Engine');

  useEffect(() => {
    settingsApi.getEngineInfo().then(setInfo).catch(() => {}).finally(() => setLoading(false));
  }, []);

  const handleNameBlur = () => {
    localStorage.setItem(NAME_KEY, name.slice(0, 32));
  };

  const handleCopy = (text: string) => navigator.clipboard.writeText(text);

  const handleResetPreferences = () => {
    if (!confirm('Reset local preferences (name, theme, notification settings)? This does not affect your databases or shares.')) return;
    localStorage.clear();
    window.location.reload();
  };

  return (
    <div className="space-y-6">
      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-3 mb-4">
          <Sliders size={20} style={{ color: 'var(--accentPrimary)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>General</h2>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Engine Name</label>
            <p className="text-xs mb-2" style={{ color: 'var(--textMuted)' }}>
              A visible name for this engine, stored locally on this device — useful if you run Bennett Studio on multiple machines. Max 32 characters.
            </p>
            <input
              type="text"
              value={name}
              maxLength={32}
              onChange={(e) => setName(e.target.value)}
              onBlur={handleNameBlur}
              className="input w-full max-w-sm"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)', border: '1px solid var(--borderDefault)' }}
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Host ID</label>
            <p className="text-xs mb-2" style={{ color: 'var(--textMuted)' }}>
              This engine's stable identifier used by the relay to route shares and API key traffic to this machine.
            </p>
            <div className="flex items-center gap-2">
              <code className="text-sm px-3 py-2 rounded-lg flex-1" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)' }}>
                {loading ? 'Loading...' : info?.host_id}
              </code>
              {info?.host_id && (
                <button onClick={() => handleCopy(info.host_id)} className="p-2 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)' }}>
                  <Copy size={16} style={{ color: 'var(--textSecondary)' }} />
                </button>
              )}
            </div>
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
            { label: 'Version', value: info ? `v${info.version}` : '...' },
            { label: 'Relay URL', value: info?.relay_url || '...' },
            { label: 'Data Directory', value: info?.data_dir || '...' },
            { label: 'Databases', value: info ? String(info.database_count) : '...' },
            { label: 'Active Shares', value: info ? String(info.active_share_count) : '...' },
            { label: 'Protocol', value: 'gRPC + WebSocket' },
          ].map((item, i) => (
            <div key={i} className="p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>{item.label}</p>
              <p className="text-sm font-mono break-all" style={{ color: 'var(--textPrimary)' }}>{item.value}</p>
            </div>
          ))}
        </div>
      </div>

      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--accentError)' }}>
        <h2 className="text-lg font-semibold mb-2" style={{ color: 'var(--accentError)' }}>Danger Zone</h2>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Reset local preferences</p>
            <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Clears engine name, theme, and notification settings stored in this app. Databases and shares are unaffected.</p>
          </div>
          <button onClick={handleResetPreferences} className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}>
            <Trash2 size={14} /> Reset
          </button>
        </div>
      </div>
    </div>
  );
}
