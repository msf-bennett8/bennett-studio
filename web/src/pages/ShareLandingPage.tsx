import { useState, useEffect, useRef } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import { 
  Database, Lock, Globe, Clock, Zap, ExternalLink, Play, Table2, 
  ChevronRight, AlertCircle, Loader2, Code2, Terminal, Copy, Check,
  Shield, Server, Wifi, WifiOff, ChevronDown, ChevronUp, Info,
  FileJson, Braces, Eye, EyeOff
} from 'lucide-react';
import { clientFromUrl, extractConnectionInfo, type ConnectionInfo } from '@bennettstudio/sdk';

export function ShareLandingPage({ code: propCode, token: propToken }: { code?: string; token?: string } = {}) {
  const { code: urlCode } = useParams<{ code: string }>();
  const [searchParams] = useSearchParams();
  
  const code = propCode || urlCode;
  const token = propToken || searchParams.get('t');

  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [schema, setSchema] = useState<any[] | null>(null);
  const [schemaLoading, setSchemaLoading] = useState(false);
  const [queryResult, setQueryResult] = useState<any>(null);
  const [queryLoading, setQueryLoading] = useState(false);
  const [sql, setSql] = useState('SELECT * FROM users LIMIT 10');
  
  // Connection diagnostics
  const [connectionMode, setConnectionMode] = useState<'p2p' | 'relay' | 'direct' | 'unknown'>('unknown');
  const [connectionLatency, setConnectionLatency] = useState<number | null>(null);
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  
  // Code snippet copy states
  const [copiedSnippet, setCopiedSnippet] = useState<string | null>(null);
  
  // Expandable sections
  const [expandedSection, setExpandedSection] = useState<string | null>(null);
  
  // Raw JWT viewer
  const [showRawJwt, setShowRawJwt] = useState(false);
  
  // Connection attempt log
  const [connectionLog, setConnectionLog] = useState<string[]>([]);
  
  const queryInputRef = useRef<HTMLInputElement>(null);

  // Build full share URL
  const shareUrl = typeof window !== 'undefined'
    ? `${window.location.origin}/db/${code}?t=${token}`
    : '';

  useEffect(() => {
    if (!code || !token) {
      setError('Invalid share URL — missing code or token');
      setLoading(false);
      return;
    }

    const log = (msg: string) => setConnectionLog(prev => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);

    try {
      log('Parsing share URL...');
      const info = extractConnectionInfo(shareUrl);
      if (!info) {
        setError('Could not parse share URL');
        log('ERROR: Failed to parse share URL');
      } else {
        setConnectionInfo(info);
        setConnectionMode(info.mode);
        log(`Connection mode detected: ${info.mode}`);
        log(`Database: ${info.dbId}`);
        log(`Permissions: ${info.permission}`);
        log(`Tables: ${info.tables?.join(', ') || 'all'}`);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to parse share URL';
      setError(msg);
      log(`ERROR: ${msg}`);
    } finally {
      setLoading(false);
    }
  }, [code, token, shareUrl]);

  const [, setDeepLinkStatus] = useState<'idle' | 'opening' | 'unavailable'>('idle');

  const handleDeepLink = () => {
    // Open in Bennett Studio desktop app
    const deepLink = `bennett://share/${code}?t=${encodeURIComponent(token || '')}`;
    setDeepLinkStatus('opening');
    
    // Try to open the app
    window.location.href = deepLink;
    
    // If app doesn't open within 2s, show fallback
    setTimeout(() => {
      setDeepLinkStatus('unavailable');
    }, 2000);
  };

  const handleQueryInBrowser = async () => {
    if (!shareUrl) return;
    setQueryLoading(true);
    setQueryResult(null);
    const startTime = performance.now();

    try {
      setConnectionLog(prev => [...prev, `[${new Date().toLocaleTimeString()}] Executing query...`]);
      const client = clientFromUrl(shareUrl);
      const result = await client.query(sql);
      const latency = Math.round(performance.now() - startTime);
      setConnectionLatency(latency);
      setQueryResult(result);
      setConnectionLog(prev => [...prev, `[${new Date().toLocaleTimeString()}] Query OK — ${result.rowCount} rows in ${latency}ms`]);
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Query failed';
      setQueryResult({ error: msg });
      setConnectionLog(prev => [...prev, `[${new Date().toLocaleTimeString()}] ERROR: ${msg}`]);
    } finally {
      setQueryLoading(false);
    }
  };

  const handleFetchSchema = async () => {
    if (!shareUrl) return;
    setSchemaLoading(true);
    setSchema(null);

    try {
      const client = clientFromUrl(shareUrl);
      const result = await client.getSchema();
      if (result.success && result.tables) {
        setSchema(result.tables);
      } else {
        setSchema([]);
      }
    } catch (e) {
      setSchema([]);
    } finally {
      setSchemaLoading(false);
    }
  };

  const copyToClipboard = async (text: string, label: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedSnippet(label);
    setTimeout(() => setCopiedSnippet(null), 2000);
  };

  const getConnectionIcon = () => {
    switch (connectionMode) {
      case 'p2p': return <Wifi size={14} className="text-emerald-400" />;
      case 'direct': return <Server size={14} className="text-blue-400" />;
      case 'relay': return <Globe size={14} className="text-amber-400" />;
      default: return <WifiOff size={14} className="text-gray-400" />;
    }
  };

  const getModeColor = () => {
    switch (connectionMode) {
      case 'p2p': return 'text-emerald-400 bg-emerald-400/10 border-emerald-400/20';
      case 'direct': return 'text-blue-400 bg-blue-400/10 border-blue-400/20';
      case 'relay': return 'text-amber-400 bg-amber-400/10 border-amber-400/20';
      default: return 'text-gray-400 bg-gray-400/10 border-gray-400/20';
    }
  };

  const toggleSection = (section: string) => {
    setExpandedSection(expandedSection === section ? null : section);
  };

  const getModeBadge = () => {
    if (!connectionInfo) return null;
    const mode = connectionInfo.mode;
    const baseClass = "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border";
    
    if (mode === 'p2p') {
      return (
        <span className={`${baseClass} text-emerald-400 bg-emerald-400/10 border-emerald-400/20`}>
          <Zap size={12} className="animate-pulse" /> P2P Direct
        </span>
      );
    }
    if (mode === 'direct') {
      return (
        <span className={`${baseClass} text-blue-400 bg-blue-400/10 border-blue-400/20`}>
          <Globe size={12} /> Direct HTTP
        </span>
      );
    }
    return (
      <span className={`${baseClass} text-gray-400 bg-gray-400/10 border-gray-400/20`}>
        <WifiOff size={12} /> Relay Fallback
      </span>
    );
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center" style={{ backgroundColor: 'var(--bgPrimary)' }}>
        <div className="text-center">
          <Loader2 size={40} className="animate-spin mx-auto mb-4" style={{ color: 'var(--accentPrimary)' }} />
          <p className="text-sm" style={{ color: 'var(--textMuted)' }}>Establishing secure connection...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center p-8" style={{ backgroundColor: 'var(--bgPrimary)' }}>
        <div className="max-w-md w-full p-6 rounded-2xl text-center" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <AlertCircle size={48} className="mx-auto mb-4" style={{ color: 'var(--accentError)' }} />
          <h1 className="text-xl font-bold mb-2" style={{ color: 'var(--textPrimary)' }}>Invalid Share Link</h1>
          <p className="text-sm mb-4" style={{ color: 'var(--textSecondary)' }}>{error}</p>
          <div className="p-3 rounded-lg text-left font-mono text-xs" style={{ backgroundColor: 'var(--bgTertiary)' }}>
            <p style={{ color: 'var(--textMuted)' }}>Expected format:</p>
            <p style={{ color: 'var(--textSecondary)' }}>https://share.bennett.studio/db/CODE?t=JWT</p>
          </div>
        </div>
      </div>
    );
  }

  const sdkInstallSnippet = `npm install @bennettstudio/sdk`;

  const sdkUsageSnippet = `import { BennettClient } from '@bennettstudio/sdk';

const db = await BennettClient.fromShareUrl(
  '${shareUrl}'
);

// Query your data
const users = await db.query('SELECT * FROM users LIMIT 10');
console.log(users.rows);

// Get schema
const schema = await db.getSchema();
console.log(schema.tables);`;

  const curlSnippet = `curl -X POST https://bennett-relay.onrender.com/api/share/${code}/query \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer ${token?.substring(0, 20)}..." \\
  -d '{"sql": "SELECT * FROM users LIMIT 10"}'`;

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
          <span className="inline-flex items-center gap-1">
            {getConnectionIcon()}
            {getModeBadge()}
          </span>
          {connectionLatency !== null && (
            <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>
              {connectionLatency}ms
            </span>
          )}
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
            {getModeBadge()}
            <span className="w-px h-4" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium" 
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              <Lock size={10} /> {connectionInfo?.permission || 'ro'}
            </span>
            <span className="w-px h-4" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium" 
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              <Clock size={10} />
              {connectionInfo?.expiresAt
                ? `Expires ${new Date(connectionInfo.expiresAt * 1000).toLocaleDateString()}`
                : 'No expiry'}
            </span>
            <span className="w-px h-4" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <span className="inline-flex items-center gap-1 text-xs font-mono" style={{ color: 'var(--textMuted)' }}>
              <Code2 size={10} /> {connectionInfo?.dbId || code}
            </span>
          </div>
        </div>

        {/* Quick Actions Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-10">
          {/* SDK Card */}
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

          {/* Deep Link Card */}
          <button
            onClick={handleDeepLink}
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

          {/* cURL Card */}
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
            <p className="text-xs mt-3" style={{ color: 'var(--textMuted)' }}>
              Replace the token with your full JWT. The relay proxies to the host database automatically.
            </p>
          </div>
        )}

        {/* Live Query Editor */}
        <div className="mb-8 p-6 rounded-2xl" 
          style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
              <Play size={16} style={{ color: 'var(--accentPrimary)' }} /> Live Query
            </h3>
            {connectionLatency !== null && (
              <span className="text-xs font-mono px-2 py-1 rounded-full" 
                style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                Last: {connectionLatency}ms
              </span>
            )}
          </div>
          <div className="flex gap-3">
            <input
              ref={queryInputRef}
              type="text"
              value={sql}
              onChange={(e) => setSql(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleQueryInBrowser()}
              className="flex-1 font-mono text-sm px-4 py-2.5 rounded-xl outline-none transition-all"
              style={{ 
                backgroundColor: 'var(--bgSecondary)', 
                color: 'var(--textPrimary)',
                border: '1px solid var(--borderDefault)'
              }}
              placeholder="SELECT * FROM users LIMIT 10"
            />
            <button
              onClick={handleQueryInBrowser}
              disabled={queryLoading}
              className="px-5 py-2.5 rounded-xl flex items-center gap-2 font-medium text-sm disabled:opacity-50 transition-all"
              style={{ backgroundColor: 'var(--accentPrimary)', color: '#fff' }}
            >
              {queryLoading ? <Loader2 size={16} className="animate-spin" /> : <Play size={16} />}
              Run
            </button>
          </div>

          {/* Query Results */}
          {queryResult && (
            <div className="mt-4">
              {queryResult.error ? (
                <div className="p-4 rounded-xl flex items-start gap-3" 
                  style={{ backgroundColor: 'rgba(255,68,68,0.08)', border: '1px solid rgba(255,68,68,0.2)' }}>
                  <AlertCircle size={16} className="mt-0.5 flex-shrink-0" style={{ color: 'var(--accentError)' }} />
                  <p className="text-sm" style={{ color: 'var(--accentError)' }}>{queryResult.error}</p>
                </div>
              ) : (
                <div>
                  <div className="flex items-center justify-between mb-3 px-1">
                    <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>
                      {queryResult.rowCount} rows · {queryResult.executionTimeMs}ms
                    </span>
                    <button
                      onClick={() => copyToClipboard(JSON.stringify(queryResult.rows, null, 2), 'results')}
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
                          {queryResult.columns?.map((col: string) => (
                            <th key={col} className="text-left px-3 py-2.5 font-medium text-xs" 
                              style={{ color: 'var(--textSecondary)', borderBottom: '1px solid var(--borderDefault)' }}>
                              {col}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {queryResult.rows?.map((row: any[], i: number) => (
                          <tr key={i} style={{ borderBottom: '1px solid var(--borderSubtle)' }}>
                            {row.map((cell, j) => (
                              <td key={j} className="px-3 py-2 font-mono text-xs" 
                                style={{ color: 'var(--textPrimary)' }}>
                                {cell === null ? (
                                  <span style={{ color: 'var(--textMuted)' }}>NULL</span>
                                ) : (
                                  JSON.stringify(cell)
                                )}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
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
            <button
              onClick={handleFetchSchema}
              disabled={schemaLoading}
              className="text-xs px-3 py-1.5 rounded-lg transition-all hover:opacity-80 font-medium"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
            >
              {schemaLoading ? <Loader2 size={14} className="animate-spin inline mr-1" /> : null}
              {schemaLoading ? 'Loading...' : schema === null ? 'Load Schema' : 'Refresh'}
            </button>
          </div>

          {schema === null ? (
            <div className="text-center py-10">
              <Table2 size={32} className="mx-auto mb-3 opacity-20" style={{ color: 'var(--textMuted)' }} />
              <p className="text-sm" style={{ color: 'var(--textMuted)' }}>
                Load the schema to explore tables and columns
              </p>
            </div>
          ) : schema.length === 0 ? (
            <div className="text-center py-10">
              <Info size={32} className="mx-auto mb-3 opacity-20" style={{ color: 'var(--textMuted)' }} />
              <p className="text-sm" style={{ color: 'var(--textMuted)' }}>
                No tables found or schema unavailable
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {schema.map((table: any) => (
                <div key={table.name} className="p-3 rounded-xl transition-all hover:opacity-80" 
                  style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  <div className="flex items-center gap-2 mb-2">
                    <Table2 size={14} style={{ color: 'var(--accentPrimary)' }} />
                    <span className="font-medium text-sm" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                    <span className="text-xs px-2 py-0.5 rounded-full" 
                      style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                      {table.columns?.length || 0} columns
                    </span>
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {table.columns?.slice(0, 6).map((col: any) => (
                      <span key={col.name} className="text-xs px-2 py-0.5 rounded-full" 
                        style={{ backgroundColor: 'var(--bgPrimary)', color: 'var(--textSecondary)', border: '1px solid var(--borderSubtle)' }}>
                        {col.name} <span style={{ color: 'var(--textMuted)' }}>{col.dataType}</span>
                      </span>
                    ))}
                    {table.columns?.length > 6 && (
                      <span className="text-xs px-2 py-0.5 rounded-full" 
                        style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                        +{table.columns.length - 6} more
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Connection Diagnostics (Collapsible) */}
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
                {connectionLog.length} events
              </span>
            </div>
            {showDiagnostics ? <ChevronUp size={16} style={{ color: 'var(--textMuted)' }} /> 
              : <ChevronDown size={16} style={{ color: 'var(--textMuted)' }} />}
          </button>
          
          {showDiagnostics && (
            <div className={`mt-2 p-4 rounded-2xl font-mono text-xs space-y-1 ${getModeColor()}`} 
              style={{ backgroundColor: 'var(--bgSecondary)', border: '1px solid var(--borderDefault)' }}>
              {connectionLog.map((log, i) => (
                <div key={i} className="flex gap-2">
                  <span style={{ color: 'var(--textMuted)' }}>›</span>
                  <span style={{ color: log.includes('ERROR') ? 'var(--accentError)' : 'var(--textSecondary)' }}>
                    {log}
                  </span>
                </div>
              ))}
              {connectionLog.length === 0 && (
                <span style={{ color: 'var(--textMuted)' }}>No connection events yet.</span>
              )}
            </div>
          )}
        </div>

        {/* Raw JWT Inspector (Collapsible) */}
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
          
          {showRawJwt && connectionInfo && (
            <div className="mt-2 p-4 rounded-2xl" 
              style={{ backgroundColor: 'var(--bgSecondary)', border: '1px solid var(--borderDefault)' }}>
              <pre className="font-mono text-xs overflow-x-auto" style={{ color: 'var(--textSecondary)' }}>
                {JSON.stringify({
                  mode: connectionInfo.mode,
                  code: connectionInfo.code,
                  dbId: connectionInfo.dbId,
                  permission: connectionInfo.permission,
                  tables: connectionInfo.tables,
                  host: connectionInfo.host,
                  port: connectionInfo.port,
                  expiresAt: connectionInfo.expiresAt ? new Date(connectionInfo.expiresAt * 1000).toISOString() : null,
                }, null, 2)}
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
