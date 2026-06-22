import { useState, useCallback } from 'react';
import { shareApi } from '../services/shareApi';
import type { CreateShareRequest, ValidateShareResponse } from '@bennett/shared';

export function useSharing() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const createShare = useCallback(async (req: CreateShareRequest) => {
    setLoading(true);
    setError(null);
    try {
      const result = await shareApi.createShare(req);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create share';
      setError(msg);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  const validateShare = useCallback(async (code: string, token: string): Promise<ValidateShareResponse | null> => {
    setLoading(true);
    setError(null);
    try {
      const result = await shareApi.validateShare(code, token);
      return result;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Invalid share';
      setError(msg);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  const revokeShare = useCallback(async (code: string) => {
    setLoading(true);
    setError(null);
    try {
      return await shareApi.revokeShare(code);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to revoke';
      setError(msg);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    createShare,
    validateShare,
    revokeShare,
    loading,
    error,
    clearError: () => setError(null),
  };
}
