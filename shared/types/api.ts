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
