import { useState, useEffect } from 'react';
import { Table2, Columns, Key, Link2, AlertCircle } from 'lucide-react';
import { api } from '../services/api';
import { useDatabaseStore } from '../stores/databaseStore';
import { useRemoteConnectionStore } from '../stores/remoteConnectionStore';
import { remoteApi } from '../services/remoteApi';

interface ColumnInfo {
  name: string; type: string; nullable: boolean; default?: string;
  isPrimary: boolean; isForeign: boolean; references?: string;
}

interface TableInfo {
  name: string; columns: ColumnInfo[]; rowCount: number; size: string;
}

export function SchemaPage() {
  const { databases, selectedDatabase, selectDatabase } = useDatabaseStore();
  const { connections: remoteConnections } = useRemoteConnectionStore();
  const runningDbs = [...databases.filter(d => d.status === 'running'), ...remoteConnections
    .filter(c => c.status === 'connected')
    .map(c => ({
      id: c.id,
      name: `${c.dbName || 'Remote Database'} ${c.code}`,
      type: (c.dbType || 'postgres') as 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis',
      version: '',
      status: 'running' as const,
      port: 0,
      size: '',
      created_at: c.connectedAt,
      source: 'bennett' as any,
      isRemote: true,
      shareCode: c.code,
      remotePermission: c.permission,
      remoteHost: c.baseUrl,
    }))];

  const selectedDb = selectedDatabase?.id || '';
  const setSelectedDb = (id: string) => {
    const db = runningDbs.find(d => d.id === id);
    selectDatabase(db || null);
  };
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [selectedTable, setSelectedTable] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (runningDbs.length > 0 && !selectedDatabase) {
      selectDatabase(runningDbs[0]);
    }
  }, [runningDbs, selectedDatabase]);

  useEffect(() => {
    if (!selectedDb) return;
    setLoading(true);
    setError(null);

    const remoteDb = runningDbs.find(d => d.id === selectedDb && d.isRemote);
    if (remoteDb) {
      const conn = remoteConnections.find(c => c.id === selectedDb);
      if (!conn) {
        setError('Remote connection not found');
        setLoading(false);
        return;
      }
      remoteApi.fetchSchema(conn)
        .then(schema => {
          const mapped = schema.map(t => ({
            name: t.name,
            engine: conn.dbType || 'postgres',
            version: 'remote',
            rowCount: 0,
            size: '-',
            columns: t.columns.map(c => ({
              name: c.name,
              type: (c as any).dataType || (c as any).data_type || 'UNKNOWN',
              nullable: (c as any).nullable ?? true,
              isPrimary: (c as any).isPrimaryKey ?? false,
              isForeign: (c as any).isForeignKey ?? false,
              default: (c as any).defaultValue || (c as any).column_default || undefined,
            })),
          }));
          setTables(mapped);
          if (mapped.length > 0) setSelectedTable(mapped[0].name);
          setLoading(false);
        })
        .catch(err => {
          setError(err instanceof Error ? err.message : 'Failed to load remote schema');
          setLoading(false);
        });
      return;
    }

    api.getSchema(selectedDb)
      .then(res => {
        const mapped: TableInfo[] = res.map(t => ({
          name: t.name,
          rowCount: 0,
          size: '-',
          columns: t.columns.map(c => ({
            name: c.name,
            type: c.data_type,
            nullable: c.nullable,
            isPrimary: false,
            isForeign: false,
          })),
        }));
        setTables(mapped);
        if (mapped.length > 0) setSelectedTable(mapped[0].name);
        setLoading(false);
      })
      .catch(err => {
        setError(err instanceof Error ? err.message : 'Failed to load schema');
        setLoading(false);
      });
  }, [selectedDb]);

  const selectedTableInfo = tables.find(t => t.name === selectedTable);

  return (
    <div className="flex h-full">
      <div className="w-64 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <select
            className="input w-full text-sm mb-3"
            value={selectedDb}
            onChange={e => setSelectedDb(e.target.value)}
          >
            {runningDbs.map(db => (
              <option key={db.id} value={db.id}>
                {db.isRemote ? `${db.name} 🔗` : `${db.name} (${db.type})`}
              </option>
            ))}
            {runningDbs.length === 0 && <option>No running databases</option>}
          </select>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Tables</h3>
          <p className="text-xs mt-1" style={{ color: 'var(--textMuted)' }}>{tables.length} tables</p>
        </div>
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {loading && (
            <div className="text-center py-4 text-xs" style={{ color: 'var(--textMuted)' }}>Loading...</div>
          )}
          {error && (
            <div className="p-2 text-xs" style={{ color: 'var(--accentError)' }}>
              <AlertCircle size={12} className="inline mr-1" />
              {error}
            </div>
          )}
          {tables.map((table) => (
            <button key={table.name} onClick={() => setSelectedTable(table.name)} className="w-full text-left p-3 rounded-xl text-sm transition-all"
              style={{ backgroundColor: selectedTable === table.name ? 'var(--surfaceActive)' : 'transparent', color: selectedTable === table.name ? 'var(--accentPrimary)' : 'var(--textSecondary)' }}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Table2 size={16} />
                  <span className="font-medium">{table.name}</span>
                </div>
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{(table.rowCount ?? 0).toLocaleString()}</span>
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-auto p-8">
        {selectedTableInfo && selectedTableInfo.columns && (
          <div>
            <div className="flex items-center justify-between mb-6">
              <div>
                <h1 className="text-2xl font-bold" style={{ color: 'var(--textPrimary)' }}>{selectedTableInfo.name}</h1>
                <div className="flex items-center gap-4 mt-2">
                  <span className="text-sm" style={{ color: 'var(--textSecondary)' }}>{selectedTableInfo.columns?.length || 0} columns</span>
                </div>
              </div>
              <button className="btn-secondary px-4 py-2 rounded-xl">View Data</button>
            </div>

            <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
              <table className="w-full">
                <thead>
                  <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                    <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Column</th>
                    <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                    <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Nullable</th>
                    <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Default</th>
                    <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Keys</th>
                  </tr>
                </thead>
                <tbody>
                  {selectedTableInfo.columns?.map((column, index) => (
                    <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <Columns size={14} style={{ color: 'var(--textMuted)' }} />
                          <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{column.name}</span>
                        </div>
                      </td>
                      <td className="px-4 py-3">
                        <span className="text-xs px-2 py-1 rounded-full font-mono" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentSecondary)' }}>{column.type || 'UNKNOWN'}</span>
                      </td>
                      <td className="px-4 py-3">
                        <span className="text-sm" style={{ color: column.nullable ? 'var(--accentWarning)' : 'var(--accentSuccess)' }}>{column.nullable ? 'YES' : 'NO'}</span>
                      </td>
                      <td className="px-4 py-3">
                        <span className="text-sm font-mono" style={{ color: 'var(--textMuted)' }}>{column.default || '-'}</span>
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          {column.isPrimary && (
                            <span className="flex items-center gap-1 text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'rgba(0,212,170,0.1)', color: 'var(--accentPrimary)' }}>
                              <Key size={10} /> PK
                            </span>
                          )}
                          {column.isForeign && (
                            <span className="flex items-center gap-1 text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'rgba(107,138,255,0.1)', color: 'var(--accentSecondary)' }}>
                              <Link2 size={10} /> FK
                            </span>
                          )}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
