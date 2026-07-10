import { useState, useCallback } from 'react';
import { clientFromUrl } from '@bennettstudio/sdk';
import type { RemoteConnection } from '@bennettstudio/shared';

export function useShareSession() {
  const [connection, setConnection] = useState<RemoteConnection | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const connect = useCallback(async (url: string): Promise<boolean> => {
    setLoading(true);
    setError(null);

    try {
      const client = clientFromUrl(url);
      const info = await client.getSchema();

      if (!info.success) {
        throw new Error('Failed to validate share');
      }

      const newConnection: RemoteConnection = {
        id: crypto.randomUUID(),
        code: client['code'],
        token: client['token'],
        baseUrl: client['baseUrl'],
        dbId: info.databaseName,
        dbName: info.databaseName,
        dbType: info.databaseType,
        permission: 'ro',
        tables: info.tables.map(t => t.name),
        connectedAt: new Date().toISOString(),
        lastActivity: new Date().toISOString(),
        status: 'connected',
        shareUrl: url,
      };

      setConnection(newConnection);
      return true;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Connection failed';
      setError(msg);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  const disconnect = useCallback(() => {
    setConnection(null);
    setError(null);
  }, []);

  return {
    connection,
    connect,
    disconnect,
    loading,
    error,
    clearError: () => setError(null),
  };
}
