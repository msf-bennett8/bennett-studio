import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Globe, Link2, AlertCircle, Loader2, ArrowLeft, CheckCircle, Database, Lock, Unlock, Clock } from 'lucide-react';
import { useRemoteConnectionStore } from '../stores/remoteConnectionStore';

export function JoinSharePage() {
  const navigate = useNavigate();
  const { validateUrl, connect, isConnecting, connectionError, clearError } = useRemoteConnectionStore();
  
  const [url, setUrl] = useState('');
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{
    valid: boolean;
    dbName?: string;
    permission?: string;
    tables?: string[];
    expiresAt?: string;
  } | null>(null);

  const handleValidate = async () => {
    if (!url.trim()) return;
    
    clearError();
    setIsValidating(true);
    setValidationResult(null);
    
    try {
      const result = await validateUrl(url.trim());
      setValidationResult({
        valid: true,
        dbName: result.db_id,
        permission: result.permission,
        tables: result.tables,
        expiresAt: result.expires_at,
      });
    } catch (err) {
      setValidationResult({
        valid: false,
      });
    } finally {
      setIsValidating(false);
    }
  };

  const handleConnect = async () => {
    if (!validationResult?.valid) return;

    try {
      await connect(url.trim());
      navigate('/remote-query');
    } catch {
      // Error handled in store
    }
  };

  const getPermissionIcon = (perm?: string) => {
    switch (perm) {
      case 'ro': return <Lock size={16} />;
      case 'rw': return <Unlock size={16} />;
      case 'adm': return <Database size={16} />;
      default: return <Lock size={16} />;
    }
  };

  const getPermissionLabel = (perm?: string) => {
    switch (perm) {
      case 'ro': return 'Read-only';
      case 'rw': return 'Read-write';
      case 'adm': return 'Admin';
      default: return perm || 'Unknown';
    }
  };

  return (
    <div className="p-8 max-w-2xl mx-auto">
      <button 
        onClick={() => navigate('/')} 
        className="flex items-center gap-2 text-sm mb-6 hover:opacity-80 transition-opacity"
        style={{ color: 'var(--textSecondary)' }}
      >
        <ArrowLeft size={16} /> Back to Home
      </button>

      <div className="text-center mb-8">
        <div className="w-16 h-16 rounded-2xl flex items-center justify-center mx-auto mb-4" style={{ backgroundColor: 'rgba(0,212,170,0.1)' }}>
          <Globe size={32} style={{ color: 'var(--accentSuccess)' }} />
        </div>
        <h1 className="text-3xl font-bold mb-2" style={{ color: 'var(--textPrimary)' }}>Join Shared Database</h1>
        <p style={{ color: 'var(--textSecondary)' }}>Enter a share link to connect to a remote database</p>
      </div>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium mb-2" style={{ color: 'var(--textSecondary)' }}>
            Share URL
          </label>
          <div className="relative">
            <Link2 size={16} className="absolute left-3 top-1/2 -translate-y-1/2" style={{ color: 'var(--textMuted)' }} />
            <input
              type="text"
              value={url}
              onChange={(e) => {
                setUrl(e.target.value);
                setValidationResult(null);
                clearError();
              }}
              placeholder="https://share.bennett.studio/db/ACQPFDAQ7P?t=eyJhbG..."
              className="input w-full pl-10"
              disabled={isConnecting || isValidating}
            />
          </div>
          <p className="text-xs mt-1" style={{ color: 'var(--textMuted)' }}>
            Paste the full share link including the token
          </p>
        </div>

        {connectionError && (
          <div className="p-4 rounded-xl flex items-center gap-3" style={{ backgroundColor: 'rgba(255,68,68,0.1)', border: '1px solid var(--accentError)' }}>
            <AlertCircle size={20} style={{ color: 'var(--accentError)' }} />
            <p className="text-sm" style={{ color: 'var(--accentError)' }}>{connectionError}</p>
          </div>
        )}

        {validationResult && (
          <div className={`p-4 rounded-xl border ${validationResult.valid ? 'border-green-500/30' : 'border-red-500/30'}`} 
            style={{ backgroundColor: validationResult.valid ? 'rgba(0,212,170,0.05)' : 'rgba(255,68,68,0.05)' }}>
            <div className="flex items-center gap-3">
              {validationResult.valid ? (
                <CheckCircle size={20} style={{ color: 'var(--accentSuccess)' }} />
              ) : (
                <AlertCircle size={20} style={{ color: 'var(--accentError)' }} />
              )}
              <div>
                <p className="font-medium" style={{ color: 'var(--textPrimary)' }}>
                  {validationResult.valid ? 'Share link is valid' : 'Invalid share link'}
                </p>
                {validationResult.valid && (
                  <div className="mt-2 space-y-1 text-sm" style={{ color: 'var(--textSecondary)' }}>
                    <div className="flex items-center gap-2">
                      <Database size={14} />
                      <span>Database: {validationResult.dbName}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      {getPermissionIcon(validationResult.permission)}
                      <span>Permission: {getPermissionLabel(validationResult.permission)}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Clock size={14} />
                      <span>Expires: {validationResult.expiresAt ? new Date(validationResult.expiresAt).toLocaleString() : 'Unknown'}</span>
                    </div>
                    {validationResult.tables && validationResult.tables.length > 0 && validationResult.tables[0] !== '*' && (
                      <div className="flex items-center gap-2 flex-wrap">
                        <span>Tables:</span>
                        {validationResult.tables.map(t => (
                          <span key={t} className="text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)' }}>
                            {t}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        <div className="flex gap-3">
          <button
            onClick={handleValidate}
            disabled={!url.trim() || isValidating || isConnecting}
            className="btn-secondary flex-1 py-3 rounded-xl flex items-center justify-center gap-2 disabled:opacity-50"
          >
            {isValidating ? <Loader2 size={16} className="animate-spin" /> : <CheckCircle size={16} />}
            {isValidating ? 'Validating...' : 'Validate Link'}
          </button>
          
          <button
            onClick={handleConnect}
            disabled={!validationResult?.valid || isConnecting}
            className="btn-primary flex-1 py-3 rounded-xl flex items-center justify-center gap-2 disabled:opacity-50"
          >
            {isConnecting ? <Loader2 size={16} className="animate-spin" /> : <Globe size={16} />}
            {isConnecting ? 'Connecting...' : 'Connect'}
          </button>
        </div>
      </div>

      <div className="mt-8 p-4 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
        <h3 className="font-medium mb-2" style={{ color: 'var(--textPrimary)' }}>What you can do</h3>
        <ul className="space-y-2 text-sm" style={{ color: 'var(--textSecondary)' }}>
          <li className="flex items-center gap-2">
            <Database size={14} style={{ color: 'var(--accentPrimary)' }} />
            Browse database schema and tables
          </li>
          <li className="flex items-center gap-2">
            <Unlock size={14} style={{ color: 'var(--accentPrimary)' }} />
            Run SQL queries with autocomplete
          </li>
          <li className="flex items-center gap-2">
            <Lock size={14} style={{ color: 'var(--accentWarning)' }} />
            Permissions are enforced by the host
          </li>
        </ul>
      </div>
    </div>
  );
}
