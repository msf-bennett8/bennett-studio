import { useState, useEffect } from 'react';
import { Shield, Database, Lock, Trash2, ScrollText } from 'lucide-react';
import { vaultService } from '../../services/vaultService';
import { settingsApi } from '../../services/settingsApi';

export function PrivacySettings() {
  const [telemetry, setTelemetry] = useState(false);
  const [vaultStatus, setVaultStatus] = useState<{ available: boolean; type: string; initialized: boolean } | null>(null);
  const [clearingVault, setClearingVault] = useState(false);
  const [clearingActivity, setClearingActivity] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    vaultService.status?.().then(setVaultStatus).catch(() => setVaultStatus(null));
  }, []);

  const handleClearVault = async () => {
    if (!confirm('Clear all locally stored share tokens? You will need to re-join any active shares.')) return;
    setClearingVault(true);
    try {
      await vaultService.clear();
      setMessage('Local vault cleared.');
    } catch {
      setMessage('Failed to clear vault.');
    } finally {
      setClearingVault(false);
    }
  };

  const handleClearActivity = async () => {
    if (!confirm('Permanently delete your local query activity log? This cannot be undone.')) return;
    setClearingActivity(true);
    try {
      await settingsApi.clearActivity();
      setMessage('Activity log cleared.');
    } catch {
      setMessage('Failed to clear activity log.');
    } finally {
      setClearingActivity(false);
    }
  };

  return (
    <div className="space-y-6">
      {message && (
        <div className="p-3 rounded-xl text-sm" style={{ backgroundColor: 'var(--surfaceActive)', color: 'var(--textPrimary)' }}>
          {message}
        </div>
      )}

      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-3 mb-4">
          <Shield size={20} style={{ color: 'var(--accentError)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Privacy & Security</h2>
        </div>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Telemetry</p>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>
                Reserved for future use — Bennett Studio does not currently collect or transmit any usage data.
              </p>
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
          <Lock size={20} style={{ color: 'var(--accentInfo)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Local Vault</h2>
        </div>
        <p className="text-xs mb-3" style={{ color: 'var(--textMuted)' }}>
          Share tokens are stored encrypted on this device using {vaultStatus?.type === 'tauri_secure' ? "your OS's secure keychain" : vaultStatus?.type === 'indexeddb_encrypted' ? 'encrypted browser storage' : 'in-memory storage (cleared on close)'}.
        </p>
        <div className="flex items-center justify-between p-4 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
          <div>
            <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>
              Status: {vaultStatus === null ? 'Checking...' : vaultStatus.available ? 'Available' : 'Unavailable'}
            </p>
            <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Removing stored tokens will require re-joining any active shares.</p>
          </div>
          <button onClick={handleClearVault} disabled={clearingVault} className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm disabled:opacity-50" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}>
            <Trash2 size={14} /> {clearingVault ? 'Clearing...' : 'Clear Local Vault'}
          </button>
        </div>
      </div>

      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-3 mb-4">
          <ScrollText size={20} style={{ color: 'var(--accentWarning)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Activity Log</h2>
        </div>
        <div className="flex items-center justify-between p-4 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
          <div>
            <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Query activity is retained for 90 days</p>
            <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Automatically deleted after 90 days, or clear it immediately below.</p>
          </div>
          <button onClick={handleClearActivity} disabled={clearingActivity} className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm disabled:opacity-50" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}>
            <Trash2 size={14} /> {clearingActivity ? 'Clearing...' : 'Clear Activity Log'}
          </button>
        </div>
      </div>
    </div>
  );
}
