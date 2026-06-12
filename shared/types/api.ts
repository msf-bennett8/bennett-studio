// Re-export all database types from canonical source
export * from './database';

// ============================================================================
// API Response Wrapper (API-only)
// ============================================================================

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Sharing Types (API-only domain)
// ============================================================================

export interface ShareLink {
  id: string;
  database_id: string;
  token: string;
  expires_at?: string;
  permissions: 'read' | 'write' | 'admin';
}

export interface ShareSession {
  id: string;
  database_id: string;
  guest_count: number;
  active: boolean;
  created_at: string;
}
