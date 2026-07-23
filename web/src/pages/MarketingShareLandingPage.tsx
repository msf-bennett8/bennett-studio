import { useState } from 'react';
import {
  Database, Lock, Clock, Zap, ExternalLink, Play, Table2,
  ChevronRight, Loader2, Code2, Terminal, Copy, Check,
  Shield, Wifi, ChevronDown, ChevronUp,
  FileJson, Braces, Eye, EyeOff, Sparkles, MousePointer, ArrowRight
} from 'lucide-react';

export function MarketingShareLandingPage() {
  const [expandedSection, setExpandedSection] = useState<string | null>(null);
  const [copiedSnippet, setCopiedSnippet] = useState<string | null>(null);
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [showRawJwt, setShowRawJwt] = useState(false);
  const [activeQuery, setActiveQuery] = useState(false);
  const [queryResults, setQueryResults] = useState(false);
  const [hoveredTable, setHoveredTable] = useState<string | null>(null);

  const copyToClipboard = async (text: string, label: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedSnippet(label);
    setTimeout(() => setCopiedSnippet(null), 2000);
  };

  const toggleSection = (section: string) => {
    setExpandedSection(expandedSection === section ? null : section);
  };

  const handleRunQuery = () => {
    setActiveQuery(true);
    setTimeout(() => {
      setActiveQuery(false);
      setQueryResults(true);
    }, 800);
  };

  const sdkInstallSnippet = `npm install @bennettstudio/sdk`;

  const sdkUsageSnippet = `import { BennettClient } from '@bennettstudio/sdk';

const db = await BennettClient.fromShareUrl(
  'https://share.bennett.studio/db/DEMO123?t=eyJ...'
);

// Query your data
const users = await db.query('SELECT * FROM users LIMIT 10');
console.log(users.rows);`;

  const curlSnippet = `curl -X POST https://bennett-relay.onrender.com/api/share/DEMO123/query \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer eyJ..." \\
  -d '{"sql": "SELECT * FROM users LIMIT 10"}'`;

  const sampleUsers = [
    { id: 1, name: "Alice Johnson", email: "alice@example.com", created_at: "2024-01-15T10:30:00Z" },
    { id: 2, name: "Bob Smith", email: "bob@example.com", created_at: "2024-02-20T14:45:00Z" },
    { id: 3, name: "Carol White", email: "carol@example.com", created_at: "2024-03-10T09:15:00Z" },
    { id: 4, name: "David Brown", email: "david@example.com", created_at: "2024-04-05T16:20:00Z" },
    { id: 5, name: "Eva Green", email: "eva@example.com", created_at: "2024-05-12T11:00:00Z" },
    { id: 6, name: "Frank Lee", email: "frank@example.com", created_at: "2024-06-18T08:45:00Z" },
    { id: 7, name: "Grace Kim", email: "grace@example.com", created_at: "2024-07-22T13:30:00Z" },
    { id: 8, name: "Henry Wilson", email: "henry@example.com", created_at: "2024-08-30T17:10:00Z" },
    { id: 9, name: "Ivy Chen", email: "ivy@example.com", created_at: "2024-09-14T09:55:00Z" },
    { id: 10, name: "Jack Taylor", email: "jack@example.com", created_at: "2024-10-01T14:25:00Z" },
  ];

  const sampleOrders = [
    { id: 101, user_id: 1, total: 149.99, status: "completed", created_at: "2024-11-01T10:00:00Z" },
    { id: 102, user_id: 2, total: 299.50, status: "pending", created_at: "2024-11-02T11:30:00Z" },
    { id: 103, user_id: 3, total: 49.99, status: "completed", created_at: "2024-11-03T09:15:00Z" },
  ];

  const tables = [
    {
      name: "users",
      columns: 4,
      columnData: [
        { name: "id", type: "INTEGER", nullable: false, primary: true },
        { name: "name", type: "VARCHAR(255)", nullable: false, primary: false },
        { name: "email", type: "VARCHAR(255)", nullable: false, primary: false },
        { name: "created_at", type: "TIMESTAMP", nullable: true, primary: false },
      ],
      rowCount: 1247,
      sampleRows: sampleUsers,
    },
    {
      name: "orders",
      columns: 5,
      columnData: [
        { name: "id", type: "INTEGER", nullable: false, primary: true },
        { name: "user_id", type: "INTEGER", nullable: false, primary: false },
        { name: "total", type: "DECIMAL(10,2)", nullable: false, primary: false },
        { name: "status", type: "VARCHAR(50)", nullable: false, primary: false },
        { name: "created_at", type: "TIMESTAMP", nullable: true, primary: false },
      ],
      rowCount: 3842,
      sampleRows: sampleOrders,
    },
    {
      name: "products",
      columns: 6,
      columnData: [
        { name: "id", type: "INTEGER", nullable: false, primary: true },
        { name: "sku", type: "VARCHAR(100)", nullable: false, primary: false },
        { name: "name", type: "VARCHAR(255)", nullable: false, primary: false },
        { name: "price", type: "DECIMAL(10,2)", nullable: false, primary: false },
        { name: "stock", type: "INTEGER", nullable: false, primary: false },
        { name: "updated_at", type: "TIMESTAMP", nullable: true, primary: false },
      ],
      rowCount: 856,
      sampleRows: [],
    },
  ];

  return (
    <div className="min-h-screen" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      {/* Top Navigation Bar */}
      <nav className="sticky top-0 z-50 px-6 py-3 flex items-center justify-between backdrop-blur-xl"
        style={{ backgroundColor: 'rgba(15,23,42,0.8)', borderBottom: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-2">
          <Database size={20} style={{ color: 'var(--accentPrimary)' }} />
          <span className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Bennett Studio</span>
        </div>
        <div className="flex items-center gap-3">
          <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border text-emerald-400 bg-emerald-400/10 border-emerald-400/20">
            <Zap size={12} className="animate-pulse" /> P2P Direct
          </span>
          <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>24ms</span>
        </div>
      </nav>

      <div className="max-w-5xl mx-auto px-6 py-10">
        {/* Hero Section */}
        <div className="text-center mb-12">
          <div className="w-16 h-16 rounded-2xl flex items-center justify-center mx-auto mb-5"
            style={{ backgroundColor: 'rgba(0,212,170,0.1)', border: '1px solid rgba(0,212,170,0.2)' }}>
            <Database size={32} style={{ color: 'var(--accentSuccess)' }} />
          </div>
          <h1 className="text-4xl font-bold mb-3 tracking-tight" style={{ color: 'var(--textPrimary)' }}>
            Shared Database
          </h1>
          <p className="text-base mb-5 max-w-lg mx-auto" style={{ color: 'var(--textSecondary)' }}>
            Query this database directly from your code. No server setup, no API keys — just a share URL.
          </p>

          {/* Connection Status Bar */}
          <div className="inline-flex items-center gap-3 px-4 py-2 rounded-xl"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
            <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border text-emerald-400 bg-emerald-400/10 border-emerald-400/20">
              <Zap size={12} className="animate-pulse" /> P2P Direct
            </span>
            <span className="w-px h-4" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              <Lock size={10} /> rw
            </span>
            <span className="w-px h-4" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              <Clock size={10} /> Expires Jan 15, 2026
            </span>
            <span className="w-px h-4" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <span className="inline-flex items-center gap-1 text-xs font-mono" style={{ color: 'var(--textMuted)' }}>
              <Code2 size={10} /> demo-postgres
            </span>
          </div>
        </div>

        {/* Quick Actions Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-10">
          <button
            onClick={() => toggleSection('sdk')}
            className="p-5 rounded-2xl text-left transition-all hover:opacity-90 group relative overflow-hidden"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="absolute top-0 right-0 w-20 h-20 opacity-5"
              style={{ background: 'radial-gradient(circle, var(--accentPrimary) 0%, transparent 70%)' }} />
            <div className="flex items-start justify-between mb-3">
              <div className="w-10 h-10 rounded-lg flex items-center justify-center"
                style={{ backgroundColor: 'rgba(0,212,170,0.1)' }}>
                <Terminal size={20} style={{ color: 'var(--accentSuccess)' }} />
              </div>
              {expandedSection === 'sdk' ? <ChevronUp size={16} style={{ color: 'var(--textMuted)' }} />
                : <ChevronDown size={16} style={{ color: 'var(--textMuted)' }} />}
            </div>
            <h3 className="font-semibold text-sm mb-1" style={{ color: 'var(--textPrimary)' }}>Use the SDK</h3>
            <p className="text-xs leading-relaxed" style={{ color: 'var(--textSecondary)' }}>
              npm install @bennettstudio/sdk — type-safe queries with auto-completion
            </p>
          </button>

          <button
            className="p-5 rounded-2xl text-left transition-all hover:opacity-90 group relative overflow-hidden"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="absolute top-0 right-0 w-20 h-20 opacity-5"
              style={{ background: 'radial-gradient(circle, #3b82f6 0%, transparent 70%)' }} />
            <div className="flex items-start justify-between mb-3">
              <div className="w-10 h-10 rounded-lg flex items-center justify-center"
                style={{ backgroundColor: 'rgba(59,130,246,0.1)' }}>
                <ExternalLink size={20} style={{ color: '#3b82f6' }} />
              </div>
              <ChevronRight size={16} className="opacity-0 group-hover:opacity-100 transition-opacity"
                style={{ color: 'var(--textMuted)' }} />
            </div>
            <h3 className="font-semibold text-sm mb-1" style={{ color: 'var(--textPrimary)' }}>Open in Bennett Studio</h3>
            <p className="text-xs leading-relaxed" style={{ color: 'var(--textSecondary)' }}>
              Full IDE with schema browser, query history, and export tools
            </p>
          </button>

          <button
            onClick={() => toggleSection('curl')}
            className="p-5 rounded-2xl text-left transition-all hover:opacity-90 group relative overflow-hidden"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="absolute top-0 right-0 w-20 h-20 opacity-5"
              style={{ background: 'radial-gradient(circle, #f59e0b 0%, transparent 70%)' }} />
            <div className="flex items-start justify-between mb-3">
              <div className="w-10 h-10 rounded-lg flex items-center justify-center"
                style={{ backgroundColor: 'rgba(245,158,11,0.1)' }}>
                <Braces size={20} style={{ color: '#f59e0b' }} />
              </div>
              {expandedSection === 'curl' ? <ChevronUp size={16} style={{ color: 'var(--textMuted)' }} />
                : <ChevronDown size={16} style={{ color: 'var(--textMuted)' }} />}
            </div>
            <h3 className="font-semibold text-sm mb-1" style={{ color: 'var(--textPrimary)' }}>Raw HTTP API</h3>
            <p className="text-xs leading-relaxed" style={{ color: 'var(--textSecondary)' }}>
              cURL, fetch, or any HTTP client — no SDK required
            </p>
          </button>
        </div>

        {/* Expandable SDK Section */}
        {expandedSection === 'sdk' && (
          <div className="mb-8 p-6 rounded-2xl"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
                <Terminal size={16} style={{ color: 'var(--accentSuccess)' }} /> SDK Installation
              </h3>
              <button
                onClick={() => copyToClipboard(sdkInstallSnippet, 'install')}
                className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-lg transition-all"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
              >
                {copiedSnippet === 'install' ? <Check size={12} /> : <Copy size={12} />}
                {copiedSnippet === 'install' ? 'Copied' : 'Copy'}
              </button>
            </div>
            <pre className="p-3 rounded-lg font-mono text-xs mb-5 overflow-x-auto"
              style={{ backgroundColor: 'var(--bgSecondary)', color: 'var(--textPrimary)' }}>
              <code>{sdkInstallSnippet}</code>
            </pre>

            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
                <Code2 size={16} style={{ color: 'var(--accentSuccess)' }} /> Usage Example
              </h3>
              <button
                onClick={() => copyToClipboard(sdkUsageSnippet, 'usage')}
                className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-lg transition-all"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
              >
                {copiedSnippet === 'usage' ? <Check size={12} /> : <Copy size={12} />}
                {copiedSnippet === 'usage' ? 'Copied' : 'Copy'}
              </button>
            </div>
            <pre className="p-3 rounded-lg font-mono text-xs overflow-x-auto"
              style={{ backgroundColor: 'var(--bgSecondary)', color: 'var(--textPrimary)' }}>
              <code>{sdkUsageSnippet}</code>
            </pre>
          </div>
        )}

        {/* Expandable cURL Section */}
        {expandedSection === 'curl' && (
          <div className="mb-8 p-6 rounded-2xl"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
                <Braces size={16} style={{ color: '#f59e0b' }} /> cURL Example
              </h3>
              <button
                onClick={() => copyToClipboard(curlSnippet, 'curl')}
                className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-lg transition-all"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
              >
                {copiedSnippet === 'curl' ? <Check size={12} /> : <Copy size={12} />}
                {copiedSnippet === 'curl' ? 'Copied' : 'Copy'}
              </button>
            </div>
            <pre className="p-3 rounded-lg font-mono text-xs overflow-x-auto"
              style={{ backgroundColor: 'var(--bgSecondary)', color: 'var(--textPrimary)' }}>
              <code>{curlSnippet}</code>
            </pre>
          </div>
        )}

        {/* Live Query Preview */}
        <div className="mb-8 p-6 rounded-2xl"
          style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
              <Play size={16} style={{ color: 'var(--accentPrimary)' }} /> Live Query
            </h3>
            <span className="text-xs font-mono px-2 py-1 rounded-full"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
              Last: 24ms
            </span>
          </div>
          <div className="flex gap-3">
            <div className="flex-1 font-mono text-sm px-4 py-2.5 rounded-xl flex items-center"
              style={{
                backgroundColor: 'var(--bgSecondary)',
                color: 'var(--textPrimary)',
                border: '1px solid var(--borderDefault)'
              }}>
              <span className="text-emerald-400 mr-2">›</span>
              SELECT * FROM users LIMIT 10
            </div>
            <button
              onClick={handleRunQuery}
              disabled={activeQuery}
              className="px-5 py-2.5 rounded-xl flex items-center gap-2 font-medium text-sm disabled:opacity-50 transition-all"
              style={{ backgroundColor: 'var(--accentPrimary)', color: '#fff' }}
            >
              {activeQuery ? <Loader2 size={16} className="animate-spin" /> : <Play size={16} />}
              Run
            </button>
          </div>

          {/* Query Results */}
          {queryResults && (
            <div className="mt-4">
              <div className="flex items-center justify-between mb-3 px-1">
                <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>
                  10 rows · 24ms
                </span>
                <button
                  onClick={() => copyToClipboard(JSON.stringify(sampleUsers, null, 2), 'results')}
                  className="flex items-center gap-1.5 text-xs px-2 py-1 rounded transition-all"
                  style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
                >
                  {copiedSnippet === 'results' ? <Check size={12} /> : <Copy size={12} />}
                  JSON
                </button>
              </div>
              <div className="overflow-x-auto rounded-xl"
                style={{ border: '1px solid var(--borderDefault)' }}>
                <table className="w-full text-sm">
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bgSecondary)' }}>
                      <th className="text-left px-3 py-2.5 font-medium text-xs"
                        style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>id</th>
                      <th className="text-left px-3 py-2.5 font-medium text-xs"
                        style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>name</th>
                      <th className="text-left px-3 py-2.5 font-medium text-xs"
                        style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>email</th>
                      <th className="text-left px-3 py-2.5 font-medium text-xs"
                        style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>created_at</th>
                    </tr>
                  </thead>
                  <tbody>
                    {sampleUsers.map((row, i) => (
                      <tr key={i} style={{ borderBottom: '1px solid var(--borderSubtle)' }}>
                        <td className="px-3 py-2 font-mono text-xs" style={{ color: 'var(--textPrimary)' }}>{row.id}</td>
                        <td className="px-3 py-2 font-mono text-xs" style={{ color: 'var(--textPrimary)' }}>{JSON.stringify(row.name)}</td>
                        <td className="px-3 py-2 font-mono text-xs" style={{ color: 'var(--textPrimary)' }}>{JSON.stringify(row.email)}</td>
                        <td className="px-3 py-2 font-mono text-xs" style={{ color: 'var(--textPrimary)' }}>{JSON.stringify(row.created_at)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>

        {/* Schema Preview */}
        <div className="mb-8 p-6 rounded-2xl"
          style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
              <Table2 size={16} style={{ color: 'var(--accentPrimary)' }} /> Schema Preview
            </h3>
            <span className="text-xs px-3 py-1.5 rounded-lg font-medium"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              3 tables · 5,945 rows
            </span>
          </div>

          <div className="space-y-2">
            {tables.map((table) => (
              <div
                key={table.name}
                className="p-3 rounded-xl transition-all cursor-pointer"
                style={{
                  backgroundColor: hoveredTable === table.name ? 'var(--bgPrimary)' : 'var(--bgSecondary)',
                  border: hoveredTable === table.name ? '1px solid var(--accentPrimary)' : '1px solid transparent',
                }}
                onMouseEnter={() => setHoveredTable(table.name)}
                onMouseLeave={() => setHoveredTable(null)}
              >
                <div className="flex items-center gap-2 mb-2">
                  <Table2 size={14} style={{ color: 'var(--accentPrimary)' }} />
                  <span className="font-medium text-sm" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                  <span className="text-xs px-2 py-0.5 rounded-full"
                    style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                    {table.columns} columns
                  </span>
                  <span className="text-xs px-2 py-0.5 rounded-full"
                    style={{ backgroundColor: 'rgba(0,212,170,0.1)', color: 'var(--accentSuccess)' }}>
                    {table.rowCount.toLocaleString()} rows
                  </span>
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {table.columnData.map((col) => (
                    <span key={col.name} className="text-xs px-2 py-0.5 rounded-full inline-flex items-center gap-1"
                      style={{ backgroundColor: 'var(--bgPrimary)', color: 'var(--textSecondary)', border: '1px solid var(--borderSubtle)' }}>
                      {col.primary && <Sparkles size={8} style={{ color: 'var(--accentPrimary)' }} />}
                      {col.name}
                      <span style={{ color: 'var(--textMuted)' }}>{col.type}</span>
                      {col.nullable && <span style={{ color: 'var(--textMuted)' }}>?</span>}
                    </span>
                  ))}
                </div>
                {hoveredTable === table.name && table.sampleRows.length > 0 && (
                  <div className="mt-3 overflow-x-auto rounded-lg"
                    style={{ border: '1px solid var(--borderSubtle)' }}>
                    <table className="w-full text-xs">
                      <thead>
                        <tr style={{ backgroundColor: 'var(--bgTertiary)' }}>
                          {Object.keys(table.sampleRows[0]).map((key) => (
                            <th key={key} className="text-left px-2 py-1.5 font-medium"
                              style={{ color: 'var(--textMuted)', borderBottom: '1px solid var(--borderSubtle)' }}>
                              {key}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {table.sampleRows.slice(0, 3).map((row, i) => (
                          <tr key={i} style={{ borderBottom: '1px solid var(--borderSubtle)' }}>
                            {Object.values(row).map((val, j) => (
                              <td key={j} className="px-2 py-1.5 font-mono" style={{ color: 'var(--textSecondary)' }}>
                                {JSON.stringify(val)}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Features Highlight */}
        <div className="mb-8 p-6 rounded-2xl"
          style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <h3 className="font-semibold mb-4 flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
            <Sparkles size={16} style={{ color: 'var(--accentPrimary)' }} /> Why Bennett Studio?
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="flex items-start gap-3 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0" style={{ backgroundColor: 'rgba(0,212,170,0.1)' }}>
                <Wifi size={16} style={{ color: 'var(--accentSuccess)' }} />
              </div>
              <div>
                <h4 className="text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>P2P-First</h4>
                <p className="text-xs" style={{ color: 'var(--textSecondary)' }}>Direct connection when possible. No data passes through our servers.</p>
              </div>
            </div>
            <div className="flex items-start gap-3 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0" style={{ backgroundColor: 'rgba(59,130,246,0.1)' }}>
                <Shield size={16} style={{ color: '#3b82f6' }} />
              </div>
              <div>
                <h4 className="text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Zero Config</h4>
                <p className="text-xs" style={{ color: 'var(--textSecondary)' }}>No port forwarding, no VPN, no firewall rules. It just works.</p>
              </div>
            </div>
            <div className="flex items-start gap-3 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0" style={{ backgroundColor: 'rgba(245,158,11,0.1)' }}>
                <MousePointer size={16} style={{ color: '#f59e0b' }} />
              </div>
              <div>
                <h4 className="text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>One-Click Share</h4>
                <p className="text-xs" style={{ color: 'var(--textSecondary)' }}>Generate a share link in seconds. Control permissions per link.</p>
              </div>
            </div>
            <div className="flex items-start gap-3 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
              <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0" style={{ backgroundColor: 'rgba(168,85,247,0.1)' }}>
                <ArrowRight size={16} style={{ color: '#a855f7' }} />
              </div>
              <div>
                <h4 className="text-sm font-medium mb-1" style={{ color: 'var(--textPrimary)' }}>Self-Hosted</h4>
                <p className="text-xs" style={{ color: 'var(--textSecondary)' }}>Your data stays on your machine. You control everything.</p>
              </div>
            </div>
          </div>
        </div>

        {/* Connection Diagnostics */}
        <div className="mb-8">
          <button
            onClick={() => setShowDiagnostics(!showDiagnostics)}
            className="w-full flex items-center justify-between p-4 rounded-2xl transition-all"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="flex items-center gap-2">
              <Shield size={16} style={{ color: 'var(--textMuted)' }} />
              <span className="text-sm font-medium" style={{ color: 'var(--textSecondary)' }}>
                Connection Diagnostics
              </span>
              <span className="text-xs px-2 py-0.5 rounded-full"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                4 events
              </span>
            </div>
            {showDiagnostics ? <ChevronUp size={16} style={{ color: 'var(--textMuted)' }} />
              : <ChevronDown size={16} style={{ color: 'var(--textMuted)' }} />}
          </button>

          {showDiagnostics && (
            <div className="mt-2 p-4 rounded-2xl font-mono text-xs space-y-1 text-emerald-400 bg-emerald-400/10 border-emerald-400/20"
              style={{ backgroundColor: 'var(--bgSecondary)', border: '1px solid var(--borderDefault)' }}>
              <div className="flex gap-2">
                <span style={{ color: 'var(--textMuted)' }}>›</span>
                <span style={{ color: 'var(--textSecondary)' }}>[10:30:15 AM] Parsing share URL...</span>
              </div>
              <div className="flex gap-2">
                <span style={{ color: 'var(--textMuted)' }}>›</span>
                <span style={{ color: 'var(--textSecondary)' }}>[10:30:15 AM] Connection mode detected: p2p</span>
              </div>
              <div className="flex gap-2">
                <span style={{ color: 'var(--textMuted)' }}>›</span>
                <span style={{ color: 'var(--textSecondary)' }}>[10:30:15 AM] Database: demo-postgres</span>
              </div>
              <div className="flex gap-2">
                <span style={{ color: 'var(--textMuted)' }}>›</span>
                <span style={{ color: 'var(--textSecondary)' }}>[10:30:15 AM] Permissions: rw</span>
              </div>
            </div>
          )}
        </div>

        {/* Raw JWT Inspector */}
        <div className="mb-10">
          <button
            onClick={() => setShowRawJwt(!showRawJwt)}
            className="w-full flex items-center justify-between p-4 rounded-2xl transition-all"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="flex items-center gap-2">
              <FileJson size={16} style={{ color: 'var(--textMuted)' }} />
              <span className="text-sm font-medium" style={{ color: 'var(--textSecondary)' }}>
                Inspect JWT Payload
              </span>
              <span className="text-xs px-2 py-0.5 rounded-full"
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                {showRawJwt ? 'Hide' : 'Show'}
              </span>
            </div>
            {showRawJwt ? <EyeOff size={16} style={{ color: 'var(--textMuted)' }} />
              : <Eye size={16} style={{ color: 'var(--textMuted)' }} />}
          </button>

          {showRawJwt && (
            <div className="mt-2 p-4 rounded-2xl"
              style={{ backgroundColor: 'var(--bgSecondary)', border: '1px solid var(--borderDefault)' }}>
              <pre className="font-mono text-xs overflow-x-auto" style={{ color: 'var(--textSecondary)' }}>
{`{
  "mode": "p2p",
  "code": "DEMO123",
  "dbId": "demo-postgres",
  "permission": "rw",
  "tables": ["users", "orders", "products"],
  "host": "192.168.1.42",
  "port": 5432,
  "expiresAt": "2026-01-15T00:00:00.000Z"
}`}
              </pre>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="text-center pb-8">
          <p className="text-xs" style={{ color: 'var(--textMuted)' }}>
            Powered by <span style={{ color: 'var(--accentPrimary)' }}>Bennett Studio</span> ·
            Self-hosted · Zero-config · P2P-first
          </p>
        </div>
      </div>
    </div>
  );
}
