import { useState } from 'react';
import { Play, Copy, Check, Download, Clock, Save, FileText } from 'lucide-react';

interface QueryResult {
  columns: string[]; rows: any[][]; executionTime: number; rowCount: number;
}

const mockResults: QueryResult = {
  columns: ['id', 'name', 'email', 'created_at', 'status'],
  rows: [
    [1, 'Alice Johnson', 'alice@example.com', '2024-01-15', 'active'],
    [2, 'Bob Smith', 'bob@example.com', '2024-02-20', 'active'],
    [3, 'Charlie Brown', 'charlie@example.com', '2024-03-10', 'inactive'],
    [4, 'Diana Prince', 'diana@example.com', '2024-04-05', 'active'],
    [5, 'Eve Davis', 'eve@example.com', '2024-05-12', 'pending'],
  ],
  executionTime: 142, rowCount: 5,
};

const queryHistory = [
  'SELECT * FROM users WHERE status = \'active\'',
  'SELECT COUNT(*) FROM orders',
  'UPDATE users SET status = \'active\' WHERE id = 3',
  'CREATE INDEX idx_users_email ON users(email)',
];

export function QueryPage() {
  const [query, setQuery] = useState('SELECT * FROM users WHERE status = \'active\';');
  const [results, setResults] = useState<QueryResult | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [savedQueries, setSavedQueries] = useState<string[]>([]);

  const handleExecute = () => {
    setIsExecuting(true);
    setTimeout(() => { setResults(mockResults); setIsExecuting(false); }, 500);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(query);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleSave = () => {
    setSavedQueries([...savedQueries, query]);
  };

  const handleExport = async () => {
    if (!results) return;
    const csv = [results.columns.join(','), ...results.rows.map(row => row.join(','))].join('\n');

    // Try native save dialog first
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      const { save } = window.__TAURI__.dialog;
      // @ts-ignore
      const { writeTextFile } = window.__TAURI__.fs;
      const path = await save({ defaultPath: 'query-results.csv', filters: [{ name: 'CSV', extensions: ['csv'] }] });
      if (path) {
        // @ts-ignore
        await writeTextFile(path, csv);
        return;
      }
    }

    // Fallback to browser download
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
        {savedQueries.length > 0 && (
          <>
            <div className="p-4 border-t border-b" style={{ borderColor: 'var(--borderDefault)' }}>
              <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Saved Queries</h3>
            </div>
            <div className="p-2 space-y-1">
              {savedQueries.map((q, index) => (
                <button key={index} onClick={() => setQuery(q)} className="w-full text-left p-3 rounded-xl text-xs transition-all"
                  style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
                  <FileText size={12} className="inline mr-2" />
                  {q.length > 40 ? q.substring(0, 40) + '...' : q}
                </button>
              ))}
            </div>
          </>
        )}
      </div>

      <div className="flex-1 flex flex-col">
        <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <div className="flex items-center gap-2">
            <button onClick={handleExecute} disabled={isExecuting} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
              <Play size={16} /> {isExecuting ? 'Executing...' : 'Execute'}
            </button>
            <button onClick={handleCopy} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              {copied ? <Check size={16} /> : <Copy size={16} />} {copied ? 'Copied!' : 'Copy'}
            </button>
            <button onClick={handleSave} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              <Save size={16} /> Save
            </button>
            <button onClick={handleExport} disabled={!results} className="btn-secondary flex items-center gap-2 px-3 py-2 rounded-xl">
              <Download size={16} /> Export
            </button>
          </div>
          <span className="text-xs" style={{ color: 'var(--textMuted)' }}>Ctrl+Enter to execute</span>
        </div>

        <div className="flex-1 flex flex-col">
          <textarea value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => { if (e.ctrlKey && e.key === 'Enter') handleExecute(); }}
            className="sql-editor flex-1 p-4 resize-none outline-none" placeholder="Write your SQL query here..." spellCheck={false} />
        </div>

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

