import { useState, useEffect } from 'react';
import { Share2, Copy, Check, Globe, Lock, Users, Clock, X, Trash2, Pin, PinOff, AlertCircle, Loader2 } from 'lucide-react';
import { ConfirmModal } from '../components/ui/ConfirmModal';
import { useDatabaseStore } from '../stores/databaseStore';
import { useShareStore } from '../stores/shareStore';
import type { ShareLink, SharePermission } from '@bennett/shared';

export function SharePage() {
  const { databases } = useDatabaseStore();
  const { shares, loading, error, creating, fetchShares, createShare, revokeShare, deleteShare, togglePin, getShareUrl, initVault, clearError } = useShareStore();
  
  const runningDbs = databases.filter(d => d.status === 'running');
  
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const [selectedDb, setSelectedDb] = useState<string>('');
  const [permission, setPermission] = useState<SharePermission>('ro');
  const [duration, setDuration] = useState<number>(24);
  const [tables, setTables] = useState<string[]>(['*']);
  const [createError, setCreateError] = useState<string | null>(null);
  const [copyError, setCopyError] = useState<string | null>(null);
  const [filter, setFilter] = useState<'active' | 'pinned' | 'all' | 'revoked' | 'expired'>('active');
  
  // Confirmation modal state
  const [confirmModal, setConfirmModal] = useState<{
    open: boolean;
    type: 'revoke' | 'delete';
    code: string;
    title: string;
    message: string;
    confirmText: string;
  } | null>(null);

  // Load shares on mount + init vault
  useEffect(() => {
    // Initialize vault first, then fetch shares
    const initAndFetch = async () => {
      await initVault();
      fetchShares();
    };
    initAndFetch();
    
    const interval = setInterval(fetchShares, 30000); // Refresh every 30s
    return () => clearInterval(interval);
  }, []);

  // Set default selected DB
  useEffect(() => {
    if (runningDbs.length > 0 && !selectedDb) {
      setSelectedDb(runningDbs[0].id);
    }
  }, [runningDbs]);

  const handleCopy = async (share: ShareLink) => {
    const fullUrl = await getShareUrl(share.code);

    if (!fullUrl || fullUrl.includes('...')) {
      // Token not in vault — show warning in the share list
      setCopyError('Full share link not available. This token was created in a different session, browser, or was cleared. Create a new share to get a fresh link.');
      setTimeout(() => setCopyError(null), 5000);
      return;
    }

    setCopyError(null);
    
    try {
      await navigator.clipboard.writeText(fullUrl);
      setCopiedCode(share.code);
      setTimeout(() => setCopiedCode(null), 2000);
    } catch {
      const textarea = document.createElement('textarea');
      textarea.value = fullUrl;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      setCopiedCode(share.code);
      setTimeout(() => setCopiedCode(null), 2000);
    }
  };

  const openRevokeConfirm = (code: string) => {
    setConfirmModal({
      open: true,
      type: 'revoke',
      code,
      title: 'Revoke Share Access',
      message: 'All connected guests will be immediately disconnected. The share will be marked as revoked but can still be viewed in history.',
      confirmText: 'Revoke Access',
    });
  };

  const openDeleteConfirm = (code: string) => {
    setConfirmModal({
      open: true,
      type: 'delete',
      code,
      title: 'Permanently Delete Share',
      message: 'This share will be permanently removed from the database. Any guests with this link will see a "not found" error.',
      confirmText: 'Delete Permanently',
    });
  };

  const handleConfirmAction = async () => {
    if (!confirmModal) return;
    const { type, code } = confirmModal;
    setConfirmModal(null);
    if (type === 'revoke') {
      await revokeShare(code);
    } else {
      await deleteShare(code);
    }
  };

  const handleCreate = async () => {
    if (!selectedDb) {
      setCreateError('Please select a database');
      return;
    }
    
    setCreateError(null);
    
    const result = await createShare({
      database_id: selectedDb,
      permission,
      tables: tables.length > 0 ? tables : ['*'],
      duration_hours: duration,
    });
    
    if (result) {
      setShowCreateModal(false);
      setSelectedDb('');
      setPermission('ro');
      setDuration(24);
      setTables(['*']);
    } else {
      setCreateError('Failed to create share. Please try again.');
    }
  };

  const getPermissionLabel = (perm: SharePermission) => {
    switch (perm) {
      case 'ro': return 'Read-only';
      case 'rw': return 'Read-write';
      case 'adm': return 'Admin';
      default: return perm;
    }
  };

  const getPermissionIcon = (perm: SharePermission) => {
    switch (perm) {
      case 'ro': return Lock;
      case 'rw': return Globe;
      case 'adm': return Users;
      default: return Lock;
    }
  };

  const formatDuration = (hours: number) => {
    if (hours < 24) return `${hours}h`;
    if (hours === 24) return '24h';
    if (hours < 168) return `${Math.floor(hours / 24)}d`;
    return '7d';
  };

  return (
    <div className="p-8 max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold" style={{ color: 'var(--textPrimary)' }}>Share Access</h1>
          <p className="text-sm mt-1" style={{ color: 'var(--textSecondary)' }}>
            {loading ? 'Loading shares...' : `${shares.filter(s => s.status === 'active').length} active share${shares.filter(s => s.status === 'active').length !== 1 ? 's' : ''}`}
          </p>
        </div>
        <button 
          onClick={() => setShowCreateModal(true)} 
          disabled={runningDbs.length === 0}
          className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl disabled:opacity-50"
        >
          <Share2 size={18} /> New Share
        </button>
      </div>

      {copyError && (
        <div className="mb-6 p-4 rounded-xl flex items-center gap-3" style={{ backgroundColor: 'rgba(255,170,0,0.1)', border: '1px solid var(--accentWarning)' }}>
          <AlertCircle size={20} style={{ color: 'var(--accentWarning)' }} />
          <div className="flex-1">
            <p style={{ color: 'var(--accentWarning)' }}>{copyError}</p>
          </div>
          <button onClick={() => setCopyError(null)} className="text-sm" style={{ color: 'var(--accentWarning)' }}>Dismiss</button>
        </div>
      )}

      {error && (
        <div className="mb-6 p-4 rounded-xl flex items-center gap-3" style={{ backgroundColor: 'rgba(255,68,68,0.1)', border: '1px solid var(--accentError)' }}>
          <AlertCircle size={20} style={{ color: 'var(--accentError)' }} />
          <div className="flex-1">
            <p style={{ color: 'var(--accentError)' }}>{error}</p>
          </div>
          <button onClick={clearError} className="text-sm" style={{ color: 'var(--accentError)' }}>Dismiss</button>
        </div>
      )}

      <div className="space-y-4">
        {loading && shares.length === 0 && (
          <div className="text-center py-16">
            <Loader2 size={32} className="animate-spin mx-auto mb-4" style={{ color: 'var(--accentPrimary)' }} />
            <p style={{ color: 'var(--textSecondary)' }}>Loading shares...</p>
          </div>
        )}

      {/* Filter tabs */}
      <div className="flex items-center gap-2 mb-4">
        {(['active', 'pinned', 'all', 'revoked', 'expired'] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className="px-3 py-1.5 rounded-lg text-xs font-medium transition-all"
            style={{
              backgroundColor: filter === f ? 'var(--accentPrimary)' : 'var(--bgTertiary)',
              color: filter === f ? 'var(--textInverse)' : 'var(--textSecondary)',
            }}
          >
            {f === 'active' ? `Active (${shares.filter(s => s.status === 'active').length})` :
             f === 'pinned' ? `Pinned (${shares.filter(s => s.pinned).length})` :
             f === 'all' ? `All (${shares.length})` :
             f === 'revoked' ? `Revoked (${shares.filter(s => s.status === 'revoked').length})` :
             `Expired (${shares.filter(s => s.status === 'expired').length})`}
          </button>
        ))}
      </div>
        
        {shares
          .filter((share) => {
            if (filter === 'active') return share.status === 'active';
            if (filter === 'pinned') return share.pinned;
            if (filter === 'revoked') return share.status === 'revoked';
            if (filter === 'expired') return share.status === 'expired';
            return true;
          })
          .sort((a, b) => {
            // Pinned shares always on top
            if (a.pinned && !b.pinned) return -1;
            if (!a.pinned && b.pinned) return 1;
            // Then sort by created_at desc
            return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
          })
          .map((share) => {
          const PermIcon = getPermissionIcon(share.permission);
          const isActive = share.status === 'active';
          const isExpired = share.status === 'expired';
          
          return (
            <div key={share.code} className="card p-6 rounded-xl" style={{ 
              backgroundColor: 'var(--surfaceDefault)', 
              border: '1px solid var(--borderDefault)',
              opacity: isActive ? 1 : 0.7
            }}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ 
                    backgroundColor: isActive ? 'rgba(0,212,170,0.1)' : 
                      isExpired ? 'rgba(255,170,0,0.1)' : 'rgba(255,68,68,0.1)' 
                  }}>
                    {isActive ? <Globe size={24} style={{ color: 'var(--accentSuccess)' }} /> : 
                      isExpired ? <Clock size={24} style={{ color: 'var(--accentWarning)' }} /> : 
                      <Lock size={24} style={{ color: 'var(--accentError)' }} />}
                  </div>
                  <div>
                    <h3 className="font-semibold" style={{ color: 'var(--textPrimary)' }}>{share.db_name || 'Unknown Database'}</h3>
                    <div className="flex items-center gap-3 mt-1 flex-wrap">
                      <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
                        {share.db_type || 'Unknown'}
                      </span>
                      <span className="text-xs flex items-center gap-1" style={{ color: 'var(--textMuted)' }}>
                        <PermIcon size={12} /> {getPermissionLabel(share.permission)}
                      </span>
                      <span className="text-xs" style={{ color: 'var(--textMuted)' }}>
                        <Users size={12} className="inline mr-1" />{share.guest_count} guest{share.guest_count !== 1 ? 's' : ''}
                      </span>
                      {share.tables.length > 0 && share.tables[0] !== '*' && (
                        <span className="text-xs" style={{ color: 'var(--textMuted)' }}>
                          {share.tables.length} table{share.tables.length !== 1 ? 's' : ''}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-2 px-3 py-1 rounded-full" style={{ 
                    backgroundColor: isActive ? 'rgba(0,212,170,0.1)' : 
                      isExpired ? 'rgba(255,170,0,0.1)' : 'rgba(255,68,68,0.1)' 
                  }}>
                    <div className="w-2 h-2 rounded-full" style={{ 
                      backgroundColor: isActive ? 'var(--accentSuccess)' : 
                        isExpired ? 'var(--accentWarning)' : 'var(--accentError)' 
                    }} />
                    <span className="text-xs font-medium" style={{ 
                      color: isActive ? 'var(--accentSuccess)' : 
                        isExpired ? 'var(--accentWarning)' : 'var(--accentError)' 
                    }}>
                      {share.status}
                    </span>
                  </div>
                  {isActive && (
                    <>
                      <button
                        onClick={() => handleCopy(share)}
                        className="p-2 rounded-lg transition-all hover:opacity-80"
                        style={{ backgroundColor: 'var(--bgTertiary)' }}
                        title="Copy full share link"
                      >
                        {copiedCode === share.code ? <Check size={16} style={{ color: 'var(--accentSuccess)' }} /> : <Copy size={16} />}
                      </button>
                      <button
                        onClick={() => togglePin(share.code)}
                        className="p-2 rounded-lg transition-all hover:opacity-80"
                        style={{ 
                          backgroundColor: share.pinned ? 'var(--accentPrimary)' : 'var(--bgTertiary)',
                        }}
                        title={share.pinned ? 'Unpin share' : 'Pin share'}
                      >
                        {share.pinned ? <PinOff size={16} style={{ color: 'var(--textInverse)' }} /> : <Pin size={16} style={{ color: 'var(--textMuted)' }} />}
                      </button>
                      <button
                        onClick={() => openRevokeConfirm(share.code)}
                        className="p-2 rounded-lg transition-all hover:bg-red-500/20"
                        style={{ backgroundColor: 'var(--bgTertiary)' }}
                        title="Revoke access"
                      >
                        <X size={16} style={{ color: 'var(--accentError)' }} />
                      </button>
                      <button
                        onClick={() => openDeleteConfirm(share.code)}
                        className="p-2 rounded-lg transition-all hover:bg-red-500/40"
                        style={{ backgroundColor: 'var(--bgTertiary)' }}
                        title="Permanently delete"
                      >
                        <Trash2 size={16} style={{ color: 'var(--accentError)' }} />
                      </button>
                    </>
                  )}
                </div>
              </div>
              {isActive && (
                <div className="mt-4 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  <div className="flex items-center justify-between gap-4">
                    <code className="text-sm font-mono truncate" style={{ color: 'var(--accentSecondary)', flex: 1 }}>{share.url}</code>
                    <span className="text-xs whitespace-nowrap" style={{ color: 'var(--textMuted)' }}>
                      <Clock size={12} className="inline mr-1" />
                      Expires: {new Date(share.expires_at).toLocaleString()}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 mt-2">
                    <span className="text-xs font-mono px-2 py-1 rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                      {share.code}
                    </span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>
                      Created: {new Date(share.created_at).toLocaleString()}
                    </span>
                  </div>
                </div>
              )}
            </div>
          );
        })}
        
        {shares.length === 0 && !loading && (
          <div className="text-center py-16 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px dashed var(--borderDefault)' }}>
            <Share2 size={48} className="mx-auto mb-4" style={{ color: 'var(--textMuted)' }} />
            <p style={{ color: 'var(--textSecondary)' }}>No active shares</p>
            <p className="text-sm mt-1" style={{ color: 'var(--textMuted)' }}>
              {runningDbs.length === 0 ? 'Start a database to create a share' : 'Create a share to collaborate with your team'}
            </p>
          </div>
        )}
      </div>

      {/* Confirm Modal */}
      {confirmModal?.open && (
        <ConfirmModal
          open={confirmModal.open}
          type={confirmModal.type}
          title={confirmModal.title}
          message={confirmModal.message}
          code={confirmModal.code}
          confirmText={confirmModal.confirmText}
          onConfirm={handleConfirmAction}
          onCancel={() => setConfirmModal(null)}
        />
      )}

      {showCreateModal && (
        <div className="fixed inset-0 flex items-center justify-center z-50" style={{ backgroundColor: 'var(--bgOverlay)' }}>
          <div className="w-full max-w-md p-6 rounded-2xl" style={{ backgroundColor: 'var(--bgElevated)', border: '1px solid var(--borderDefault)' }}>
            <h2 className="text-xl font-bold mb-6" style={{ color: 'var(--textPrimary)' }}>Create Share Link</h2>
            
            {createError && (
              <div className="mb-4 p-3 rounded-xl flex items-center gap-2" style={{ backgroundColor: 'rgba(255,68,68,0.1)', border: '1px solid var(--accentError)' }}>
                <AlertCircle size={16} style={{ color: 'var(--accentError)' }} />
                <span className="text-sm" style={{ color: 'var(--accentError)' }}>{createError}</span>
              </div>
            )}
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database</label>
                <select 
                  value={selectedDb} 
                  onChange={(e) => setSelectedDb(e.target.value)} 
                  className="input w-full"
                >
                  <option value="">Select a database...</option>
                  {runningDbs.map(db => (
                    <option key={db.id} value={db.id}>{db.name} ({db.type} {db.version})</option>
                  ))}
                </select>
                {runningDbs.length === 0 && (
                  <p className="text-xs mt-1" style={{ color: 'var(--accentWarning)' }}>
                    No running databases. Start a database first.
                  </p>
                )}
              </div>
              
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Permissions</label>
                <div className="flex gap-2">
                  <button 
                    onClick={() => setPermission('ro')} 
                    className="flex-1 p-3 rounded-xl text-sm font-medium transition-all"
                    style={{ 
                      backgroundColor: permission === 'ro' ? 'var(--accentPrimary)' : 'var(--bgTertiary)', 
                      color: permission === 'ro' ? 'var(--textInverse)' : 'var(--textSecondary)' 
                    }}
                  >
                    <Lock size={14} className="inline mr-2" />Read-only
                  </button>
                  <button 
                    onClick={() => setPermission('rw')} 
                    className="flex-1 p-3 rounded-xl text-sm font-medium transition-all"
                    style={{ 
                      backgroundColor: permission === 'rw' ? 'var(--accentPrimary)' : 'var(--bgTertiary)', 
                      color: permission === 'rw' ? 'var(--textInverse)' : 'var(--textSecondary)' 
                    }}
                  >
                    <Globe size={14} className="inline mr-2" />Read-write
                  </button>
                </div>
                <p className="text-xs mt-1" style={{ color: 'var(--textMuted)' }}>
                  {permission === 'ro' ? 'Guests can only run SELECT queries' : 'Guests can run SELECT, INSERT, UPDATE, DELETE'}
                </p>
              </div>
              
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Duration</label>
                <div className="flex gap-2">
                  {[1, 24, 168].map((h) => (
                    <button 
                      key={h} 
                      onClick={() => setDuration(h)} 
                      className="px-4 py-2 rounded-xl text-sm font-medium transition-all"
                      style={{ 
                        backgroundColor: duration === h ? 'var(--accentPrimary)' : 'var(--bgTertiary)', 
                        color: duration === h ? 'var(--textInverse)' : 'var(--textSecondary)' 
                      }}
                    >
                      {formatDuration(h)}
                    </button>
                  ))}
                </div>
              </div>
              
              <div>
                <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Tables</label>
                <div className="flex items-center gap-2">
                  <input 
                    type="checkbox" 
                    checked={tables.length === 1 && tables[0] === '*'}
                    onChange={(e) => setTables(e.target.checked ? ['*'] : [])}
                    className="rounded"
                  />
                  <span className="text-sm" style={{ color: 'var(--textSecondary)' }}>All tables</span>
                </div>
                <p className="text-xs mt-1" style={{ color: 'var(--textMuted)' }}>
                  {tables[0] === '*' ? 'All tables will be accessible' : 'Specific table selection coming in Phase 2'}
                </p>
              </div>
            </div>
            
            <div className="flex gap-3 mt-6">
              <button 
                onClick={() => {
                  setShowCreateModal(false);
                  setCreateError(null);
                }} 
                className="btn-secondary flex-1 py-2 rounded-xl"
              >
                Cancel
              </button>
              <button 
                onClick={handleCreate} 
                disabled={creating || !selectedDb}
                className="btn-primary flex-1 py-2 rounded-xl disabled:opacity-50 flex items-center justify-center gap-2"
              >
                {creating ? <Loader2 size={16} className="animate-spin" /> : <Share2 size={16} />}
                {creating ? 'Creating...' : 'Create Share'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

