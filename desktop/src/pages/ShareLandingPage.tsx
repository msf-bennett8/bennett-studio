import { useState, useEffect } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import { Database, Lock, Globe, Clock, Zap, ExternalLink, Play, Table2, ChevronRight, AlertCircle, Loader2 } from 'lucide-react';
import { clientFromUrl, extractConnectionInfo, type ConnectionInfo } from '@bennettstudio/sdk';

export function ShareLandingPage() {
  const { code } = useParams<{ code: string }>();
  const [searchParams] = useSearchParams();
  const token = searchParams.get('t');

  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [schema, setSchema] = useState<any[] | null>(null);
  const [schemaLoading, setSchemaLoading] = useState(false);
  const [queryResult, setQueryResult] = useState<any>(null);
  const [queryLoading, setQueryLoading] = useState(false);
  const [sql, setSql] = useState('SELECT * FROM users LIMIT 10');

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

    try {
      const info = extractConnectionInfo(shareUrl);
      if (!info) {
        setError('Could not parse share URL');
      } else {
        setConnectionInfo(info);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to parse share URL');
    } finally {
      setLoading(false);
    }
  }, [code, token]);

  const handleDeepLink = () => {
    // Open in Bennett Studio desktop app
    const deepLink = `bennett://share/${code}?t=${encodeURIComponent(token || '')}`;
    window.location.href = deepLink;
  };

  const handleQueryInBrowser = async () => {
    if (!shareUrl) return;
    setQueryLoading(true);
    setQueryResult(null);

    try {
      const client = clientFromUrl(shareUrl);
      const result = await client.query(sql);
      setQueryResult(result);
    } catch (e) {
      setQueryResult({ error: e instanceof Error ? e.message : 'Query failed' });
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

  const getModeBadge = () => {
    if (!connectionInfo) return null;
    const mode = connectionInfo.mode;
    if (mode === 'p2p') {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium" style={{ backgroundColor: 'rgba(0,212,170,0.15)', color: 'var(--accentSuccess)' }}>
          <Zap size={12} /> P2P Ready
        </span>
      );
    }
    if (mode === 'direct') {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium" style={{ backgroundColor: 'rgba(59,130,246,0.15)', color: '#3b82f6' }}>
          <Globe size={12} /> Direct
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
        Unknown Mode
      </span>
    );
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center" style={{ backgroundColor: 'var(--bgPrimary)' }}>
        <Loader2 size={32} className="animate-spin" style={{ color: 'var(--accentPrimary)' }} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center p-8" style={{ backgroundColor: 'var(--bgPrimary)' }}>
        <div className="max-w-md w-full p-6 rounded-2xl text-center" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <AlertCircle size={48} className="mx-auto mb-4" style={{ color: 'var(--accentError)' }} />
          <h1 className="text-xl font-bold mb-2" style={{ color: 'var(--textPrimary)' }}>Invalid Share Link</h1>
          <p className="text-sm" style={{ color: 'var(--textSecondary)' }}>{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen p-8" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <div className="text-center mb-10">
          <div className="w-20 h-20 rounded-2xl flex items-center justify-center mx-auto mb-4" style={{ backgroundColor: 'rgba(0,212,170,0.1)' }}>
            <Database size={40} style={{ color: 'var(--accentSuccess)' }} />
          </div>
          <h1 className="text-3xl font-bold mb-2" style={{ color: 'var(--textPrimary)' }}>
            Shared Database
          </h1>
          <p className="text-sm mb-4" style={{ color: 'var(--textSecondary)' }}>
            Connect to <code className="px-2 py-1 rounded text-xs" style={{ backgroundColor: 'var(--bgTertiary)' }}>{connectionInfo?.dbId || code}</code>
          </p>
          <div className="flex items-center justify-center gap-2 flex-wrap">
            {getModeBadge()}
            <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              <Lock size={12} /> {connectionInfo?.permission || 'ro'}
            </span>
            <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
              <Clock size={12} />
              {connectionInfo?.expiresAt
                ? `Expires ${new Date(connectionInfo.expiresAt * 1000).toLocaleString()}`
                : 'No expiry'}
            </span>
          </div>
        </div>

        {/* Action Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-8">
          {/* Deep Link to Desktop */}
          <button
            onClick={handleDeepLink}
            className="p-6 rounded-2xl text-left transition-all hover:opacity-90 group"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="flex items-start justify-between mb-4">
              <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ backgroundColor: 'rgba(0,212,170,0.1)' }}>
                <ExternalLink size={24} style={{ color: 'var(--accentSuccess)' }} />
              </div>
              <ChevronRight size={20} className="opacity-0 group-hover:opacity-100 transition-opacity" style={{ color: 'var(--textMuted)' }} />
            </div>
            <h3 className="font-semibold mb-1" style={{ color: 'var(--textPrimary)' }}>Open in Bennett Studio</h3>
            <p className="text-sm" style={{ color: 'var(--textSecondary)' }}>
              Connect with the desktop app for full features — schema browser, query editor, export, and more.
            </p>
          </button>

          {/* Query in Browser */}
          <button
            onClick={handleQueryInBrowser}
            className="p-6 rounded-2xl text-left transition-all hover:opacity-90 group"
            style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}
          >
            <div className="flex items-start justify-between mb-4">
              <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ backgroundColor: 'rgba(59,130,246,0.1)' }}>
                <Play size={24} style={{ color: '#3b82f6' }} />
              </div>
              <ChevronRight size={20} className="opacity-0 group-hover:opacity-100 transition-opacity" style={{ color: 'var(--textMuted)' }} />
            </div>
            <h3 className="font-semibold mb-1" style={{ color: 'var(--textPrimary)' }}>Query in Browser</h3>
            <p className="text-sm" style={{ color: 'var(--textSecondary)' }}>
              Run SQL directly in your browser. No installation needed — works on any device.
            </p>
          </button>
        </div>

        {/* Query Editor */}
        <div className="mb-8 p-6 rounded-2xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <h3 className="font-semibold mb-4 flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
            <Play size={18} style={{ color: 'var(--accentPrimary)' }} /> SQL Query
          </h3>
          <div className="flex gap-3">
            <input
              type="text"
              value={sql}
              onChange={(e) => setSql(e.target.value)}
              className="input flex-1 font-mono text-sm"
              placeholder="SELECT * FROM users LIMIT 10"
            />
            <button
              onClick={handleQueryInBrowser}
              disabled={queryLoading}
              className="btn-primary px-4 py-2 rounded-xl flex items-center gap-2 disabled:opacity-50"
            >
              {queryLoading ? <Loader2 size={16} className="animate-spin" /> : <Play size={16} />}
              Run
            </button>
          </div>

          {queryResult && (
            <div className="mt-4">
              {queryResult.error ? (
                <div className="p-4 rounded-xl" style={{ backgroundColor: 'rgba(255,68,68,0.1)', border: '1px solid var(--accentError)' }}>
                  <p className="text-sm" style={{ color: 'var(--accentError)' }}>{queryResult.error}</p>
                </div>
              ) : (
                <div>
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>
                      {queryResult.rowCount} rows · {queryResult.executionTimeMs}ms
                    </span>
                  </div>
                  <div className="overflow-x-auto">
                    <table className="w-full text-sm">
                      <thead>
                        <tr style={{ borderBottom: '1px solid var(--borderDefault)' }}>
                          {queryResult.columns?.map((col: string) => (
                            <th key={col} className="text-left px-3 py-2 font-medium" style={{ color: 'var(--textSecondary)' }}>{col}</th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {queryResult.rows?.map((row: any[], i: number) => (
                          <tr key={i} style={{ borderBottom: '1px solid var(--borderSubtle)' }}>
                            {row.map((cell, j) => (
                              <td key={j} className="px-3 py-2 font-mono text-xs" style={{ color: 'var(--textPrimary)' }}>
                                {JSON.stringify(cell)}
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
        <div className="p-6 rounded-2xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold flex items-center gap-2" style={{ color: 'var(--textPrimary)' }}>
              <Table2 size={18} style={{ color: 'var(--accentPrimary)' }} /> Schema Preview
            </h3>
            <button
              onClick={handleFetchSchema}
              disabled={schemaLoading}
              className="text-sm px-3 py-1.5 rounded-lg transition-all hover:opacity-80"
              style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}
            >
              {schemaLoading ? <Loader2 size={14} className="animate-spin inline" /> : 'Refresh'}
            </button>
          </div>

          {schema === null ? (
            <p className="text-sm text-center py-8" style={{ color: 'var(--textMuted)' }}>
              Click Refresh to load schema
            </p>
          ) : schema.length === 0 ? (
            <p className="text-sm text-center py-8" style={{ color: 'var(--textMuted)' }}>
              No tables found or schema unavailable
            </p>
          ) : (
            <div className="space-y-3">
              {schema.map((table: any) => (
                <div key={table.name} className="p-3 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
                  <div className="flex items-center gap-2 mb-2">
                    <Table2 size={14} style={{ color: 'var(--accentPrimary)' }} />
                    <span className="font-medium text-sm" style={{ color: 'var(--textPrimary)' }}>{table.name}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>
                      {table.columns?.length || 0} columns
                    </span>
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {table.columns?.slice(0, 5).map((col: any) => (
                      <span key={col.name} className="text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
                        {col.name} <span style={{ color: 'var(--textMuted)' }}>{col.dataType}</span>
                      </span>
                    ))}
                    {table.columns?.length > 5 && (
                      <span className="text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
                        +{table.columns.length - 5} more
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
