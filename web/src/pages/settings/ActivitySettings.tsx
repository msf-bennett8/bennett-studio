import { useEffect, useMemo, useState } from 'react';
import { Activity as ActivityIcon, CheckCircle, XCircle } from 'lucide-react';
import { settingsApi, AuditEntry } from '../../services/settingsApi';

type Filter = 'all' | 'reads' | 'writes' | 'errors';

export function ActivitySettings() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<Filter>('all');

  useEffect(() => {
    settingsApi.listActivity(200).then(setEntries).catch(() => {}).finally(() => setLoading(false));
  }, []);

  const filtered = useMemo(() => {
    switch (filter) {
      case 'reads': return entries.filter(e => e.query_type === 'Select');
      case 'writes': return entries.filter(e => ['Insert', 'Update', 'Delete'].includes(e.query_type));
      case 'errors': return entries.filter(e => !e.success);
      default: return entries;
    }
  }, [entries, filter]);

  return (
    <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <ActivityIcon size={20} style={{ color: 'var(--accentPrimary)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Activity</h2>
        </div>
        <select value={filter} onChange={(e) => setFilter(e.target.value as Filter)} className="text-sm px-3 py-1.5 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textPrimary)' }}>
          <option value="all">All queries</option>
          <option value="reads">Reads only</option>
          <option value="writes">Writes only</option>
          <option value="errors">Errors only</option>
        </select>
      </div>

      {loading && <p className="text-sm" style={{ color: 'var(--textMuted)' }}>Loading...</p>}
      {!loading && filtered.length === 0 && <p className="text-sm" style={{ color: 'var(--textMuted)' }}>No activity recorded yet.</p>}

      <div className="space-y-2">
        {filtered.map((e) => (
          <div key={e.id} className="flex items-center gap-3 p-3 rounded-lg" style={{ backgroundColor: 'var(--bgSecondary)' }}>
            {e.success
              ? <CheckCircle size={16} style={{ color: 'var(--accentSuccess)', flexShrink: 0 }} />
              : <XCircle size={16} style={{ color: 'var(--accentError)', flexShrink: 0 }} />}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 text-xs" style={{ color: 'var(--textMuted)' }}>
                <span className="px-2 py-0.5 rounded-full font-medium" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textSecondary)' }}>{e.query_type}</span>
                <span>{e.share_code}</span>
                <span>•</span>
                <span>{new Date(e.timestamp).toLocaleString()}</span>
                <span>•</span>
                <span>{e.rows_affected} rows</span>
                <span>•</span>
                <span>{e.execution_time_ms}ms</span>
              </div>
              <code className="text-xs font-mono truncate block mt-1" style={{ color: 'var(--textSecondary)' }}>{e.sql}</code>
              {e.error_message && <p className="text-xs mt-1" style={{ color: 'var(--accentError)' }}>{e.error_message}</p>}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
