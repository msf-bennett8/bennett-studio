import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useNavigate } from 'react-router-dom';

interface DeepLinkEvent {
  code: string;
  token: string;
  source: string;
}

export function useDeepLink() {
  const navigate = useNavigate();
  const [pendingShare, setPendingShare] = useState<DeepLinkEvent | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setupListener = async () => {
      unlisten = await listen<DeepLinkEvent>('deep-link-share', (event) => {
        console.log('[DeepLink] Received:', event.payload);
        setPendingShare(event.payload);
        // Auto-navigate to join share page
        navigate(`/join-share?code=${event.payload.code}&t=${encodeURIComponent(event.payload.token)}`);
      });
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [navigate]);

  return { pendingShare };
}
