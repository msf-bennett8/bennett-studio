import React from 'react';
import type { ShareLink as ShareLinkType } from '@bennettstudio/shared';

interface ShareLinkProps {
  share: ShareLinkType;
  onRevoke: () => void;
}

export const ShareLink: React.FC<ShareLinkProps> = ({ share, onRevoke }) => {
  const handleCopy = () => {
    navigator.clipboard.writeText(share.url);
  };

  const getStatusColor = () => {
    switch (share.status) {
      case 'active': return 'text-green-600';
      case 'expired': return 'text-yellow-600';
      case 'revoked': return 'text-red-600';
      default: return 'text-gray-600';
    }
  };

  return (
    <div className="share-link border rounded p-3 mb-2 bg-white shadow-sm">
      <div className="flex justify-between items-start">
        <div className="flex-1 min-w-0">
          <p className="text-sm font-mono text-blue-600 truncate">{share.url}</p>
          <div className="flex gap-2 mt-1 text-xs text-gray-500">
            <span className={getStatusColor()}>{share.status}</span>
            <span>•</span>
            <span>{share.permission}</span>
            <span>•</span>
            <span>Guests: {share.guest_count}</span>
            <span>•</span>
            <span>Expires: {new Date(share.expires_at).toLocaleString()}</span>
          </div>
        </div>
        <div className="flex gap-2 ml-2">
          <button
            onClick={handleCopy}
            className="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded"
          >
            Copy
          </button>
          {share.status === 'active' && (
            <button
              onClick={onRevoke}
              className="px-2 py-1 text-xs bg-red-100 text-red-700 hover:bg-red-200 rounded"
            >
              Revoke
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
