import { useEffect, useState } from 'react';
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
      let listen: typeof import('@tauri-apps/api/event').listen | undefined;
      try {
        const tauri = await import('@tauri-apps/api/event');
        listen = tauri.listen;
      } catch {
        // Not in Tauri environment
        return;
      }
      unlisten = await listen<DeepLinkEvent>('deep-link-share', (event: { payload: DeepLinkEvent }) => {
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
