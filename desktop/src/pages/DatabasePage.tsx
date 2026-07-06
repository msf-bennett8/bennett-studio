import { useState, useEffect } from 'react';
import { Database, Plus, Play, Square, Trash2, RefreshCw, CheckCircle, XCircle, Clock, AlertCircle, FolderOpen, ScanLine, Lock, Unlock, Globe } from 'lucide-react';
import { useRemoteConnectionStore } from '../stores/remoteConnectionStore';
import { useDatabaseStore } from '../stores/databaseStore';
import { UnlockDatabaseModal } from '../components/database/UnlockDatabaseModal';

const dbTypes = [
  { id: 'postgres', name: 'PostgreSQL', versions: ['16.2', '15.6', '14.11'] },
  { id: 'mysql', name: 'MySQL', versions: ['8.0', '8.4'] },
  { id: 'mariadb', name: 'MariaDB', versions: ['11.2', '10.11'] },
  { id: 'sqlite', name: 'SQLite', versions: ['3.45'] },
  { id: 'redis', name: 'Redis', versions: ['7.2', '7.0'] },
];

export function DatabasePage() {
  const {
    databases, loading, error, logs, clearError, unlockedDatabases,
    fetchDatabases, discoverLocalDatabases, createDatabase, deleteDatabase,
    startDatabase, stopDatabase, unlockDatabase, scanEnvFiles,
    getRemoteDatabases,
  } = useDatabaseStore();

  const { disconnect: disconnectRemote } = useRemoteConnectionStore();
  const allDatabases = [...databases, ...getRemoteDatabases()];

  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedType, setSelectedType] = useState('postgres');
  const [selectedVersion, setSelectedVersion] = useState('16.2');
  const [dbName, setDbName] = useState('');
  const [showUnlockModal, setShowUnlockModal] = useState<string | null>(null);

  useEffect(() => {
    fetchDatabases();
    const interval = setInterval(fetchDatabases, 3000);
    return () => clearInterval(interval);
  }, []);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'running': return <CheckCircle size={16} style={{ color: 'var(--accentSuccess)' }} />;
      case 'stopped': return <XCircle size={16} style={{ color: 'var(--accentError)' }} />;
      case 'starting': return <Clock size={16} style={{ color: 'var(--accentWarning)' }} />;
      case 'error': return <AlertCircle size={16} style={{ color: 'var(--accentError)' }} />;
      default: return <XCircle size={16} style={{ color: 'var(--accentError)' }} />;
    }
  };

  const handleAddDatabase = async () => {
    if (!dbName) return;
    await createDatabase({ name: dbName, type: selectedType, version: selectedVersion });
    setShowAddModal(false);
    setDbName('');
  };

  const handleDelete = async (id: string) => {
    await deleteDatabase(id);
  };

  const handleUnlock = async (id: string, username: string, password: string, database: string): Promise<boolean> => {
    try {
      const success = await unlockDatabase(id, username, password, database);
      if (success) {
        setShowUnlockModal(null);
      }
      return !!success;
    } catch {
      return false;
    }
  };

  const handleOpenUnlock = (id: string) => {
    scanEnvFiles(id);
    setShowUnlockModal(id);
  };

  const handleToggle = async (id: string, status: string) => {
    if (status === 'running') {
      await stopDatabase(id);
    } else {
      await startDatabase(id);
    }
  };

  const handleOpenFolder = (id: string) => {
    // @ts-ignore
    if (window.__TAURI__) {
      // @ts-ignore
      window.__TAURI__.shell.open(`~/studio.dev/bennett studio/data/${id}`);
    }
  };

  return (
    <div className="flex h-full">
      <div className="flex-1 p-8 max-w-6xl mx-auto overflow-auto">
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold" style={{ color: 'var(--textPrimary)' }}>Databases</h1>
            <p className="text-sm mt-1" style={{ color: 'var(--textSecondary)' }}>
              {loading ? 'Loading...' : `${databases.length} instance${databases.length !== 1 ? 's' : ''}`}
            </p>
          </div>
          <div className="flex items-center gap-3">
            <button onClick={fetchDatabases} className="p-2 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Refresh">
              <RefreshCw size={18} style={{ color: 'var(--textSecondary)' }} />
            </button>
            <button onClick={discoverLocalDatabases} className="btn-secondary flex items-center gap-2 px-4 py-2 rounded-xl" disabled={loading}>
              <ScanLine size={18} /> Discover Local
            </button>
            <button onClick={() => setShowAddModal(true)} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
              <Plus size={18} /> Add Database
            </button>
          </div>
        </div>

        {error && (
          <div className="mb-6 p-4 rounded-xl flex items-center gap-3" style={{ backgroundColor: 'rgba(255,68,68,0.1)', border: '1px solid var(--accentError)' }}>
            <AlertCircle size={20} style={{ color: 'var(--accentError)' }} />
            <div className="flex-1">
              <p style={{ color: 'var(--accentError)' }}>{error}</p>
            </div>
            <button onClick={clearError} className="text-sm" style={{ color: 'var(--accentError)' }}>Dismiss</button>
          </div>
        )}

        <div className="space-y-4">
          {allDatabases.map((db) => (
            <div key={db.id} className="card p-6 rounded-xl flex items-center justify-between" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ backgroundColor: db.isRemote ? 'rgba(107,138,255,0.1)' : 'var(--bgTertiary)' }}>
                  {db.isRemote ? <Globe size={24} style={{ color: 'var(--accentSecondary)' }} /> : <Database size={24} style={{ color: 'var(--accentPrimary)' }} />}
                </div>
                <div>
                  <h3 className="font-semibold" style={{ color: 'var(--textPrimary)' }}>{db.name}</h3>
                  <div className="flex items-center gap-2 mt-1">
                  {db.isRemote ? (
                    <>
                      <span className="text-xs px-2 py-1 rounded-full font-medium" style={{ backgroundColor: 'var(--accentSecondary)', color: 'var(--textInverse)' }}>
                        Remote
                      </span>
                      <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
                        {db.shareCode}
                      </span>
                      <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: db.remotePermission === 'ro' ? 'rgba(255,170,0,0.2)' : 'rgba(0,212,170,0.2)', color: db.remotePermission === 'ro' ? 'var(--accentWarning)' : 'var(--accentSuccess)' }}>
                        {db.remotePermission === 'ro' ? 'Read-only' : 'Read-write'}
                      </span>
                    </>
                  ) : (
                    <>
                      <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>
                        {db.type} {db.version}
                      </span>
                      {db.source === 'bennett' && (
                        <span className="text-xs px-2 py-0.5 rounded-full font-medium" style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}>
                          BnT
                        </span>
                      )}
                      {(db.source === 'local' || db.is_discovered) && (
                        <span className="text-xs px-2 py-0.5 rounded-full font-medium" style={{ backgroundColor: 'var(--accentInfo)', color: 'var(--textInverse)' }}>
                          Local
                        </span>
                      )}
                      {(db.source === 'local' || db.is_discovered) && !unlockedDatabases.has(db.id) && (
                        <span className="text-xs px-2 py-0.5 rounded-full font-medium flex items-center gap-1" style={{ backgroundColor: 'var(--accentWarning)', color: 'var(--textInverse)' }}>
                          <Lock size={10} /> Locked
                        </span>
                      )}
                      {(db.source === 'local' || db.is_discovered) && unlockedDatabases.has(db.id) && (
                        <span className="text-xs px-2 py-0.5 rounded-full font-medium flex items-center gap-1" style={{ backgroundColor: 'var(--accentSuccess)', color: 'var(--textInverse)' }}>
                          <Unlock size={10} /> Unlocked
                        </span>
                      )}
                      <span className="text-xs" style={{ color: 'var(--textMuted)' }}>port:{db.port}</span>
                      <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{db.size}</span>
                      {db.container_id && (
                        <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>{db.container_id}</span>
                      )}
                    </>
                  )}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                {db.isRemote ? (
                  <>
                    <div className="flex items-center gap-2 px-3 py-1 rounded-full" style={{ backgroundColor: 'rgba(0,212,170,0.1)' }}>
                      <div className="w-2 h-2 rounded-full" style={{ backgroundColor: 'var(--accentSuccess)' }} />
                      <span className="text-xs font-medium" style={{ color: 'var(--accentSuccess)' }}>connected</span>
                    </div>
                    <button
                      onClick={() => disconnectRemote(db.id)}
                      className="p-2 rounded-lg transition-all hover:bg-red-500/20"
                      style={{ backgroundColor: 'var(--bgTertiary)' }}
                      title="Disconnect"
                    >
                      <XCircle size={16} style={{ color: 'var(--accentError)' }} />
                    </button>
                  </>
                ) : (
                  <>
                    {(db.source === 'local' || db.is_discovered) && !unlockedDatabases.has(db.id) && (
                      <button
                        onClick={() => handleOpenUnlock(db.id)}
                        className="p-2 rounded-lg transition-all"
                        style={{ backgroundColor: 'var(--accentWarning)', color: 'var(--textInverse)' }}
                        title="Unlock database"
                      >
                        <Lock size={16} />
                      </button>
                    )}
                    <div className="flex items-center gap-2 px-3 py-1 rounded-full" style={{
                      backgroundColor: db.status === 'running' ? 'rgba(0,212,170,0.1)' :
                        db.status === 'starting' ? 'rgba(255,170,0,0.1)' : 'rgba(255,68,68,0.1)'
                    }}>
                      {getStatusIcon(db.status)}
                      <span className="text-xs font-medium" style={{
                        color: db.status === 'running' ? 'var(--accentSuccess)' :
                          db.status === 'starting' ? 'var(--accentWarning)' : 'var(--accentError)'
                      }}>
                        {db.status}
                      </span>
                    </div>
                    <button
                      onClick={() => handleToggle(db.id, db.status)}
                      disabled={db.status === 'starting' || loading || db.source === 'local'}
                      className="p-2 rounded-lg transition-all disabled:opacity-50"
                      style={{ backgroundColor: 'var(--bgTertiary)' }}
                      title={db.status === 'running' ? 'Stop' : 'Start'}
                    >
                      {db.status === 'running' ? <Square size={16} /> : <Play size={16} />}
                    </button>
                    {db.source !== 'local' && (
                      <button
                        onClick={() => handleOpenFolder(db.id)}
                        className="p-2 rounded-lg transition-all"
                        style={{ backgroundColor: 'var(--bgTertiary)' }}
                        title="Open data folder"
                      >
                        <FolderOpen size={16} />
                      </button>
                    )}
                    <button
                      onClick={() => handleDelete(db.id)}
                      disabled={loading || db.source === 'local'}
                      className="p-2 rounded-lg transition-all hover:bg-red-500/20 disabled:opacity-50"
                      style={{ backgroundColor: 'var(--bgTertiary)' }}
                      title="Delete"
                    >
                      <Trash2 size={16} style={{ color: 'var(--accentError)' }} />
                    </button>
                  </>
                )}
              </div>
            </div>
          ))}

          {allDatabases.length === 0 && !loading && (
            <div className="text-center py-16 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px dashed var(--borderDefault)' }}>
              <Database size={48} className="mx-auto mb-4" style={{ color: 'var(--textMuted)' }} />
              <p style={{ color: 'var(--textSecondary)' }}>No databases yet</p>
              <p className="text-sm mt-1" style={{ color: 'var(--textMuted)' }}>Click "Add Database" to get started</p>
            </div>
          )}

          {loading && allDatabases.length === 0 && (
            <div className="text-center py-16">
              <div className="w-8 h-8 border-2 border-t-transparent rounded-full animate-spin mx-auto" style={{ borderColor: 'var(--accentPrimary)' }} />
              <p className="mt-4 text-sm" style={{ color: 'var(--textSecondary)' }}>Loading databases...</p>
            </div>
          )}
        </div>

        {showAddModal && (
          <div className="fixed inset-0 flex items-center justify-center z-50" style={{ backgroundColor: 'var(--bgOverlay)' }}>
            <div className="w-full max-w-md p-6 rounded-2xl" style={{ backgroundColor: 'var(--bgElevated)', border: '1px solid var(--borderDefault)' }}>
              <h2 className="text-xl font-bold mb-6" style={{ color: 'var(--textPrimary)' }}>Add Database</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database Name</label>
                  <input 
                    type="text" 
                    value={dbName} 
                    onChange={(e) => setDbName(e.target.value)} 
                    placeholder="e.g., my-project-db" 
                    className="input"
                    disabled={loading}
                  />
                </div>
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database Type</label>
                  <div className="grid grid-cols-2 gap-2">
                    {dbTypes.map((type) => (
                      <button 
                        key={type.id} 
                        onClick={() => { setSelectedType(type.id); setSelectedVersion(type.versions[0]); }}
                        disabled={loading}
                        className="p-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50"
                        style={{ 
                          backgroundColor: selectedType === type.id ? 'var(--accentPrimary)' : 'var(--bgTertiary)', 
                          color: selectedType === type.id ? 'var(--textInverse)' : 'var(--textSecondary)' 
                        }}
                      >
                        {type.name}
                      </button>
                    ))}
                  </div>
                </div>
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Version</label>
                  <div className="flex gap-2">
                    {dbTypes.find(t => t.id === selectedType)?.versions.map((version) => (
                      <button 
                        key={version} 
                        onClick={() => setSelectedVersion(version)}
                        disabled={loading}
                        className="px-4 py-2 rounded-xl text-sm font-medium transition-all disabled:opacity-50"
                        style={{ 
                          backgroundColor: selectedVersion === version ? 'var(--accentPrimary)' : 'var(--bgTertiary)', 
                          color: selectedVersion === version ? 'var(--textInverse)' : 'var(--textSecondary)' 
                        }}
                      >
                        {version}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
              <div className="flex gap-3 mt-6">
                <button 
                  onClick={() => setShowAddModal(false)} 
                  disabled={loading}
                  className="btn-secondary flex-1 py-2 rounded-xl disabled:opacity-50"
                >
                  Cancel
                </button>
                <button 
                  onClick={handleAddDatabase} 
                  disabled={!dbName || loading}
                  className="btn-primary flex-1 py-2 rounded-xl disabled:opacity-50"
                >
                  {loading ? 'Creating...' : 'Add Database'}
                </button>
              </div>
            </div>
          </div>
        )}
        {/* Unlock Modal */}
        {showUnlockModal && (
          <UnlockDatabaseModal
            databaseId={showUnlockModal}
            databaseName={databases.find(d => d.id === showUnlockModal)?.name || ''}
            onClose={() => setShowUnlockModal(null)}
            onUnlock={handleUnlock}
            envSuggestions={useDatabaseStore.getState().envSuggestions[showUnlockModal] || []}
          />
        )}
      </div>
      <div className="w-80 border-l flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b flex items-center justify-between" style={{ borderColor: 'var(--borderDefault)' }}>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Engine Logs</h3>
          <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
            {logs.length} entries
          </span>
        </div>
        <div className="flex-1 overflow-auto p-3 space-y-1 font-mono text-xs">
          {logs.length === 0 && (
            <p style={{ color: 'var(--textMuted)' }}>No logs yet...</p>
          )}
          {logs.map((log, index) => (
            <div key={index} className="py-1" style={{ color: 'var(--textSecondary)' }}>
              <span style={{ color: 'var(--accentPrimary)' }}>$</span> {log}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
