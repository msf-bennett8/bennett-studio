import React, { useEffect, useState } from 'react';
import { useApiKeysStore } from '../../stores/apiKeysStore';
import { useDatabaseStore } from '../../stores/databaseStore';
import { ApiKeyItem } from './ApiKeyItem';

const isRemoteMode = () => {
  const url = (import.meta as any).env?.VITE_API_URL || '';
  return url.includes('onrender.com') || url.includes('vercel.app') || window.location.hostname.includes('vercel.app');
};

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
    if (!isRemoteMode()) fetchKeys();
  }, [fetchKeys]);

  if (isRemoteMode()) {
    return (
      <div className="api-key-panel p-4 text-center text-gray-500">
        API key management is only available when connected to your local engine —
        open the desktop app or run this page pointed at <code>localhost</code>.
      </div>
    );
  }

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
        <div className="mb-4 p-2 bg-red-50 text-red-700 text-sm rounded flex justify-between">
          <span>{error}</span>
          <button onClick={clearError} className="font-bold">×</button>
        </div>
      )}

      {justCreatedKey && (
        <div className="mb-4 p-3 bg-yellow-50 border border-yellow-300 rounded">
          <p className="text-sm font-semibold mb-1">Save this key now — it won't be shown again:</p>
          <div className="flex gap-2 items-center">
            <code className="text-xs bg-white p-2 rounded flex-1 truncate">{justCreatedKey}</code>
            <button onClick={handleCopyKey} className="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded">Copy</button>
            <button onClick={dismissJustCreatedKey} className="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded">Dismiss</button>
          </div>
        </div>
      )}

      <div className="mb-4 space-y-2">
        <div>
          <label className="block text-sm font-medium mb-1">Database</label>
          <select value={selectedDb} onChange={(e) => setSelectedDb(e.target.value)} className="w-full p-2 border rounded">
            <option value="">Choose a database...</option>
            {databases.map((db) => (
              <option key={db.id} value={db.id}>{db.name} ({db.type})</option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Key name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. oshocks-backend"
            className="w-full p-2 border rounded"
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Permission</label>
          <select value={permission} onChange={(e) => setPermission(e.target.value as 'ro' | 'rw' | 'adm')} className="w-full p-2 border rounded">
            <option value="ro">Read only</option>
            <option value="rw">Read/write</option>
            <option value="adm">Admin</option>
          </select>
        </div>
        <button
          onClick={handleCreate}
          disabled={!selectedDb || !name.trim() || creating}
          className="px-4 py-2 bg-blue-600 text-white rounded disabled:opacity-50"
        >
          {creating ? 'Generating...' : 'Generate API Key'}
        </button>
      </div>

      <div className="mt-6">
        <h3 className="text-lg font-semibold mb-2">Active Keys</h3>
        {loading && <p className="text-sm text-gray-500">Loading...</p>}
        {!loading && keys.length === 0 && <p className="text-sm text-gray-500">No API keys yet.</p>}
        {keys.map((k) => (
          <ApiKeyItem key={k.id} apiKey={k} onRevoke={() => revokeKey(k.id)} />
        ))}
      </div>
    </div>
  );
};
