import { useState, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Play, Copy, Check, Download, Clock, AlertCircle,
  Globe, Lock, Unlock, RefreshCw, X, Loader2, Database,
  Table2, Columns, TreePine, History, FileText, Share2
} from 'lucide-react';
import { useRemoteConnectionStore } from '../stores/remoteConnectionStore';
import { useDatabaseStore } from '../stores/databaseStore';
import type { AutocompleteSuggestion } from '@bennett/shared';

export function RemoteQueryPage() {
  const navigate = useNavigate();
  const {
    connections,
    activeConnectionId,
    currentSql,
    queryResult,
    isExecuting,
    queryError,
    schema,
    schemaLoading,
    schemaError,
    setCurrentSql,
    executeQuery,
    executeWrite,
    refreshSchema,
    setActiveConnection,
    getAutocomplete,
    getQueryHistory,
    exportResults,
    clearError,
  } = useRemoteConnectionStore();

  const [copied, setCopied] = useState(false);
  const [showSchemaPanel, setShowSchemaPanel] = useState(true);
  const [showHistoryPanel, setShowHistoryPanel] = useState(false);
  const [selectedTable, setSelectedTable] = useState<string | null>(null);
  const [autocompleteOpen, setAutocompleteOpen] = useState(false);
  const [autocompleteSuggestions, setAutocompleteSuggestions] = useState<AutocompleteSuggestion[]>([]);
  const [autocompleteIndex, setAutocompleteIndex] = useState(0);
  const [cursorPosition, setCursorPosition] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const activeConnection = connections.find(c => c.id === activeConnectionId);

  // Sync activeConnection with databaseStore so other pages see the same DB
  useEffect(() => {
    if (activeConnection) {
      const { selectDatabase } = useDatabaseStore.getState();
      const remoteDb = {
        id: activeConnection.id,
        name: `${activeConnection.dbName || 'Remote Database'} ${activeConnection.code}`,
        type: (activeConnection.dbType || 'postgres') as 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis',
        version: '',
        status: 'running' as const,
        port: 0,
        size: '',
        created_at: activeConnection.connectedAt,
        source: 'bennett' as any,
        isRemote: true,
        shareCode: activeConnection.code,
        remotePermission: activeConnection.permission,
        remoteHost: activeConnection.baseUrl,
      };
      selectDatabase(remoteDb);
    }
  }, [activeConnection]);

  // Auto-refresh schema periodically
  useEffect(() => {
    if (!activeConnectionId) return;
    
    const interval = setInterval(() => {
      refreshSchema();
    }, 30000); // Refresh every 30s
    
    return () => clearInterval(interval);
  }, [activeConnectionId]);

  // Handle autocomplete
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value;
    const cursor = e.target.selectionStart || 0;
    setCurrentSql(value);
    setCursorPosition(cursor);

    // Extract word at cursor
    const beforeCursor = value.substring(0, cursor);
    const match = beforeCursor.match(/(\w+)$/);
    
    if (match && match[1].length >= 2) {
      const prefix = match[1];
      const suggestions = getAutocomplete(prefix);
      if (suggestions.length > 0) {
        setAutocompleteSuggestions(suggestions);
        setAutocompleteOpen(true);
        setAutocompleteIndex(0);
      } else {
        setAutocompleteOpen(false);
      }
    } else {
      setAutocompleteOpen(false);
    }
  };

  const handleAutocompleteSelect = (suggestion: AutocompleteSuggestion) => {
    const value = currentSql;
    const beforeCursor = value.substring(0, cursorPosition);
    const afterCursor = value.substring(cursorPosition);
    
    // Replace the partial word
    const replaced = beforeCursor.replace(/(\w+)$/, suggestion.insertText) + afterCursor;
    setCurrentSql(replaced);
    setAutocompleteOpen(false);
    
    // Focus back on textarea
    setTimeout(() => textareaRef.current?.focus(), 0);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === 'Enter') {
      e.preventDefault();
      handleExecute();
      return;
    }

    if (!autocompleteOpen) return;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setAutocompleteIndex(i => (i + 1) % autocompleteSuggestions.length);
        break;
      case 'ArrowUp':
        e.preventDefault();
        setAutocompleteIndex(i => (i - 1 + autocompleteSuggestions.length) % autocompleteSuggestions.length);
        break;
      case 'Enter':
      case 'Tab':
        e.preventDefault();
        handleAutocompleteSelect(autocompleteSuggestions[autocompleteIndex]);
        break;
      case 'Escape':
        setAutocompleteOpen(false);
        break;
    }
  };

  const handleExecute = async () => {
    if (!activeConnection || !currentSql.trim()) return;

    const trimmed = currentSql.trim().toUpperCase();
    const isWrite = trimmed.startsWith('INSERT') || trimmed.startsWith('UPDATE') || trimmed.startsWith('DELETE');

    if (isWrite) {
      if (activeConnection.permission === 'ro') {
        // Show error
        return;
      }
      await executeWrite(currentSql);
    } else {
      await executeQuery();
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(currentSql);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleExport = async (format: 'csv' | 'json') => {
    try {
      const data = await exportResults(format);
      const blob = new Blob([data], { type: format === 'csv' ? 'text/csv' : 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `query-results.${format}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      // Error handled in store
    }
  };

  const queryHistory = activeConnection ? getQueryHistory() : [];

  if (!activeConnection) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <Globe size={48} className="mx-auto mb-4" style={{ color: 'var(--textMuted)' }} />
          <h2 className="text-xl font-bold mb-2" style={{ color: 'var(--textPrimary)' }}>No Active Connection</h2>
          <p className="mb-4" style={{ color: 'var(--textSecondary)' }}>Connect to a shared database to start querying</p>
          <button onClick={() => navigate('/join-share')} className="btn-primary px-6 py-3 rounded-xl">
            <Globe size={18} className="inline mr-2" /> Join Share
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Schema Panel */}
      {showSchemaPanel && (
        <div className="w-72 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
          <div className="p-4 border-b flex items-center justify-between" style={{ borderColor: 'var(--borderDefault)' }}>
            <div className="flex items-center gap-2">
              <TreePine size={16} style={{ color: 'var(--accentPrimary)' }} />
              <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Schema</h3>
            </div>
            <button onClick={() => refreshSchema()} disabled={schemaLoading} className="p-1 rounded-lg hover:opacity-80">
              <RefreshCw size={14} className={schemaLoading ? 'animate-spin' : ''} style={{ color: 'var(--textMuted)' }} />
            </button>
          </div>
          
          {schemaError && (
            <div className="p-2 text-xs" style={{ color: 'var(--accentError)' }}>
              <AlertCircle size={12} className="inline mr-1" />
              {schemaError}
            </div>
          )}

          <div className="flex-1 overflow-auto p-2 space-y-1">
            {schemaLoading && !schema && (
              <div className="text-center py-4 text-xs" style={{ color: 'var(--textMuted)' }}>
                <Loader2 size={16} className="animate-spin mx-auto mb-2" />
                Loading schema...
              </div>
            )}
            
            {schema?.map(table => (
              <div key={table.name}>
                <button
                  onClick={() => setSelectedTable(selectedTable === table.name ? null : table.name)}
                  className="w-full text-left p-2 rounded-lg text-sm transition-all flex items-center gap-2"
                  style={{
                    backgroundColor: selectedTable === table.name ? 'var(--surfaceActive)' : 'transparent',
                    color: selectedTable === table.name ? 'var(--accentPrimary)' : 'var(--textSecondary)',
                  }}
                >
                  <Table2 size={14} />
                  <span className="font-medium">{table.name}</span>
                  <span className="text-xs ml-auto" style={{ color: 'var(--textMuted)' }}>{table.columns.length}</span>
                </button>
                
                {selectedTable === table.name && (
                  <div className="ml-4 mt-1 space-y-0.5">
                    {table.columns.map(col => (
                      <button
                        key={col.name}
                        onClick={() => setCurrentSql(`SELECT * FROM "${table.name}" WHERE "${col.name}" = `)}
                        className="w-full text-left p-1.5 rounded-lg text-xs transition-all flex items-center gap-2"
                        style={{ color: 'var(--textMuted)' }}
                      >
                        <Columns size={10} />
                        <span>{col.name}</span>
                        <span className="text-xs" style={{ color: 'var(--textMuted)', opacity: 0.7 }}>{(col as any).dataType || (col as any).data_type}</span>
                        {(col as any).isPrimaryKey && (
                          <span className="text-xs px-1 py-0.5 rounded" style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}>PK</span>
                        )}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* Connection Info */}
          <div className="p-3 border-t text-xs space-y-2" style={{ borderColor: 'var(--borderDefault)' }}>
            <div className="flex items-center gap-2" style={{ color: 'var(--textMuted)' }}>
              <Database size={12} />
              <span>{activeConnection.dbName}</span>
            </div>
            <div className="flex items-center gap-2" style={{ color: 'var(--textMuted)' }}>
              {activeConnection.permission === 'ro' ? <Lock size={12} /> : <Unlock size={12} />}
              <span>{activeConnection.permission === 'ro' ? 'Read-only' : 'Read-write'}</span>
            </div>
            <div className="flex items-center gap-2" style={{ color: 'var(--textMuted)' }}>
              <Clock size={12} />
              <span>Last active: {new Date(activeConnection.lastActivity).toLocaleTimeString()}</span>
            </div>
          </div>
        </div>
      )}

      {/* Main Query Area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Toolbar */}
        <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-3">
            <button onClick={() => setShowSchemaPanel(!showSchemaPanel)} className="p-2 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Toggle schema">
              <TreePine size={16} style={{ color: showSchemaPanel ? 'var(--accentPrimary)' : 'var(--textMuted)' }} />
            </button>
            
            <select
              value={activeConnectionId || ''}
              onChange={(e) => setActiveConnection(e.target.value || null)}
              className="input text-sm px-3 py-2"
            >
              {connections.map(c => (
                <option key={c.id} value={c.id}>
                  {c.dbName || 'Remote Database'} {c.code} — {c.permission === 'ro' ? 'Read-only' : 'Read-write'}
                </option>
              ))}
            </select>
            
            <button onClick={() => navigate('/join-share')} className="btn-secondary text-sm px-3 py-2 rounded-lg flex items-center gap-2">
              <Share2 size={14} /> Join
            </button>
          </div>

          <div className="flex items-center gap-2">
            <button onClick={handleCopy} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-lg text-sm">
              {copied ? <Check size={14} /> : <Copy size={14} />} {copied ? 'Copied!' : 'Copy'}
            </button>
            <button onClick={() => handleExport('csv')} disabled={!queryResult} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-lg text-sm disabled:opacity-50">
              <Download size={14} /> CSV
            </button>
            <button onClick={() => handleExport('json')} disabled={!queryResult} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-lg text-sm disabled:opacity-50">
              <FileText size={14} /> JSON
            </button>
            <button onClick={handleExecute} disabled={isExecuting || !currentSql.trim()} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-lg text-sm disabled:opacity-50">
              {isExecuting ? <Loader2 size={14} className="animate-spin" /> : <Play size={14} />}
              {isExecuting ? 'Running...' : 'Execute'}
            </button>
          </div>
        </div>

        {/* SQL Editor */}
        <div className="flex-1 relative">
          <textarea
            ref={textareaRef}
            value={currentSql}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            className="sql-editor w-full h-full p-4 resize-none outline-none font-mono text-sm"
            placeholder="-- Write your SQL query here
-- Use Ctrl+Enter to execute
-- Tables and columns autocomplete as you type"
            spellCheck={false}
            disabled={isExecuting}
          />
          
          {/* Autocomplete Dropdown */}
          {autocompleteOpen && (
            <div className="absolute z-50 w-64 max-h-48 overflow-auto rounded-xl border shadow-lg" 
              style={{ 
                backgroundColor: 'var(--bgElevated)', 
                borderColor: 'var(--borderDefault)',
                top: 'auto',
                left: 16,
                bottom: 16,
              }}>
              {autocompleteSuggestions.map((s, i) => (
                <button
                  key={`${s.type}-${s.label}`}
                  onClick={() => handleAutocompleteSelect(s)}
                  className="w-full text-left p-2 text-sm transition-all flex items-center gap-2"
                  style={{
                    backgroundColor: i === autocompleteIndex ? 'var(--surfaceActive)' : 'transparent',
                    color: i === autocompleteIndex ? 'var(--accentPrimary)' : 'var(--textSecondary)',
                  }}
                >
                  <span className="text-xs px-1.5 py-0.5 rounded" style={{ 
                    backgroundColor: s.type === 'table' ? 'var(--accentPrimary)' : 
                      s.type === 'column' ? 'var(--accentSecondary)' : 'var(--bgTertiary)',
                    color: 'var(--textInverse)',
                  }}>
                    {s.type[0].toUpperCase()}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="truncate">{s.label}</div>
                    {s.detail && (
                      <div className="text-xs truncate" style={{ color: 'var(--textMuted)' }}>{s.detail}</div>
                    )}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Error */}
        {queryError && (
          <div className="p-4 border-t flex items-center gap-2" style={{ borderColor: 'var(--accentError)', backgroundColor: 'rgba(255,68,68,0.05)' }}>
            <AlertCircle size={16} style={{ color: 'var(--accentError)' }} />
            <span className="text-sm" style={{ color: 'var(--accentError)' }}>{queryError}</span>
            <button onClick={clearError} className="ml-auto"><X size={14} /></button>
          </div>
        )}

        {/* Results */}
        {queryResult && (
          <div className="flex-1 border-t overflow-auto" style={{ borderColor: 'var(--borderDefault)', maxHeight: '50%' }}>
            <div className="flex items-center justify-between p-3 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
              <div className="flex items-center gap-4">
                <span className="text-sm" style={{ color: 'var(--textSecondary)' }}>{queryResult.rowCount} rows</span>
                <span className="text-sm" style={{ color: 'var(--textMuted)' }}>{queryResult.executionTimeMs}ms</span>
              </div>
            </div>
            <table className="w-full">
              <thead>
                <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  {queryResult.columns.map((col, i) => (
                    <th key={i} className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                      {col}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {queryResult.rows.map((row, i) => (
                  <tr key={i} style={{ backgroundColor: i % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)' }}>
                    {row.map((cell, j) => (
                      <td key={j} className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)', borderBottom: '1px solid var(--borderDefault)' }}>
                        {cell === null ? (
                          <span className="text-xs italic" style={{ color: 'var(--textMuted)' }}>NULL</span>
                        ) : typeof cell === 'boolean' ? (
                          cell ? 'true' : 'false'
                        ) : (
                          String(cell).substring(0, 100)
                        )}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* History Panel */}
        {showHistoryPanel && (
          <div className="h-48 border-t flex flex-col" style={{ borderColor: 'var(--borderDefault)' }}>
            <div className="p-2 border-b flex items-center justify-between" style={{ borderColor: 'var(--borderDefault)' }}>
              <div className="flex items-center gap-2">
                <History size={14} />
                <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Query History</span>
              </div>
              <button onClick={() => setShowHistoryPanel(false)}><X size={14} /></button>
            </div>
            <div className="flex-1 overflow-auto p-2 space-y-1">
              {queryHistory.map((h) => (
                <button
                  key={h.id}
                  onClick={() => setCurrentSql(h.sql)}
                  className="w-full text-left p-2 rounded-lg text-xs transition-all"
                  style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
                >
                  <div className="flex items-center justify-between">
                    <span className="truncate flex-1 font-mono">{h.sql}</span>
                    <span className="text-xs" style={{ color: h.status === 'success' ? 'var(--accentSuccess)' : 'var(--accentError)' }}>
                      {h.status === 'success' ? <Check size={10} /> : <AlertCircle size={10} />}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 mt-1" style={{ color: 'var(--textMuted)' }}>
                    <span>{h.rowCount} rows</span>
                    <span>{h.executionTimeMs}ms</span>
                    <span>{new Date(h.executedAt).toLocaleTimeString()}</span>
                  </div>
                </button>
              ))}
              {queryHistory.length === 0 && (
                <p className="text-xs text-center py-4" style={{ color: 'var(--textMuted)' }}>No queries yet</p>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
