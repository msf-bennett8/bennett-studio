import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface NotificationPreferencesState {
  enabled: boolean; // master switch — also gates browser Notification permission
  guestConnected: boolean;
  engineOffline: boolean;
  setEnabled: (b: boolean) => void;
  setGuestConnected: (b: boolean) => void;
  setEngineOffline: (b: boolean) => void;
}

export const useNotificationPreferencesStore = create<NotificationPreferencesState>()(
  persist(
    (set) => ({
      enabled: false,
      guestConnected: true,
      engineOffline: true,
      setEnabled: (b) => set({ enabled: b }),
      setGuestConnected: (b) => set({ guestConnected: b }),
      setEngineOffline: (b) => set({ engineOffline: b }),
    }),
    { name: 'bennett-notification-preferences' }
  )
);
