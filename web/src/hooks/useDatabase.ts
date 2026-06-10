import { useEffect, useCallback } from 'react';
import { useDatabaseStore } from '../stores/databaseStore';

export function useDatabase() {
  const store = useDatabaseStore();

  useEffect(() => {
    store.fetchDatabases();
  }, []);

  const refresh = useCallback(() => {
    store.fetchDatabases();
  }, []);

  return {
    databases: store.databases,
    loading: store.loading,
    error: store.error,
    selectedDatabase: store.selectedDatabase,
    createDatabase: store.createDatabase,
    deleteDatabase: store.deleteDatabase,
    startDatabase: store.startDatabase,
    stopDatabase: store.stopDatabase,
    selectDatabase: store.selectDatabase,
    clearError: store.clearError,
    refresh,
  };
}
