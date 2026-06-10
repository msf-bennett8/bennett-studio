import { useState } from 'react';
import { Share2, Copy, Check, Globe, Lock, Users, Clock, X } from 'lucide-react';

interface ShareSession {
  id: string; databaseName: string; databaseType: string; url: string;
  status: 'active' | 'expired' | 'revoked'; createdAt: string; expiresAt: string;
  guests: number; permissions: string;
}

const mockShares: ShareSession[] = [
  {
    id: '1', databaseName: 'local-postgres', databaseType: 'PostgreSQL',
    url: 'https://share.bennett.studio/db/abc-123-def', status: 'active',
    createdAt: '2024-06-10 14:30', expiresAt: '2024-06-11 14:30', guests: 3, permissions: 'Read-only',
  },
];

export function SharePage() {
  const [shares, setShares] = useState<ShareSession[]>(mockShares);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [selectedDb, setSelectedDb] = useState('local-postgres');
  const [permission, setPermission] = useState('read-only');
  const [duration, setDuration] = useState('24h');

  const handleCopy = (url: string, id: string) => {
    navigator.clipboard.writeText(url);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleRevoke = (id: string) => { setShares(shares.map(s => s.id === id ? { ...s, status: 'revoked' as const } : s)); };

  const handleCreate = () => {
    const newShare: ShareSession = {
      id: Date.now().toString(), databaseName: selectedDb, databaseType: 'PostgreSQL',
      url: `https://share.bennett.studio/db/${Math.random().toString(36).substring(2, 15)}`,
      status: 'active', createdAt: new Date().toLocaleString(),
      expiresAt: new Date(Date.now() + 24 * 60 * 60 * 1000).toLocaleString(),
      guests: 0, permissions: permission === 'read-only' ? 'Read-only' : 'Read-write',
    };
    setShares([...shares, newShare]);
    setShowCreateModal(false);
  };

  return (
    <div className="p-8 max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold" style={{ color: 'var(--textPrimary)' }}>Share Access</h1>
          <p className="text-sm mt-1" style={{ color: 'var(--textSecondary)' }}>Create secure sharing links for your databases</p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
          <Share2 size={18} /> New Share
        </button>
      </div>

      <div className="space-y-4">
        {shares.map((share) => (
          <div key={share.id} className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ backgroundColor: share.status === 'active' ? 'rgba(0,212,170,0.1)' : 'rgba(255,68,68,0.1)' }}>
                  {share.status === 'active' ? <Globe size={24} style={{ color: 'var(--accentSuccess)' }} /> : <Lock size={24} style={{ color: 'var(--accentError)' }} />}
                </div>
                <div>
                  <h3 className="font-semibold" style={{ color: 'var(--textPrimary)' }}>{share.databaseName}</h3>
                  <div className="flex items-center gap-3 mt-1">
                    <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{share.databaseType}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{share.permissions}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}><Users size={12} className="inline mr-1" />{share.guests} guests</span>
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <div className="flex items-center gap-2 px-3 py-1 rounded-full" style={{ backgroundColor: share.status === 'active' ? 'rgba(0,212,170,0.1)' : 'rgba(255,68,68,0.1)' }}>
                  <div className="w-2 h-2 rounded-full" style={{ backgroundColor: share.status === 'active' ? 'var(--accentSuccess)' : 'var(--accentError)' }} />
                  <span className="text-xs font-medium" style={{ color: share.status === 'active' ? 'var(--accentSuccess)' : 'var(--accentError)' }}>{share.status}</span>
                </div>
                {share.status === 'active' && (
                  <>
                    <button onClick={() => handleCopy(share.url, share.id)} className="p-2 rounded-lg transition-all" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Copy link">
                      {copiedId === share.id ? <Check size={16} style={{ color: 'var(--accentSuccess)' }} /> : <Copy size={16} />}
                    </button>
                    <button onClick={() => handleRevoke(share.id)} className="p-2 rounded-lg transition-all hover:bg-red-500/20" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Revoke access">
                      <X size={16} style={{ color: 'var(--accentError)' }} />
                    </button>
                  </>
                )}
              </div>
            </div>
            {share.status === 'active' && (
              <div className="mt-4 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                <div className="flex items-center justify-between">
                  <code className="text-sm font-mono" style={{ color: 'var(--accentSecondary)' }}>{share.url}</code>
                  <span className="text-xs" style={{ color: 'var(--textMuted)' }}><Clock size={12} className="inline mr-1" />Expires: {share.expiresAt}</span>
                </div>
              </div>
            )}
          </div>
        ))}
        {shares.length === 0 && (
          <div className="text-center py-16 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px dashed var(--borderDefault)' }}>
            <Share2 size={48} className="mx-auto mb-4" style={{ color: 'var(--textMuted)' }} />
            <p style={{ color: 'var(--textSecondary)' }}>No active shares</p>
            <p className="text-sm mt-1" style={{ color: 'var(--textMuted)' }}>Create a share to collaborate with your team</p>
          </div>
        )}
      </div>

      {showCreateModal && (
        <div className="fixed inset-0 flex items-center justify-center z-50" style={{ backgroundColor: 'var(--bgOverlay)' }}>
          <div className="w-full max-w-md p-6 rounded-2xl" style={{ backgroundColor: 'var(--bgElevated)', border: '1px solid var(--borderDefault)' }}>
            <h2 className="text-xl font-bold mb-6" style={{ color: 'var(--textPrimary)' }}>Create Share Link</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database</label>
                <select value={selectedDb} onChange={(e) => setSelectedDb(e.target.value)} className="input">
                  <option value="local-postgres">local-postgres (PostgreSQL 16.2)</option>
                  <option value="dev-mysql">dev-mysql (MySQL 8.0)</option>
                </select>
              </div>
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Permissions</label>
                <div className="flex gap-2">
                  <button onClick={() => setPermission('read-only')} className="flex-1 p-3 rounded-xl text-sm font-medium transition-all"
                    style={{ backgroundColor: permission === 'read-only' ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: permission === 'read-only' ? 'var(--textInverse)' : 'var(--textSecondary)' }}>
                    <Lock size={14} className="inline mr-2" />Read-only
                  </button>
                  <button onClick={() => setPermission('read-write')} className="flex-1 p-3 rounded-xl text-sm font-medium transition-all"
                    style={{ backgroundColor: permission === 'read-write' ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: permission === 'read-write' ? 'var(--textInverse)' : 'var(--textSecondary)' }}>
                    <Globe size={14} className="inline mr-2" />Read-write
                  </button>
                </div>
              </div>
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Duration</label>
                <div className="flex gap-2">
                  {['1h', '24h', '7d', '30d'].map((d) => (
                    <button key={d} onClick={() => setDuration(d)} className="px-4 py-2 rounded-xl text-sm font-medium transition-all"
                      style={{ backgroundColor: duration === d ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: duration === d ? 'var(--textInverse)' : 'var(--textSecondary)' }}>{d}</button>
                  ))}
                </div>
              </div>
            </div>
            <div className="flex gap-3 mt-6">
              <button onClick={() => setShowCreateModal(false)} className="btn-secondary flex-1 py-2 rounded-xl">Cancel</button>
              <button onClick={handleCreate} className="btn-primary flex-1 py-2 rounded-xl">Create Share</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

