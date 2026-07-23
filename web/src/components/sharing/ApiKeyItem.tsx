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
    <div className="api-key-item rounded p-3 mb-2" style={{ backgroundColor: 'var(--bgSecondary)', border: '1px solid var(--borderDefault)' }}>
      <div className="flex justify-between items-start">
        <div className="flex-1 min-w-0">
          <p className="text-sm font-semibold" style={{ color: 'var(--textPrimary)' }}>{apiKey.name}</p>
          <p className="text-sm font-mono truncate" style={{ color: 'var(--textMuted)' }}>{apiKey.key_preview}</p>
          <div className="flex gap-2 mt-1 text-xs" style={{ color: 'var(--textMuted)' }}>
            <span style={{ color: apiKey.revoked ? 'var(--accentError)' : 'var(--accentSuccess)' }}>
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
          <button onClick={handleCopyId} className="px-2 py-1 text-xs rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
            Copy ID
          </button>
          {!apiKey.revoked && (
            <button onClick={onRevoke} className="px-2 py-1 text-xs rounded" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}>
              Revoke
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
