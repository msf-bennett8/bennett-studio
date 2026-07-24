import React, { useEffect, useState } from 'react';
import { useApiKeysStore } from '../../stores/apiKeysStore';
import { useDatabaseStore } from '../../stores/databaseStore';
import { ApiKeyItem } from './ApiKeyItem';
import { ConfirmModal } from '../ui/ConfirmModal';

const isRemoteMode = () => {
  const url = (import.meta as any).env?.VITE_API_URL || '';
  return url.includes('onrender.com') || url.includes('vercel.app') || window.location.hostname.includes('vercel.app');
};

export const ApiKeyPanel: React.FC = () => {
  const [selectedDb, setSelectedDb] = useState<string>('');
  const [name, setName] = useState('');
  const [permission, setPermission] = useState<'ro' | 'rw' | 'adm'>('ro');
  const [enableWireAccess, setEnableWireAccess] = useState(false);
  const [wireUsername, setWireUsername] = useState('');
  const [wirePassword, setWirePassword] = useState('');
  const {
    keys, loading, creating, error, justCreatedKey, justCreatedWireCreds,
    fetchKeys, createKey, revokeKey, deleteKey, dismissJustCreatedKey, clearError,
  } = useApiKeysStore();
  const { databases } = useDatabaseStore();

  const [confirmModal, setConfirmModal] = useState<{
    open: boolean;
    type: 'revoke' | 'delete';
    id: string;
    title: string;
    message: string;
    confirmText: string;
  } | null>(null);

  const openRevokeConfirm = (id: string, name: string) => {
    setConfirmModal({
      open: true,
      type: 'revoke',
      id,
      title: 'Revoke API Key',
      message: `"${name}" will immediately stop working for any external app using it. The key stays visible in history and can be permanently deleted afterward.`,
      confirmText: 'Revoke Key',
    });
  };

  const openDeleteConfirm = (id: string, name: string) => {
    setConfirmModal({
      open: true,
      type: 'delete',
      id,
      title: 'Permanently Delete API Key',
      message: `"${name}" will be permanently removed. This cannot be undone.`,
      confirmText: 'Delete Permanently',
    });
  };

  const handleConfirmAction = async () => {
    if (!confirmModal) return;
    const { type, id } = confirmModal;
    setConfirmModal(null);
    if (type === 'revoke') {
      await revokeKey(id);
    } else {
      await deleteKey(id);
    }
  };

  useEffect(() => {
    if (!isRemoteMode()) fetchKeys();
  }, [fetchKeys]);

  if (isRemoteMode()) {
    return (
      <div className="api-key-panel p-4 text-center" style={{ color: 'var(--textMuted)' }}>
        API key management is only available when connected to your local engine —
        open the desktop app or run this page pointed at <code style={{ color: 'var(--textPrimary)' }}>localhost</code>.
      </div>
    );
  }

  const handleCreate = async () => {
    if (!selectedDb || !name.trim()) return;
    const ok = await createKey({
      database_id: selectedDb,
      name: name.trim(),
      permission,
      tables: ['*'],
      enable_wire_access: enableWireAccess,
      wire_username: wireUsername.trim() || undefined,
      wire_password: wirePassword.trim() || undefined,
    });
    if (ok) {
      setName('');
      setEnableWireAccess(false);
      setWireUsername('');
      setWirePassword('');
    }
  };

  const handleCopyKey = () => {
    if (justCreatedKey) navigator.clipboard.writeText(justCreatedKey);
  };

  return (
    <div className="api-key-panel p-4">
      <h2 className="text-xl font-bold mb-4" style={{ color: 'var(--textPrimary)' }}>API Keys</h2>
      <p className="text-sm mb-4" style={{ color: 'var(--textMuted)' }}>
        Durable keys for external apps (e-commerce backends, etc.) to connect via the
        stable <code style={{ color: 'var(--textPrimary)' }}>/api/v1</code> gateway. Unlike share links, these don't expire until revoked.
      </p>

      {error && (
        <div className="mb-4 p-2 text-sm rounded flex justify-between" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}>
          <span>{error}</span>
          <button onClick={clearError} className="font-bold">×</button>
        </div>
      )}

      {justCreatedKey && (
        <div className="mb-4 p-3 rounded" style={{ backgroundColor: 'var(--surfaceActive)', border: '1px solid var(--borderDefault)' }}>
          <p className="text-sm font-semibold mb-1" style={{ color: 'var(--accentWarning)' }}>Save this key now — it won't be shown again:</p>
          <div className="flex gap-2 items-center">
            <code className="text-xs p-2 rounded flex-1 truncate" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)' }}>{justCreatedKey}</code>
            <button onClick={handleCopyKey} className="px-2 py-1 text-xs rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>Copy</button>
          </div>

          {justCreatedWireCreds && (
            <div className="mt-3 pt-3" style={{ borderTop: '1px solid var(--borderDefault)' }}>
              <p className="text-sm font-semibold mb-1" style={{ color: 'var(--accentWarning)' }}>MySQL/Postgres wire credentials — also shown only once:</p>
              <div className="text-xs space-y-1">
                <div className="flex gap-2 items-center">
                  <span className="w-20" style={{ color: 'var(--textMuted)' }}>Username</span>
                  <code className="p-1.5 rounded flex-1" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)' }}>{justCreatedWireCreds.username}</code>
                  <button onClick={() => navigator.clipboard.writeText(justCreatedWireCreds.username)} className="px-2 py-1 rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>Copy</button>
                </div>
                <div className="flex gap-2 items-center">
                  <span className="w-20" style={{ color: 'var(--textMuted)' }}>Password</span>
                  <code className="p-1.5 rounded flex-1" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)' }}>{justCreatedWireCreds.password}</code>
                  <button onClick={() => navigator.clipboard.writeText(justCreatedWireCreds.password)} className="px-2 py-1 rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>Copy</button>
                </div>
              </div>
            </div>
          )}

          <button onClick={dismissJustCreatedKey} className="mt-3 px-2 py-1 text-xs rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>Dismiss</button>
        </div>
      )}

      <div className="mb-4 space-y-2">
        <div>
          <label className="block text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Database</label>
          <select value={selectedDb} onChange={(e) => setSelectedDb(e.target.value)} className="w-full p-2 rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)', border: '1px solid var(--borderDefault)' }}>
            <option value="" style={{ backgroundColor: 'var(--bgTertiary)' }}>Choose a database...</option>
            {databases.map((db) => (
              <option key={db.id} value={db.id} style={{ backgroundColor: 'var(--bgTertiary)' }}>{db.name} ({db.type})</option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Key name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. oshocks-backend"
            className="w-full p-2 rounded"
            style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)', border: '1px solid var(--borderDefault)' }}
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Permission</label>
          <select value={permission} onChange={(e) => setPermission(e.target.value as 'ro' | 'rw' | 'adm')} className="w-full p-2 rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)', border: '1px solid var(--borderDefault)' }}>
            <option value="ro" style={{ backgroundColor: 'var(--bgTertiary)' }}>Read only</option>
            <option value="rw" style={{ backgroundColor: 'var(--bgTertiary)' }}>Read/write</option>
            <option value="adm" style={{ backgroundColor: 'var(--bgTertiary)' }}>Admin</option>
          </select>
        </div>
        <div className="pt-2" style={{ borderTop: '1px solid var(--borderDefault)' }}>
          <label className="flex items-center gap-2 text-sm" style={{ color: 'var(--textPrimary)' }}>
            <input type="checkbox" checked={enableWireAccess} onChange={(e) => setEnableWireAccess(e.target.checked)} />
            Enable MySQL/Postgres wire access (a real DB connection string)
          </label>
          {enableWireAccess && (
            <div className="mt-2 space-y-2">
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>
                Leave blank to auto-generate. Custom values let you choose a recognizable username/password
                (e.g. for a production <code style={{ color: 'var(--textPrimary)' }}>.env</code>) instead of a random one.
              </p>
              <input
                type="text"
                value={wireUsername}
                onChange={(e) => setWireUsername(e.target.value)}
                placeholder="Wire username (optional, e.g. bennett_oshocks)"
                className="w-full p-2 rounded text-sm"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)', border: '1px solid var(--borderDefault)' }}
              />
              <input
                type="text"
                value={wirePassword}
                onChange={(e) => setWirePassword(e.target.value)}
                placeholder="Wire password (optional)"
                className="w-full p-2 rounded text-sm"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)', border: '1px solid var(--borderDefault)' }}
              />
            </div>
          )}
        </div>
        <button
          onClick={handleCreate}
          disabled={!selectedDb || !name.trim() || creating}
          className="px-4 py-2 rounded disabled:opacity-50"
          style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}
        >
          {creating ? 'Generating...' : 'Generate API Key'}
        </button>
      </div>

      <div className="mt-6">
        <h3 className="text-lg font-semibold mb-2" style={{ color: 'var(--textPrimary)' }}>Active Keys</h3>
        {loading && <p className="text-sm" style={{ color: 'var(--textMuted)' }}>Loading...</p>}
        {!loading && keys.length === 0 && <p className="text-sm" style={{ color: 'var(--textMuted)' }}>No API keys yet.</p>}
        {keys.map((k) => (
          <ApiKeyItem
            key={k.id}
            apiKey={k}
            onRevoke={() => openRevokeConfirm(k.id, k.name)}
            onDelete={() => openDeleteConfirm(k.id, k.name)}
          />
        ))}
      </div>

      {confirmModal?.open && (
        <ConfirmModal
          open={confirmModal.open}
          type={confirmModal.type}
          title={confirmModal.title}
          message={confirmModal.message}
          code={confirmModal.id}
          confirmText={confirmModal.confirmText}
          onConfirm={handleConfirmAction}
          onCancel={() => setConfirmModal(null)}
        />
      )}
    </div>
  );
};
