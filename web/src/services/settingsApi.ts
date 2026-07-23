import { API_BASE_URL, isRemoteMode } from './api';

export interface EngineInfo {
  host_id: string;
  version: string;
  relay_url: string;
  data_dir: string;
  database_count: number;
  active_share_count: number;
}

export interface AuditEntry {
  id: string;
  timestamp: string;
  share_code: string;
  db_id: string;
  peer_ip: string;
  user_agent: string | null;
  query_type: 'Select' | 'Insert' | 'Update' | 'Delete' | 'Create' | 'Alter' | 'Drop' | 'Other';
  sql: string;
  rows_affected: number;
  execution_time_ms: number;
  success: boolean;
  error_message: string | null;
  permission_level: string;
}

export interface GuestSession {
  id: string;
  share_code: string;
  ip_address: string | null;
  user_agent: string | null;
  connected_at: string;
  last_active: string;
  query_count: number;
}

async function get<T>(path: string, fallback: T): Promise<T> {
  if (isRemoteMode()) return fallback;
  const response = await fetch(`${API_BASE_URL}${path}`);
  if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  const result = await response.json();
  if (!result.success) throw new Error(result.error || 'Request failed');
  return result.data;
}

export const settingsApi = {
  getEngineInfo: () => get<EngineInfo | null>('/api/engine/info', null),
  listActivity: (limit = 100) => get<AuditEntry[]>(`/api/activity?limit=${limit}`, []),
  listGuests: () => get<GuestSession[]>('/api/guests', []),
  clearActivity: async (): Promise<number> => {
    if (isRemoteMode()) return 0;
    const response = await fetch(`${API_BASE_URL}/api/activity`, { method: 'DELETE' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    return result.data?.cleared ?? 0;
  },
  disconnectGuest: async (id: string): Promise<boolean> => {
    if (isRemoteMode()) return false;
    const response = await fetch(`${API_BASE_URL}/api/guests/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    return result.success;
  },
};
