import { useState, useEffect, useMemo } from 'react';
import {
  Search, Filter, ArrowUpDown, Plus, Pencil, Trash2, X, ChevronLeft, ChevronRight,
  Database, Table2, RefreshCw, Save, AlertCircle
} from 'lucide-react';
import { useDatabaseStore } from '../stores/databaseStore';
import { api } from '../services/api';

export function DataPage() {
  const {
    databases,
    selectedDatabase,
    selectedTable,
    tableData,
    tableDataLoading,
    editingRow,
    selectDatabase,
    selectTable,
    setEditingRow,
    clearEditingRow,
    fetchTableData,
    updateRow,
    deleteRow,
    error,
    clearError,
  } = useDatabaseStore();

  const runningDbs = databases.filter(d => d.status === 'running');
  const [tables, setTables] = useState<{ name: string; columns: { name: string; data_type: string; nullable: boolean }[] }[]>([]);
  const [tablesLoading, setTablesLoading] = useState(false);

  // Pagination & filtering
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(50);
  const [searchFilter, setSearchFilter] = useState('');
  const [sortColumn, setSortColumn] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<'ASC' | 'DESC'>('ASC');

  // Edit form state
  const [editForm, setEditForm] = useState<Record<string, any>>({});

  // Detect primary key column (first column, usually 'id')
  const primaryKeyColumn = useMemo(() => {
    if (!selectedTable || !tables.length) return 'id';
    const table = tables.find(t => t.name === selectedTable);
    if (!table) return 'id';
    // Try to find a column named 'id' or ending in '_id'
    const pk = table.columns.find(c => c.name === 'id' || c.name.endsWith('_id'));
    return pk?.name || table.columns[0]?.name || 'id';
  }, [selectedTable, tables]);

  // Load tables when DB changes
  useEffect(() => {
    if (!selectedDatabase && runningDbs.length > 0) {
      selectDatabase(runningDbs[0]);
    }
  }, [runningDbs, selectedDatabase]);

  useEffect(() => {
    if (!selectedDatabase) {
      setTables([]);
      return;
    }
    setTablesLoading(true);
    api.getSchema(selectedDatabase.id)
      .then(schema => setTables(schema))
      .catch(() => setTables([]))
      .finally(() => setTablesLoading(false));
  }, [selectedDatabase]);

  // Load table data
  useEffect(() => {
    if (!selectedDatabase || !selectedTable) return;
    setPage(1);
    loadData();
  }, [selectedDatabase, selectedTable]);

  const loadData = () => {
    if (!selectedDatabase || !selectedTable) return;
    fetchTableData(selectedDatabase.id, selectedTable, {
      limit: pageSize,
      offset: (page - 1) * pageSize,
      order_by: sortColumn || undefined,
      order_dir: sortDir,
      filter: searchFilter || undefined,
    });
  };

  useEffect(() => {
    if (selectedDatabase && selectedTable) {
      loadData();
    }
  }, [page, pageSize, sortColumn, sortDir]);

  const handleSort = (col: string) => {
    if (sortColumn === col) {
      setSortDir(d => d === 'ASC' ? 'DESC' : 'ASC');
    } else {
      setSortColumn(col);
      setSortDir('ASC');
    }
  };

  const handleRowClick = (row: any[]) => {
    if (!tableData) return;
    const rowData: Record<string, any> = {};
    tableData.columns.forEach((col, i) => {
      rowData[col] = row[i];
    });
    setEditForm({ ...rowData });
    setEditingRow(rowData);
  };

  const handleSaveEdit = async () => {
    if (!selectedDatabase || !selectedTable || !editingRow) return;
    const pk = editingRow[primaryKeyColumn];
    const data = { ...editForm };
    delete data[primaryKeyColumn]; // Don't update PK
    await updateRow(selectedDatabase.id, selectedTable, pk, primaryKeyColumn, data);
    clearEditingRow();
    setEditForm({});
    loadData();
  };

  const handleDeleteRow = async () => {
    if (!selectedDatabase || !selectedTable || !editingRow) return;
    if (!confirm('Are you sure you want to delete this row?')) return;
    const pk = editingRow[primaryKeyColumn];
    await deleteRow(selectedDatabase.id, selectedTable, pk, primaryKeyColumn);
    clearEditingRow();
    setEditForm({});
    loadData();
  };

  const totalPages = tableData ? Math.ceil(tableData.total_count / pageSize) : 0;

  return (
    <div className="flex h-full">
      {/* Left sidebar: Database selector + Table list */}
      <div className="w-64 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <select
            className="input w-full text-sm mb-3"
            value={selectedDatabase?.id || ''}
            onChange={e => {
              const db = runningDbs.find(d => d.id === e.target.value);
              if (db) selectDatabase(db);
            }}
          >
            {runningDbs.map(db => (
              <option key={db.id} value={db.id}>{db.name} ({db.type})</option>
            ))}
            {runningDbs.length === 0 && <option>No running databases</option>}
          </select>
          <div className="flex items-center gap-2">
            <Database size={16} style={{ color: 'var(--accentPrimary)' }} />
            <span className="text-sm font-semibold" style={{ color: 'var(--textPrimary)' }}>Tables</span>
          </div>
        </div>

        <div className="flex-1 overflow-auto px-2 py-2 space-y-1">
          {tablesLoading && <div className="text-center py-4 text-xs" style={{ color: 'var(--textMuted)' }}>Loading...</div>}
          {tables.map(table => (
            <button
              key={table.name}
              onClick={() => selectTable(table.name)}
              className="w-full text-left p-3 rounded-xl text-sm transition-all"
              style={{
                backgroundColor: selectedTable === table.name ? 'var(--surfaceActive)' : 'transparent',
                color: selectedTable === table.name ? 'var(--accentPrimary)' : 'var(--textSecondary)',
                borderRight: selectedTable === table.name ? '3px solid var(--accentPrimary)' : '3px solid transparent',
              }}
            >
              <div className="flex items-center gap-2">
                <Table2 size={16} />
                <span className="font-medium">{table.name}</span>
              </div>
              <div className="text-xs mt-1" style={{ color: 'var(--textMuted)' }}>{table.columns.length} columns</div>
            </button>
          ))}
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Toolbar */}
        <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-3">
            <h2 className="text-lg font-bold" style={{ color: 'var(--textPrimary)' }}>
              {selectedTable || 'Select a table'}
            </h2>
            {tableData && (
              <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                {tableData.total_count.toLocaleString()} rows
              </span>
            )}
          </div>

          <div className="flex items-center gap-2">
            <div className="relative">
              <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2" style={{ color: 'var(--textMuted)' }} />
              <input
                type="text"
                value={searchFilter}
                onChange={e => setSearchFilter(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && loadData()}
                placeholder="Filter rows..."
                className="input pl-9 text-sm"
                style={{ width: 200 }}
              />
            </div>
            <button onClick={loadData} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl text-sm">
              <RefreshCw size={14} /> Refresh
            </button>
            <button className="btn-primary flex items-center gap-2 px-3 py-2 rounded-xl text-sm">
              <Plus size={14} /> Add Row
            </button>
          </div>
        </div>

        {/* Error */}
        {error && (
          <div className="p-3 border-b flex items-center gap-2" style={{ borderColor: 'var(--accentError)', backgroundColor: 'rgba(255,68,68,0.05)' }}>
            <AlertCircle size={14} style={{ color: 'var(--accentError)' }} />
            <span className="text-sm" style={{ color: 'var(--accentError)' }}>{error}</span>
            <button onClick={clearError} className="ml-auto"><X size={14} /></button>
          </div>
        )}

        {/* Data grid */}
        <div className="flex-1 overflow-auto">
          {tableDataLoading ? (
            <div className="flex items-center justify-center h-full">
              <RefreshCw size={24} className="animate-spin" style={{ color: 'var(--textMuted)' }} />
            </div>
          ) : !tableData || tableData.rows.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-3">
              <Table2 size={48} style={{ color: 'var(--textMuted)', opacity: 0.3 }} />
              <p className="text-sm" style={{ color: 'var(--textMuted)' }}>
                {selectedTable ? 'No rows found' : 'Select a table to view data'}
              </p>
            </div>
          ) : (
            <table className="w-full">
              <thead className="sticky top-0 z-10">
                <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  {tableData.columns.map(col => (
                    <th
                      key={col}
                      onClick={() => handleSort(col)}
                      className="text-left px-4 py-3 text-xs font-semibold uppercase cursor-pointer select-none transition-colors hover:opacity-80"
                      style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}
                    >
                      <div className="flex items-center gap-1">
                        {col}
                        {sortColumn === col && (
                          <ArrowUpDown size={12} style={{ color: 'var(--accentPrimary)' }} />
                        )}
                      </div>
                    </th>
                  ))}
                  <th className="w-16" style={{ borderBottom: '1px solid var(--borderDefault)' }} />
                </tr>
              </thead>
              <tbody>
                {tableData.rows.map((row, rowIndex) => (
                  <tr
                    key={rowIndex}
                    onClick={() => handleRowClick(row)}
                    className="cursor-pointer transition-all hover:opacity-80"
                    style={{
                      backgroundColor: rowIndex % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)',
                      borderBottom: '1px solid var(--borderDefault)',
                    }}
                  >
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex} className="px-4 py-2 text-sm font-mono truncate max-w-xs" style={{ color: 'var(--textPrimary)' }}>
                        {cell === null ? (
                          <span className="text-xs italic" style={{ color: 'var(--textMuted)' }}>NULL</span>
                        ) : typeof cell === 'boolean' ? (
                          cell ? 'true' : 'false'
                        ) : (
                          String(cell).substring(0, 100)
                        )}
                      </td>
                    ))}
                    <td className="px-2">
                      <Pencil size={14} style={{ color: 'var(--textMuted)' }} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Pagination */}
        {tableData && tableData.total_count > 0 && (
          <div className="flex items-center justify-between p-3 border-t" style={{ borderColor: 'var(--borderDefault)' }}>
            <div className="flex items-center gap-2">
              <span className="text-xs" style={{ color: 'var(--textMuted)' }}>
                Showing {((page - 1) * pageSize) + 1} - {Math.min(page * pageSize, tableData.total_count)} of {tableData.total_count}
              </span>
              <select
                value={pageSize}
                onChange={e => { setPageSize(Number(e.target.value)); setPage(1); }}
                className="input text-xs px-2 py-1"
              >
                <option value={25}>25</option>
                <option value={50}>50</option>
                <option value={100}>100</option>
                <option value={250}>250</option>
              </select>
            </div>
            <div className="flex items-center gap-1">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page <= 1}
                className="p-2 rounded-lg transition-all disabled:opacity-30"
                style={{ backgroundColor: 'var(--bgSecondary)' }}
              >
                <ChevronLeft size={16} />
              </button>
              <span className="text-sm px-3" style={{ color: 'var(--textSecondary)' }}>
                Page {page} of {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
                className="p-2 rounded-lg transition-all disabled:opacity-30"
                style={{ backgroundColor: 'var(--bgSecondary)' }}
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Slide-over edit panel */}
      {editingRow && (
        <>
          <div
            className="fixed inset-0 z-40"
            style={{ backgroundColor: 'rgba(0,0,0,0.3)' }}
            onClick={() => { clearEditingRow(); setEditForm({}); }}
          />
          <div
            className="fixed right-0 top-0 bottom-0 w-96 z-50 flex flex-col border-l shadow-2xl"
            style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}
          >
            <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
              <h3 className="text-lg font-bold" style={{ color: 'var(--textPrimary)' }}>Edit Row</h3>
              <button
                onClick={() => { clearEditingRow(); setEditForm({}); }}
                className="p-2 rounded-lg hover:opacity-80"
                style={{ backgroundColor: 'var(--bgTertiary)' }}
              >
                <X size={18} />
              </button>
            </div>

            <div className="flex-1 overflow-auto p-4 space-y-4">
              {tableData?.columns.map(col => {
                const isPk = col === primaryKeyColumn;
                const value = editForm[col] ?? editingRow[col] ?? '';
                return (
                  <div key={col}>
                    <label className="block text-xs font-semibold uppercase mb-1" style={{ color: 'var(--textSecondary)' }}>
                      {col}
                      {isPk && <span className="ml-2 text-xs px-1.5 py-0.5 rounded" style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}>PK</span>}
                    </label>
                    {isPk ? (
                      <div className="p-3 rounded-xl text-sm font-mono" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                        {String(value)}
                      </div>
                    ) : (
                      <input
                        type="text"
                        value={value === null ? '' : String(value)}
                        onChange={e => setEditForm(f => ({ ...f, [col]: e.target.value }))}
                        className="input w-full text-sm"
                        placeholder="NULL"
                      />
                    )}
                  </div>
                );
              })}
            </div>

            <div className="p-4 border-t space-y-2" style={{ borderColor: 'var(--borderDefault)' }}>
              <button
                onClick={handleSaveEdit}
                className="w-full btn-primary flex items-center justify-center gap-2 px-4 py-3 rounded-xl text-sm font-medium"
              >
                <Save size={16} /> Save Changes
              </button>
              <button
                onClick={handleDeleteRow}
                className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl text-sm font-medium"
                style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)' }}
              >
                <Trash2 size={16} /> Delete Row
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
