import { useRemoteConnectionStore } from '../../stores/remoteConnectionStore';

export const ShareSession: React.FC = () => {
  const { connections, activeConnectionId, disconnect } = useRemoteConnectionStore();
  const remoteConnection = connections.find(c => c.id === activeConnectionId);

  if (!remoteConnection) {
    return (
      <div className="share-session p-4 text-center text-gray-500">
        No active share session. Join a share to get started.
      </div>
    );
  }

  return (
    <div className="share-session p-4 border rounded bg-white">
      <div className="flex justify-between items-center mb-3">
        <h3 className="text-lg font-semibold">Connected to {remoteConnection.dbName}</h3>
        <button
          onClick={() => disconnect(remoteConnection.id)}
          className="px-3 py-1 text-sm bg-red-100 text-red-700 rounded hover:bg-red-200"
        >
          Disconnect
        </button>
      </div>
      
      <div className="text-sm text-gray-600 space-y-1">
        <p>Database: {remoteConnection.dbName} ({remoteConnection.dbType})</p>
        <p>Permission: {remoteConnection.permission}</p>
        <p>Tables: {remoteConnection.tables.join(', ')}</p>
        <p>Connected: {new Date(remoteConnection.connectedAt).toLocaleString()}</p>
      </div>
    </div>
  );
};
