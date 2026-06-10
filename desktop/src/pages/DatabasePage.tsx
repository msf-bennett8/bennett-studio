import { useState, useEffect } from 'react';
import { Database, Plus, Play, Trash2, RefreshCw, CheckCircle, XCircle, Clock, FolderOpen } from 'lucide-react';

interface DatabaseInstance {
  id: string; name: string; type: 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis';
  version: string; status: 'running' | 'stopped' | 'error' | 'starting';
  port: number; size: string; createdAt: string; containerId?: string;
}

const mockDatabases: DatabaseInstance[] = [
  { id: '1', name: 'local-postgres', type: 'postgres', version: '16.2', status: 'running', port: 5433, size: '245 MB', createdAt: '2024-06-10', containerId: 'pg-16-local' },
  { id: '2', name: 'dev-mysql', type: 'mysql', version: '8.0', status: 'stopped', port: 3307, size: '128 MB', createdAt: '2024-06-09' },
];

const dbTypes = [
  { id: 'postgres', name: 'PostgreSQL', versions: ['16.2', '15.6', '14.11'] },
  { id: 'mysql', name: 'MySQL', versions: ['8.0', '8.4'] },
  { id: 'mariadb', name: 'MariaDB', versions: ['11.2', '10.11'] },
  { id: 'sqlite', name: 'SQLite', versions: ['3.45'] },
  { id: 'redis', name: 'Redis', versions: ['7.2', '7.0'] },
];

export function DatabasePage() {
  const [databases, setDatabases] = useState<DatabaseInstance[]>(mockDatabases);
  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedType, setSelectedType] = useState('postgres');
  const [selectedVersion, setSelectedVersion] = useState('16.2');
  const [dbName, setDbName] = useState('');
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = (message: string) => {
    setLogs(prev => [...prev.slice(-50), `[${new Date().toLocaleTimeString()}] ${message}`]);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'running': return <CheckCircle size={16} style={{ color: 'var(--accentSuccess)' }} />;
      case 'stopped': return <XCircle size={16} style={{ color: 'var(--accentError)' }} />;
      case 'starting': return <Clock size={16} style={{ color: 'var(--accentWarning)' }} />;
      default: return <XCircle size={16} style={{ color: 'var(--accentError)' }} />;
    }
  };

  const handleAddDatabase = () => {
    if (!dbName) return;
    addLog(`Initializing ${selectedType} ${selectedVersion} container...`);
    const newDb: DatabaseInstance = {
      id: Date.now().toString(), name: dbName, type: selectedType as any, version: selectedVersion,
      status: 'starting', port: 5432 + databases.length + 1, size: '0 MB', createdAt: new Date().toISOString().split('T')[0],
    };
    setDatabases([...databases, newDb]);
    setShowAddModal(false); setDbName('');
    addLog(`Pulling ${selectedType}:${selectedVersion}-alpine image...`);
    setTimeout(() => {
      addLog(`Container created. Starting health checks...`);
      setDatabases(prev => prev.map(db => db.id === newDb.id ? { ...db, status: 'running', containerId: `${selectedType}-${selectedVersion}-${dbName}` } : db));
      addLog(`Database ${dbName} is ready on port ${newDb.port}`);
    }, 3000);
  };

  const handleDelete = (id: string) => {
    const db = databases.find(d => d.id === id);
    if (db) addLog(`Removing container ${db.containerId || db.name}...`);
    setDatabases(databases.filter(db => db.id !== id));
    addLog(`Database ${db?.name} removed`);
  };

  const handleToggle = (id: string) => {
    const db = databases.find(d => d.id === id);
    const isStarting = db?.status === 'stopped';
    setDatabases(databases.map(db => db.id === id ? { ...db, status: db.status === 'running' ? 'stopped' : 'starting' as any } : db));
    addLog(`${isStarting ? 'Starting' : 'Stopping'} ${db?.name}...`);
    setTimeout(() => {
      setDatabases(prev => prev.map(db => db.id === id ? { ...db, status: db.status === 'starting' ? 'running' : 'stopped' as any } : db));
      addLog(`${db?.name} ${isStarting ? 'started' : 'stopped'}`);
    }, 2000);
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
            <p className="text-sm mt-1" style={{ color: 'var(--textSecondary)' }}>Manage your local Docker database instances</p>
          </div>
          <button onClick={() => setShowAddModal(true)} className="btn-primary flex items-center gap-2 px-4 py-2 rounded-xl">
            <Plus size={18} /> Add Database
          </button>
        </div>

        <div className="space-y-4">
          {databases.map((db) => (
            <div key={db.id} className="card p-6 rounded-xl flex items-center justify-between" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-xl flex items-center justify-center" style={{ backgroundColor: 'var(--bgTertiary)' }}>
                  <Database size={24} style={{ color: 'var(--accentPrimary)' }} />
                </div>
                <div>
                  <h3 className="font-semibold" style={{ color: 'var(--textPrimary)' }}>{db.name}</h3>
                  <div className="flex items-center gap-2 mt-1">
                    <span className="text-xs px-2 py-1 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{db.type} {db.version}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>port:{db.port}</span>
                    <span className="text-xs" style={{ color: 'var(--textMuted)' }}>{db.size}</span>
                    {db.containerId && <span className="text-xs font-mono" style={{ color: 'var(--textMuted)' }}>{db.containerId}</span>}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <div className="flex items-center gap-2 px-3 py-1 rounded-full" style={{ backgroundColor: db.status === 'running' ? 'rgba(0,212,170,0.1)' : db.status === 'starting' ? 'rgba(255,170,0,0.1)' : 'rgba(255,68,68,0.1)' }}>
                  {getStatusIcon(db.status)}
                  <span className="text-xs font-medium" style={{ color: db.status === 'running' ? 'var(--accentSuccess)' : db.status === 'starting' ? 'var(--accentWarning)' : 'var(--accentError)' }}>{db.status}</span>
                </div>
                <button onClick={() => handleToggle(db.id)} className="p-2 rounded-lg transition-all" style={{ backgroundColor: 'var(--bgTertiary)' }} title={db.status === 'running' ? 'Stop' : 'Start'}>
                  {db.status === 'running' ? <RefreshCw size={16} /> : <Play size={16} />}
                </button>
                <button onClick={() => handleOpenFolder(db.id)} className="p-2 rounded-lg transition-all" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Open data folder">
                  <FolderOpen size={16} />
                </button>
                <button onClick={() => handleDelete(db.id)} className="p-2 rounded-lg transition-all hover:bg-red-500/20" style={{ backgroundColor: 'var(--bgTertiary)' }} title="Delete">
                  <Trash2 size={16} style={{ color: 'var(--accentError)' }} />
                </button>
              </div>
            </div>
          ))}
          {databases.length === 0 && (
            <div className="text-center py-16 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px dashed var(--borderDefault)' }}>
              <Database size={48} className="mx-auto mb-4" style={{ color: 'var(--textMuted)' }} />
              <p style={{ color: 'var(--textSecondary)' }}>No databases yet</p>
              <p className="text-sm mt-1" style={{ color: 'var(--textMuted)' }}>Click "Add Database" to get started</p>
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
                  <input type="text" value={dbName} onChange={(e) => setDbName(e.target.value)} placeholder="e.g., my-project-db" className="input" />
                </div>
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Database Type</label>
                  <div className="grid grid-cols-2 gap-2">
                    {dbTypes.map((type) => (
                      <button key={type.id} onClick={() => { setSelectedType(type.id); setSelectedVersion(type.versions[0]); }}
                        className="p-3 rounded-xl text-sm font-medium transition-all"
                        style={{ backgroundColor: selectedType === type.id ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: selectedType === type.id ? 'var(--textInverse)' : 'var(--textSecondary)' }}>
                        {type.name}
                      </button>
                    ))}
                  </div>
                </div>
                <div>
                  <label className="block text-sm mb-2" style={{ color: 'var(--textSecondary)' }}>Version</label>
                  <div className="flex gap-2">
                    {dbTypes.find(t => t.id === selectedType)?.versions.map((version) => (
                      <button key={version} onClick={() => setSelectedVersion(version)} className="px-4 py-2 rounded-xl text-sm font-medium transition-all"
                        style={{ backgroundColor: selectedVersion === version ? 'var(--accentPrimary)' : 'var(--bgTertiary)', color: selectedVersion === version ? 'var(--textInverse)' : 'var(--textSecondary)' }}>{version}</button>
                    ))}
                  </div>
                </div>
              </div>
              <div className="flex gap-3 mt-6">
                <button onClick={() => setShowAddModal(false)} className="btn-secondary flex-1 py-2 rounded-xl">Cancel</button>
                <button onClick={handleAddDatabase} className="btn-primary flex-1 py-2 rounded-xl" disabled={!dbName}>Add Database</button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Logs Panel */}
      <div className="w-80 border-l flex flex-col" style={{ backgroundColor: 'var(--bgSecondary)', borderColor: 'var(--borderDefault)' }}>
        <div className="p-4 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          <h3 className="font-semibold text-sm" style={{ color: 'var(--textPrimary)' }}>Engine Logs</h3>
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

