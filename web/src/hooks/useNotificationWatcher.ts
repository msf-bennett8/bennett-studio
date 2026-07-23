import { useEffect, useRef } from 'react';
import { useNotificationPreferencesStore } from '../stores/notificationPreferencesStore';
import { settingsApi } from '../services/settingsApi';
import { api } from '../services/api';

export function useNotificationWatcher() {
  const { enabled, guestConnected, engineOffline } = useNotificationPreferencesStore();
  const lastGuestCount = useRef<number | null>(null);
  const lastEngineOnline = useRef<boolean | null>(null);

  useEffect(() => {
    if (!enabled || typeof Notification === 'undefined' || Notification.permission !== 'granted') return;

    const poll = async () => {
      if (guestConnected) {
        try {
          const guests = await settingsApi.listGuests();
          if (lastGuestCount.current !== null && guests.length > lastGuestCount.current) {
            new Notification('New guest connected', {
              body: `${guests.length} guest${guests.length !== 1 ? 's' : ''} now connected`,
            });
          }
          lastGuestCount.current = guests.length;
        } catch {
          // ignore — guest list unavailable (e.g. remote mode)
        }
      }

      if (engineOffline) {
        try {
          await api.health();
          if (lastEngineOnline.current === false) {
            new Notification('Engine back online', { body: 'Bennett engine is reachable again.' });
          }
          lastEngineOnline.current = true;
        } catch {
          if (lastEngineOnline.current === true) {
            new Notification('Engine offline', { body: 'Bennett engine stopped responding.' });
          }
          lastEngineOnline.current = false;
        }
      }
    };

    poll();
    const interval = setInterval(poll, 10000);
    return () => clearInterval(interval);
  }, [enabled, guestConnected, engineOffline]);
}
