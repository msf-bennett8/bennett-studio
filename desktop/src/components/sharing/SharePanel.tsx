import React, { useState } from 'react';
import { useShareStore } from '../../stores/shareStore';
import { useDatabaseStore } from '../../stores/databaseStore';
import { ShareLink } from './ShareLink';
import { ShareSettings } from './ShareSettings';
import { GuestList } from './GuestList';
import { ApiKeyPanel } from './ApiKeyPanel';

export const SharePanel: React.FC = () => {
  const [selectedDb, setSelectedDb] = useState<string>('');
  const { shares, createShare, revokeShare, loading } = useShareStore();
  const { databases } = useDatabaseStore();

  const handleCreateShare = async (settings: {
    permission: 'ro' | 'rw' | 'adm';
    tables: string[];
    durationHours: number;
  }) => {
    if (!selectedDb) return;
    await createShare({
      database_id: selectedDb,
      permission: settings.permission,
      tables: settings.tables,
      duration_hours: settings.durationHours,
    });
  };

  return (
    <div className="share-panel p-4">
      <h2 className="text-xl font-bold mb-4">Database Sharing</h2>
      
      <div className="mb-4">
        <label className="block text-sm font-medium mb-1">Select Database</label>
        <select
          value={selectedDb}
          onChange={(e) => setSelectedDb(e.target.value)}
          className="w-full p-2 border rounded"
        >
          <option value="">Choose a database...</option>
          {databases.map((db) => (
            <option key={db.id} value={db.id}>
              {db.name} ({db.type})
            </option>
          ))}
        </select>
      </div>

      {selectedDb && (
        <ShareSettings onCreate={handleCreateShare} loading={loading} />
      )}

      <div className="mt-6">
        <h3 className="text-lg font-semibold mb-2">Active Shares</h3>
        {shares.map((share) => (
          <ShareLink
            key={share.code}
            share={share}
            onRevoke={() => revokeShare(share.code)}
          />
        ))}
      </div>

      <GuestList />

      <hr className="my-6" />

      <ApiKeyPanel />
    </div>
  );
};
