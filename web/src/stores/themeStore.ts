import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { Theme, defaultTheme, getThemeColors, ThemeColors } from '../theme';

interface ThemeState {
  theme: Theme;
  colors: ThemeColors;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  _hasHydrated: boolean; // Industry standard: track hydration state
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      theme: defaultTheme,
      colors: getThemeColors(defaultTheme),
      _hasHydrated: false,

      setTheme: (theme) => set({
        theme,
        colors: getThemeColors(theme)
      }),

      toggleTheme: () => {
        const themes: Theme[] = ['dark', 'light', 'terminal', 'midnight', 'ocean'];
        const currentIndex = themes.indexOf(get().theme);
        const nextTheme = themes[(currentIndex + 1) % themes.length];
        set({ theme: nextTheme, colors: getThemeColors(nextTheme) });
      },
    }),
    {
      name: 'bennett-theme-storage',
      partialize: (state) => ({ theme: state.theme }), // Only persist source of truth
      onRehydrateStorage: () => (state) => {
        if (state) {
          // Recompute derived state from persisted source of truth
          state.colors = getThemeColors(state.theme);
          state._hasHydrated = true;
        }
      },
    }
  )
);
