import React, { useEffect, useState } from 'react';
import { useApiKeysStore } from '../../stores/apiKeysStore';
import { useDatabaseStore } from '../../stores/databaseStore';
import { ApiKeyItem } from './ApiKeyItem';

export const ApiKeyPanel: React.FC = () => {
  const [selectedDb, setSelectedDb] = useState<string>('');
  const [name, setName] = useState('');
  const [permission, setPermission] = useState<'ro' | 'rw' | 'adm'>('ro');
  const {
    keys, loading, creating, error, justCreatedKey,
    fetchKeys, createKey, revokeKey, dismissJustCreatedKey, clearError,
  } = useApiKeysStore();
  const { databases } = useDatabaseStore();

  useEffect(() => {
    fetchKeys();
  }, [fetchKeys]);

  const handleCreate = async () => {
    if (!selectedDb || !name.trim()) return;
    const ok = await createKey({ database_id: selectedDb, name: name.trim(), permission, tables: ['*'] });
    if (ok) setName('');
  };

  const handleCopyKey = () => {
    if (justCreatedKey) navigator.clipboard.writeText(justCreatedKey);
  };

  return (
    <div className="api-key-panel p-4">
      <h2 className="text-xl font-bold mb-4">API Keys</h2>
      <p className="text-sm text-gray-500 mb-4">
        Durable keys for external apps (e-commerce backends, etc.) to connect via the
        stable <code>/api/v1</code> gateway. Unlike share links, these don't expire until revoked.
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
            <button onClick={dismissJustCreatedKey} className="px-2 py-1 text-xs rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>Dismiss</button>
          </div>
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
          <ApiKeyItem key={k.id} apiKey={k} onRevoke={() => revokeKey(k.id)} />
        ))}
      </div>
    </div>
  );
};
