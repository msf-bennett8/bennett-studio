import React from 'react';
import type { ApiKeyInfo } from '../../services/apiKeysApi';

interface ApiKeyItemProps {
  apiKey: ApiKeyInfo;
  onRevoke: () => void;
}

export const ApiKeyItem: React.FC<ApiKeyItemProps> = ({ apiKey, onRevoke }) => {
  const handleCopyId = () => {
    navigator.clipboard.writeText(apiKey.id);
  };

  return (
    <div className="api-key-item border rounded p-3 mb-2 bg-white shadow-sm">
      <div className="flex justify-between items-start">
        <div className="flex-1 min-w-0">
          <p className="text-sm font-semibold">{apiKey.name}</p>
          <p className="text-sm font-mono text-gray-500 truncate">{apiKey.key_preview}</p>
          <div className="flex gap-2 mt-1 text-xs text-gray-500">
            <span className={apiKey.revoked ? 'text-red-600' : 'text-green-600'}>
              {apiKey.revoked ? 'revoked' : 'active'}
            </span>
            <span>•</span>
            <span>{apiKey.permission}</span>
            <span>•</span>
            <span>{apiKey.max_rows} rows / {apiKey.timeout_secs}s</span>
            <span>•</span>
            <span>Last used: {apiKey.last_used_at ? new Date(apiKey.last_used_at).toLocaleString() : 'never'}</span>
          </div>
        </div>
        <div className="flex gap-2 ml-2">
          <button onClick={handleCopyId} className="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded">
            Copy ID
          </button>
          {!apiKey.revoked && (
            <button onClick={onRevoke} className="px-2 py-1 text-xs bg-red-100 text-red-700 hover:bg-red-200 rounded">
              Revoke
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
