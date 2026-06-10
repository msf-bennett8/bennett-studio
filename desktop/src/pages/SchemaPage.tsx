import { useState, useMemo, useEffect } from 'react';
import { Table2, Columns, Key, Link2, Search, Database, Hash, Filter, ArrowRight } from 'lucide-react';
import { dataService } from '../services/dataService';
import type { TableSchema, ColumnSchema } from '../services/dataService';

export function SchemaPage() {
  const [selectedTable, setSelectedTable] = useState<string>('users');
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTab, setActiveTab] = useState<'columns' | 'indexes' | 'constraints' | 'triggers' | 'relations'>('columns');

  const [tables, setTables] = useState([]);
  const [metadata, setMetadata] = useState(null);
  const [selectedTableData, setSelectedTableData] = useState(null);
  const [relatedTables, setRelatedTables] = useState({ hasMany: [], belongsTo: [] });
  const [columnStats, setColumnStats] = useState(null);

  useEffect(() => {
    dataService.getAllTables().then(setTables);
    dataService.getMetadata().then(setMetadata);
  }, []);

  useEffect(() => {
    if (selectedTable) {
      dataService.getTable(selectedTable).then(table => {
        setSelectedTableData(table || null);
        if (table) {
          dataService.getRelatedTables(selectedTable).then(setRelatedTables);
          dataService.getColumnStats(selectedTable).then(setColumnStats);
        }
      });
    }
  }, [selectedTable]);

  const filteredTables = useMemo(() => {
    if (!searchQuery) return tables;
    return tables.filter(t => 
      t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.columns.some(c => c.name.toLowerCase().includes(searchQuery.toLowerCase()))
    );
  }, [searchQuery, tables]);

  const getTypeColor = (type: string) => {
    if (type.includes('SERIAL') || type.includes('INTEGER')) return 'var(--accentInfo)';
    if (type.includes('VARCHAR') || type.includes('TEXT')) return 'var(--accentSecondary)';
    if (type.includes('DECIMAL') || type.includes('NUMERIC')) return 'var(--accentWarning)';
    if (type.includes('TIMESTAMP') || type.includes('DATE')) return 'var(--accentSuccess)';
    if (type.includes('BOOLEAN')) return 'var(--accentPrimary)';
    if (type.includes('JSON')) return 'var(--accentError)';
    if (type.includes('ENUM')) return 'var(--accentInfo)';
    return 'var(--textMuted)';
  };

  const getConstraintBadges = (column: ColumnSchema) => {
    const badges = [];
    if (column.is_primary) badges.push({ label: 'PK', color: 'var(--accentPrimary)', icon: Key });
    if (column.is_foreign) badges.push({ label: 'FK', color: 'var(--accentSecondary)', icon: Link2 });
    if (column.constraints?.includes('UNIQUE')) badges.push({ label: 'UQ', color: 'var(--accentWarning)', icon: Hash });
    if (!column.nullable) badges.push({ label: 'NN', color: 'var(--accentError)', icon: Filter });
    return badges;
  };

  return (
    <div className="flex h-full">
      <div className="w-72 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-2 mb-2">
            <Database size={16} style={{ color: 'var(--accentPrimary)' }} />
            <span className="text-sm font-semibold" style={{ color: 'var(--textPrimary)' }}>{metadata?.database_name}</span>
          </div>
          <div className="flex items-center gap-2 text-xs" style={{ color: 'var(--textMuted)' }}>
            <span>{metadata?.engine} {metadata?.version}</span>
            <span>•</span>
            <span>{metadata?.total_tables} tables</span>
          </div>
        </div>
        <div className="p-3">
          <div className="relative">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2" style={{ color: 'var(--textMuted)' }} />
            <input type="text" value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} placeholder="Search tables, columns..." className="input pl-9 text-sm" />
          </div>
        </div>
        <div className="flex-1 overflow-auto px-2 pb-2 space-y-1">
          {filteredTables.map((table) => (
            <button key={table.name} onClick={() => { setSelectedTable(table.name); setActiveTab('columns'); }}
              className="w-full text-left p-3 rounded-xl text-sm transition-all"
              style={{ backgroundColor: selectedTable === table.name ? 'var(--surfaceActive)' : 'transparent', color: selectedTable === table.name ? 'var(--accentPrimary)' : 'var(--textSecondary)', borderRight: selectedTable === table.name ? '3px solid var(--accentPrimary)' : '3px solid transparent' }}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Table2 size={16} />
                  <span className="font-medium">{table.name}</span>
                </div>
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table?.row_count?.toLocaleString() ?? 0}</span>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table?.columns?.length ?? 0} cols</span>
                <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table.size}</span>
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {selectedTableData && (
          <div className="p-8">
            <div className="flex items-start justify-between mb-6">
              <div>
                <div className="flex items-center gap-3 mb-2">
                  <h1 className="text-2xl font-bold" style={{ color: 'var(--textPrimary)' }}>{selectedTableData.name}</h1>
                  <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{selectedTableData.engine} {selectedTableData.version}</span>
                </div>
                <p className="text-sm" style={{ color: 'var(--textSecondary)' }}>{selectedTableData?.row_count?.toLocaleString() ?? 0} rows • {selectedTableData?.size ?? '-'} • {selectedTableData?.columns?.length ?? 0} columns</p>
              </div>
              <div className="flex gap-2">
                <button className="btn-secondary px-4 py-2 rounded-xl text-sm">View Data</button>
                <button className="btn-primary px-4 py-2 rounded-xl text-sm">Export Schema</button>
              </div>
            </div>

            {columnStats && (
              <div className="grid grid-cols-5 gap-3 mb-6">
                {[
                  { label: 'Total', value: columnStats.total, color: 'var(--accentPrimary)' },
                  { label: 'Nullable', value: columnStats.nullable, color: 'var(--accentWarning)' },
                  { label: 'Primary Keys', value: columnStats.primary, color: 'var(--accentSuccess)' },
                  { label: 'Foreign Keys', value: columnStats.foreign, color: 'var(--accentSecondary)' },
                  { label: 'With Default', value: columnStats.withDefault, color: 'var(--accentInfo)' },
                ].map((stat, i) => (
                  <div key={i} className="p-3 rounded-xl text-center" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                    <div className="text-xl font-bold" style={{ color: stat.color }}>{stat.value}</div>
                    <div className="text-xs" style={{ color: 'var(--textMuted)' }}>{stat.label}</div>
                  </div>
                ))}
              </div>
            )}

            <div className="flex gap-1 mb-4 p-1 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              {[
                { id: 'columns', label: 'Columns', count: selectedTableData?.columns?.length ?? 0 },
                { id: 'indexes', label: 'Indexes', count: selectedTableData?.indexes?.length ?? 0 },
                { id: 'constraints', label: 'Constraints', count: selectedTableData?.constraints?.length ?? 0 },
                { id: 'triggers', label: 'Triggers', count: selectedTableData?.triggers?.length ?? 0 },
                { id: 'relations', label: 'Relations', count: (relatedTables?.hasMany?.length ?? 0) + (relatedTables?.belongsTo?.length ?? 0) },
              ].map((tab) => (
                <button key={tab.id} onClick={() => setActiveTab(tab.id as any)} className="flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-all"
                  style={{ backgroundColor: activeTab === tab.id ? 'var(--surfaceActive)' : 'transparent', color: activeTab === tab.id ? 'var(--accentPrimary)' : 'var(--textSecondary)' }}>
                  {tab.label} <span className="text-xs" style={{ color: 'var(--textMuted)' }}>({tab.count})</span>
                </button>
              ))}
            </div>

            {activeTab === 'columns' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Column</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Nullable</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Default</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Constraints</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.columns.map((column, index) => {
                      const badges = getConstraintBadges(column);
                      return (
                        <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                          <td className="px-4 py-3">
                            <div className="flex items-center gap-2">
                              <Columns size={14} style={{ color: 'var(--textMuted)' }} />
                              <span className="text-sm font-medium font-mono" style={{ color: 'var(--textPrimary)' }}>{column.name}</span>
                            </div>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-xs px-2 py-1 rounded-full font-mono" style={{ backgroundColor: 'var(--bgTertiary)', color: getTypeColor(column.type) }}>{column.type}</span>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-sm" style={{ color: column.nullable ? 'var(--accentWarning)' : 'var(--accentSuccess)' }}>{column.nullable ? 'YES' : 'NO'}</span>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-sm font-mono" style={{ color: 'var(--textMuted)' }}>{column.default || '-'}</span>
                          </td>
                          <td className="px-4 py-3">
                            <div className="flex items-center gap-1">
                              {badges.map((badge, bi) => (
                                <span key={bi} className="flex items-center gap-1 text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: `${badge.color}20`, color: badge.color }}>
                                  <badge.icon size={10} /> {badge.label}
                                </span>
                              ))}
                              {badges.length === 0 && <span className="text-xs" style={{ color: 'var(--textMuted)' }}>-</span>}
                            </div>
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-xs" style={{ color: 'var(--textSecondary)' }}>{column.description}</span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'indexes' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Name</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Columns</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Unique</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.indexes.map((idx, index) => (
                      <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{idx.name}</td>
                        <td className="px-4 py-3">
                          <div className="flex gap-1">
                            {idx.columns.map((col, ci) => (
                              <span key={ci} className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{col}</span>
                            ))}
                          </div>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentInfo)' }}>{idx.type}</span>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-sm" style={{ color: idx.unique ? 'var(--accentSuccess)' : 'var(--textMuted)' }}>{idx.unique ? 'Yes' : 'No'}</span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'constraints' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Name</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Type</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Columns</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>References</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.constraints.map((constraint, index) => (
                      <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{constraint.name}</td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentWarning)' }}>{constraint.type}</span>
                        </td>
                        <td className="px-4 py-3">
                          <div className="flex gap-1">
                            {constraint.columns.map((col, ci) => (
                              <span key={ci} className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{col}</span>
                            ))}
                          </div>
                        </td>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textMuted)' }}>{constraint.references || constraint.definition || '-'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'triggers' && (
              <div className="rounded-xl overflow-hidden" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                <table className="w-full">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Name</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Event</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Timing</th>
                      <th className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)' }}>Definition</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedTableData.triggers.map((trigger, index) => (
                      <tr key={index} style={{ backgroundColor: index % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        <td className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)' }}>{trigger.name}</td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentError)' }}>{trigger.event}</span>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--accentInfo)' }}>{trigger.timing}</span>
                        </td>
                        <td className="px-4 py-3">
                          <code className="text-xs font-mono px-2 py-1 rounded" style={{ backgroundColor: 'var(--bgSecondary)', color: 'var(--textSecondary)' }}>{trigger.definition}</code>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {activeTab === 'relations' && (
              <div className="space-y-4">
                {relatedTables.belongsTo.length > 0 && (
                  <div className="p-4 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                    <h3 className="text-sm font-semibold mb-3" style={{ color: 'var(--textSecondary)' }}>Belongs To</h3>
                    <div className="space-y-2">
                      {relatedTables.belongsTo.map((table) => (
                        <button key={table.name} onClick={() => setSelectedTable(table.name)} className="w-full flex items-center justify-between p-3 rounded-xl text-left transition-all hover:opacity-80" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                          <div className="flex items-center gap-3">
                            <Table2 size={16} style={{ color: 'var(--accentSecondary)' }} />
                            <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                            <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table?.row_count?.toLocaleString() ?? 0} rows</span>
                          </div>
                          <ArrowRight size={14} style={{ color: 'var(--textMuted)' }} />
                        </button>
                      ))}
                    </div>
                  </div>
                )}
                {relatedTables.hasMany.length > 0 && (
                  <div className="p-4 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
                    <h3 className="text-sm font-semibold mb-3" style={{ color: 'var(--textSecondary)' }}>Has Many</h3>
                    <div className="space-y-2">
                      {relatedTables.hasMany.map((table) => (
                        <button key={table.name} onClick={() => setSelectedTable(table.name)} className="w-full flex items-center justify-between p-3 rounded-xl text-left transition-all hover:opacity-80" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                          <div className="flex items-center gap-3">
                            <Table2 size={16} style={{ color: 'var(--accentPrimary)' }} />
                            <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                            <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{table?.row_count?.toLocaleString() ?? 0} rows</span>
                          </div>
                          <ArrowRight size={14} style={{ color: 'var(--textMuted)' }} />
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

