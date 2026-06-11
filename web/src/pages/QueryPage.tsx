import { useState, useEffect } from 'react';
import { Play, Copy, Check, Download, Clock, AlertCircle } from 'lucide-react';
import { api, DatabaseInstance } from '../services/api';
import { useDatabaseStore } from '../stores/databaseStore';

interface QueryResult {
  columns: string[]; rows: any[][]; executionTime: number; rowCount: number;
}

export function QueryPage() {
  const { databases } = useDatabaseStore();
  const runningDbs = databases.filter(d => d.status === 'running');
  const [selectedDb, setSelectedDb] = useState<string>('');
  const [query, setQuery] = useState('SELECT * FROM users LIMIT 10;');
  const [results, setResults] = useState<QueryResult | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queryHistory, setQueryHistory] = useState<string[]>([]);

  useEffect(() => {
    if (runningDbs.length > 0 && !selectedDb) {
      setSelectedDb(runningDbs[0].id);
    }
  }, [runningDbs]);

  const handleExecute = async () => {
    if (!selectedDb || !query.trim()) return;
    setIsExecuting(true);
    setError(null);
    const start = performance.now();
    try {
      const res = await api.executeQuery(selectedDb, query);
      setResults({
        columns: res.columns,
        rows: res.rows,
        rowCount: res.row_count,
        executionTime: Math.round(performance.now() - start),
      });
      setQueryHistory(prev => [query, ...prev.slice(0, 49)]);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Query failed');
      setResults(null);
    }
    setIsExecuting(false);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(query);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleExport = () => {
    if (!results) return;
    const csv = [results.columns.join(','), ...results.rows.map(row => row.join(','))].join('\n');
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = 'query-results.csv'; a.click();
  };

  return (
    <div className="flex h-full">
      <div className="w-64 border-r flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Query History</h3>
        </div>
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {queryHistory.map((q, index) => (
            <button key={index} onClick={() => setQuery(q)} className="w-full text-left p-3 rounded-xl text-xs transition-all"
              style={{ backgroundColor: query === q ? 'var(--surfaceActive)' : 'transparent', color: 'var(--textSecondary)' }}>
              <Clock size={12} className="inline mr-2" />
              {q.length > 40 ? q.substring(0, 40) + '...' : q}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 flex flex-col">
        <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-2">
            <select
              className="input px-3 py-2 rounded-xl text-sm"
              value={selectedDb}
              onChange={e => setSelectedDb(e.target.value)}
              disabled={runningDbs.length === 0}
            >
              {runningDbs.map(db => (
                <option key={db.id} value={db.id}>{db.name} ({db.type})</option>
              ))}
              {runningDbs.length === 0 && <option>No running databases</option>}
            </select>
            <button onClick={handleExecute} disabled={isExecuting || !selectedDb} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
              <Play size={16} /> {isExecuting ? 'Executing...' : 'Execute'}
            </button>
            <button onClick={handleCopy} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              {copied ? <Check size={16} /> : <Copy size={16} />} {copied ? 'Copied!' : 'Copy'}
            </button>
            <button onClick={handleExport} disabled={!results} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              <Download size={16} /> Export CSV
            </button>
          </div>
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>Ctrl+Enter to execute</span>
        </div>

        <div className="flex-1 flex flex-col">
          <textarea value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => { if (e.ctrlKey && e.key === 'Enter') handleExecute(); }}
            className="sql-editor flex-1 p-4 resize-none outline-none" placeholder="Write your SQL query here..." spellCheck={false} />
        </div>

        {error && (
          <div className="p-4 border-t" style={{ borderColor: 'var(--accentError)', backgroundColor: 'rgba(255,68,68,0.05)' }}>
            <div className="flex items-center gap-2">
              <AlertCircle size={16} style={{ color: 'var(--accentError)' }} />
              <span className="text-sm" style={{ color: 'var(--accentError)' }}>{error}</span>
            </div>
          </div>
        )}

        {results && (
          <div className="flex-1 border-t overflow-auto" style={{ borderColor: 'var(--borderDefault)' }}>
            <div className="flex items-center justify-between p-3 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
              <div className="flex items-center gap-4">
                <span className="text-sm" style={{ color: 'var(--textSecondary)' }}>{results.rowCount} rows</span>
                <span className="text-sm" style={{ color: 'var(--textMuted)' }}>{results.executionTime}ms</span>
              </div>
            </div>
            <table className="w-full">
              <thead>
                <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  {results.columns.map((col, index) => (
                    <th key={index} className="text-left px-4 py-3 text-xs font-semibold uppercase" style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>{col}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {results.rows.map((row, rowIndex) => (
                  <tr key={rowIndex} className="transition-all" style={{ backgroundColor: rowIndex % 2 === 0 ? 'var(--bgPrimary)' : 'var(--bgSecondary)' }}>
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex} className="px-4 py-3 text-sm font-mono" style={{ color: 'var(--textPrimary)', borderBottom: '1px solid var(--borderDefault)' }}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

