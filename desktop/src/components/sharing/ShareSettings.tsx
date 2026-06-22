import React, { useState } from 'react';

interface ShareSettingsProps {
  onCreate: (settings: {
    permission: 'ro' | 'rw' | 'adm';
    tables: string[];
    durationHours: number;
  }) => void;
  loading: boolean;
}

export const ShareSettings: React.FC<ShareSettingsProps> = ({ onCreate, loading }) => {
  const [permission, setPermission] = useState<'ro' | 'rw' | 'adm'>('ro');
  const [tables, setTables] = useState<string>('*');
  const [durationHours, setDurationHours] = useState(24);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onCreate({
      permission,
      tables: tables.split(',').map(t => t.trim()).filter(Boolean),
      durationHours,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="share-settings border rounded p-4 bg-gray-50">
      <h3 className="text-md font-semibold mb-3">Share Settings</h3>
      
      <div className="mb-3">
        <label className="block text-sm font-medium mb-1">Permission</label>
        <select
          value={permission}
          onChange={(e) => setPermission(e.target.value as 'ro' | 'rw' | 'adm')}
          className="w-full p-2 border rounded"
        >
          <option value="ro">Read Only</option>
          <option value="rw">Read & Write</option>
          <option value="adm">Admin (DDL allowed)</option>
        </select>
      </div>

      <div className="mb-3">
        <label className="block text-sm font-medium mb-1">Tables (comma-separated, * for all)</label>
        <input
          type="text"
          value={tables}
          onChange={(e) => setTables(e.target.value)}
          placeholder="users, orders, products"
          className="w-full p-2 border rounded"
        />
      </div>

      <div className="mb-3">
        <label className="block text-sm font-medium mb-1">Duration (hours)</label>
        <input
          type="number"
          value={durationHours}
          onChange={(e) => setDurationHours(Number(e.target.value))}
          min={1}
          max={168}
          className="w-full p-2 border rounded"
        />
      </div>

      <button
        type="submit"
        disabled={loading}
        className="w-full py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
      >
        {loading ? 'Creating...' : 'Create Share Link'}
      </button>
    </form>
  );
};
