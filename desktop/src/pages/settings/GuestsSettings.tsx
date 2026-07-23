import { useEffect, useState } from 'react';
import { Users, LogOut } from 'lucide-react';
import { settingsApi, GuestSession } from '../../services/settingsApi';

export function GuestsSettings() {
  const [guests, setGuests] = useState<GuestSession[]>([]);
  const [loading, setLoading] = useState(true);

  const load = () => {
    settingsApi.listGuests().then(setGuests).catch(() => {}).finally(() => setLoading(false));
  };

  useEffect(() => { load(); }, []);

  const handleDisconnect = async (id: string) => {
    const ok = await settingsApi.disconnectGuest(id).catch(() => false);
    if (ok) setGuests((g) => g.filter((x) => x.id !== id));
  };

  return (
    <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
      <div className="flex items-center gap-3 mb-4">
        <Users size={20} style={{ color: 'var(--accentPrimary)' }} />
        <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Guests</h2>
      </div>
      <p className="text-sm mb-4" style={{ color: 'var(--textMuted)' }}>
        Everyone currently connected to your shared databases via a share link.
      </p>

      {loading && <p className="text-sm" style={{ color: 'var(--textMuted)' }}>Loading...</p>}
      {!loading && guests.length === 0 && <p className="text-sm" style={{ color: 'var(--textMuted)' }}>No guests connected right now.</p>}

      <div className="space-y-2">
        {guests.map((g) => (
          <div key={g.id} className="flex items-center justify-between p-3 rounded-lg" style={{ backgroundColor: 'var(--bgSecondary)' }}>
            <div>
              <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{g.ip_address || 'Unknown IP'}</p>
              <div className="flex gap-2 text-xs mt-0.5" style={{ color: 'var(--textMuted)' }}>
                <span>Share: {g.share_code}</span>
                <span>•</span>
                <span>{g.query_count} queries</span>
                <span>•</span>
                <span>Connected: {new Date(g.connected_at).toLocaleString()}</span>
                <span>•</span>
                <span>Last active: {new Date(g.last_active).toLocaleString()}</span>
              </div>
            </div>
            <button onClick={() => handleDisconnect(g.id)} className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}>
              <LogOut size={12} /> Disconnect
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
