import { useState, useEffect } from 'react';
import { X, Lock, Unlock, KeyRound, Database, FileText } from 'lucide-react';

interface UnlockDatabaseModalProps {
  databaseId: string;
  databaseName: string;
  onClose: () => void;
  onUnlock: (id: string, username: string, password: string, database: string) => void;
  envSuggestions?: Array<{
    source: string;
    username?: string;
    password?: string;
    database?: string;
    host?: string;
    port?: string;
  }>;
}

export function UnlockDatabaseModal({
  databaseId,
  databaseName,
  onClose,
  onUnlock,
  envSuggestions = [],
}: UnlockDatabaseModalProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [database, setDatabase] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);

  // Auto-fill from first env suggestion if available
  useEffect(() => {
    if (envSuggestions.length > 0) {
      const first = envSuggestions[0];
      if (first.username) setUsername(first.username);
      if (first.password) setPassword(first.password || '');
      if (first.database) setDatabase(first.database);
    }
  }, [envSuggestions]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    
    const dbName = database || databaseName;
    const success = await onUnlock(databaseId, username, password, dbName);
    
    if (!success) {
      setError('Authentication failed. Please check your credentials.');
    }
    setLoading(false);
  };

  const applySuggestion = (suggestion: typeof envSuggestions[0]) => {
    if (suggestion.username) setUsername(suggestion.username);
    if (suggestion.password) setPassword(suggestion.password || '');
    if (suggestion.database) setDatabase(suggestion.database);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ backgroundColor: 'var(--bgOverlay)' }}>
      <div className="w-full max-w-md p-6 rounded-2xl shadow-2xl" style={{ backgroundColor: 'var(--bgElevated)', border: '1px solid var(--borderDefault)' }}>
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl flex items-center justify-center" style={{ backgroundColor: 'var(--accentWarning)' }}>
              <Lock size={20} style={{ color: 'var(--textInverse)' }} />
            </div>
            <div>
              <h2 className="text-xl font-bold" style={{ color: 'var(--textPrimary)' }}>Unlock Database</h2>
              <p className="text-sm" style={{ color: 'var(--textSecondary)' }}>{databaseName}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-lg transition-all hover:opacity-80"
            style={{ backgroundColor: 'var(--bgTertiary)' }}
          >
            <X size={18} style={{ color: 'var(--textSecondary)' }} />
          </button>
        </div>

        {/* Env Suggestions */}
        {envSuggestions.length > 0 && (
          <div className="mb-4 p-3 rounded-xl" style={{ backgroundColor: 'var(--bgTertiary)', border: '1px solid var(--borderDefault)' }}>
            <div className="flex items-center gap-2 mb-2">
              <FileText size={14} style={{ color: 'var(--accentInfo)' }} />
              <span className="text-xs font-semibold" style={{ color: 'var(--textSecondary)' }}>Auto-detected credentials from .env files</span>
            </div>
            <div className="space-y-2">
              {envSuggestions.map((sugg, idx) => (
                <button
                  key={idx}
                  onClick={() => applySuggestion(sugg)}
                  className="w-full text-left p-2 rounded-lg text-xs transition-all hover:opacity-80"
                  style={{ backgroundColor: 'var(--bgSecondary)' }}
                >
                  <div className="font-medium" style={{ color: 'var(--textPrimary)' }}>{sugg.source}</div>
                  <div className="mt-1" style={{ color: 'var(--textMuted)' }}>
                    User: {sugg.username || '—'} | DB: {sugg.database || '—'}
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm mb-2 flex items-center gap-2" style={{ color: 'var(--textSecondary)' }}>
              <KeyRound size={14} /> Username
            </label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="e.g., stacy_bennett_adams"
              className="input w-full"
              required
              disabled={loading}
            />
          </div>

          <div>
            <label className="block text-sm mb-2 flex items-center gap-2" style={{ color: 'var(--textSecondary)' }}>
              <KeyRound size={14} /> Password
            </label>
            <div className="relative">
              <input
                type={showPassword ? 'text' : 'password'}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter password"
                className="input w-full pr-10"
                required
                disabled={loading}
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-xs"
                style={{ color: 'var(--textMuted)' }}
              >
                {showPassword ? 'Hide' : 'Show'}
              </button>
            </div>
          </div>

          <div>
            <label className="block text-sm mb-2 flex items-center gap-2" style={{ color: 'var(--textSecondary)' }}>
              <Database size={14} /> Database Name
            </label>
            <input
              type="text"
              value={database}
              onChange={(e) => setDatabase(e.target.value)}
              placeholder="{e.g., oshocks_local"
              className="input w-full"
              required
              disabled={loading}
            />
          </div>

          {error && (
            <div className="p-3 rounded-xl text-sm" style={{ backgroundColor: 'rgba(255,68,68,0.1)', color: 'var(--accentError)', border: '1px solid var(--accentError)' }}>
              {error}
            </div>
          )}

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="btn-secondary flex-1 py-2 rounded-xl disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!username || !password || loading}
              className="btn-primary flex-1 py-2 rounded-xl flex items-center justify-center gap-2 disabled:opacity-50"
            >
              {loading ? (
                <div className="w-4 h-4 border-2 border-t-transparent rounded-full animate-spin" />
              ) : (
                <>
                  <Unlock size={16} /> Unlock
                </>
              )}
            </button>
          </div>
        </form>

        {/* Security Note */}
        <p className="text-xs mt-4 text-center" style={{ color: 'var(--textMuted)' }}>
          Credentials are stored in memory only and cleared when the app closes.
        </p>
      </div>
    </div>
  );
}
