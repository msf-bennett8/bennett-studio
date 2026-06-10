import { useState } from 'react';
import { Table2, Columns, Key, Link2 } from 'lucide-react';

interface ColumnInfo {
  name: string; type: string; nullable: boolean; default?: string;
  isPrimary: boolean; isForeign: boolean; references?: string;
}

interface TableInfo {
  name: string; columns: ColumnInfo[]; rowCount: number; size: string;
}

const mockTables: TableInfo[] = [
  {
    name: 'users', rowCount: 15420, size: '2.4 MB',
    columns: [
      { name: 'id', type: 'SERIAL', nullable: false, isPrimary: true, isForeign: false },
      { name: 'email', type: 'VARCHAR(255)', nullable: false, isPrimary: false, isForeign: false },
      { name: 'name', type: 'VARCHAR(100)', nullable: true, isPrimary: false, isForeign: false },
      { name: 'created_at', type: 'TIMESTAMP', nullable: false, default: 'NOW()', isPrimary: false, isForeign: false },
      { name: 'status', type: 'ENUM', nullable: false, default: 'active', isPrimary: false, isForeign: false },
    ],
  },
  {
    name: 'orders', rowCount: 8934, size: '1.8 MB',
    columns: [
      { name: 'id', type: 'SERIAL', nullable: false, isPrimary: true, isForeign: false },
      { name: 'user_id', type: 'INTEGER', nullable: false, isPrimary: false, isForeign: true, references: 'users.id' },
      { name: 'total', type: 'DECIMAL(10,2)', nullable: false, isPrimary: false, isForeign: false },
      { name: 'status', type: 'VARCHAR(50)', nullable: false, default: 'pending', isPrimary: false, isForeign: false },
      { name: 'created_at', type: 'TIMESTAMP', nullable: false, default: 'NOW()', isPrimary: false, isForeign: false },
    ],
  },
  {
    name: 'products', rowCount: 342, size: '456 KB',
    columns: [
      { name: 'id', type: 'SERIAL', nullable: false, isPrimary: true, isForeign: false },
      { name: 'name', type: 'VARCHAR(200)', nullable: false, isPrimary: false, isForeign: false },
      { name: 'price', type: 'DECIMAL(10,2)', nullable: false, isPrimary: false, isForeign: false },
      { name: 'stock', type: 'INTEGER', nullable: false, default: '0', isPrimary: false, isForeign: false },
      { name: 'category_id', type: 'INTEGER', nullable: true, isPrimary: false, isForeign: true, references: 'categories.id' },
    ],
  },
];

export function SchemaPage() {
  const [selectedTable, setSelectedTable] = useState<string>('users');
  const selectedTableInfo = mockTables.find(t => t.name === selectedTable);

  return (
    <div className="flex h-full">
      <div className="w-64 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Tables</h3>
          <p className="text-xs mt-1" style={{ color: 'var(--textMuted)' }}>{mockTables.length} tables</p>
        </div>
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {mockTables.map((table) => (
            <button key={table.name} onClick={() => setSelectedTable(table.name)} className="w-full text-left p-3 rounded-xl text-sm transition-all"
              style={{ backgroundColor: selectedTable === table.name ? 'var(--surfaceActive)' : 'transparent', color: selectedTable === table.name ? 'var(--accentPrimary)' : 'var(--textSecondary)' }}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Table2 size={16} />
                  <span className="font-medium">{table.name}</span>
                </div>
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.rowCount.toLocaleString()}</span>
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-auto p-8">
        {selectedTableInfo && (
          <div>
            <div className="flex items-center justify-between mb-6">
              <div>
                <h1 className="text-2xl font-bold" style={{ color: 'var(--textPrimary)' }}>{selectedTableInfo.name}</h1>
                <div className="flex items-center gap-4 mt-2">
                  <span className="text-sm" style={{ color: 'var(--textSecondary)' }}>{selectedTableInfo.rowCount.toLocaleString()} rows</span>
                  <span className="text-sm" style={{ color: 'var(--textMuted)' }}>{selectedTableInfo.size}</span>
                  <span className="text-sm" style={{ color: 'var(--textMuted)' }}>{selectedTableInfo.columns.length} columns</span>
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
                  {selectedTableInfo.columns.map((column, index) => (
                    <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <Columns size={14} style={{ color: 'var(--textMuted)' }} />
                          <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{column.name}</span>
                        </div>
                      </td>
                      <td className="px-4 py-3">
                        <span className="text-xs px-2 py-1 rounded-full font-mono" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentSecondary)' }}>{column.type}</span>
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

