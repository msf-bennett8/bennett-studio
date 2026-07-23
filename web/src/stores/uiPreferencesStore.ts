import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface UIPreferencesState {
  fontScale: number; // percentage, 90-130
  reduceMotion: boolean;
  compactSidebar: boolean;
  setFontScale: (n: number) => void;
  setReduceMotion: (b: boolean) => void;
  setCompactSidebar: (b: boolean) => void;
}

export const useUIPreferencesStore = create<UIPreferencesState>()(
  persist(
    (set) => ({
      fontScale: 100,
      reduceMotion: false,
      compactSidebar: false,
      setFontScale: (n) => set({ fontScale: n }),
      setReduceMotion: (b) => set({ reduceMotion: b }),
      setCompactSidebar: (b) => set({ compactSidebar: b }),
    }),
    { name: 'bennett-ui-preferences' }
  )
);
