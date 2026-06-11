import { useState, useEffect, useCallback } from 'react';
import { Theme, defaultTheme, getThemeColors, ThemeColors } from '../theme';

const THEME_STORAGE_KEY = 'bennett-theme';

export function useTheme() {
  const [theme, setThemeState] = useState<<Theme>(() => {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    return (stored as Theme) || defaultTheme;
  });

  const [colors, setColors] = useState<<ThemeColors>(() => getThemeColors(theme));

  useEffect(() => {
    const newColors = getThemeColors(theme);
    setColors(newColors);
    
    // Apply CSS variables to document root
    const root = document.documentElement;
    Object.entries(newColors).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value);
    });
    
    // Set data attribute for Tailwind dark mode
    root.setAttribute('data-theme', theme);
    
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme);
  }, []);

  const toggleTheme = useCallback(() => {
    setThemeState(prev => {
      const themes: Theme[] = ['dark', 'light', 'terminal', 'midnight', 'ocean'];
      const currentIndex = themes.indexOf(prev);
      return themes[(currentIndex + 1) % themes.length];
    });
  }, []);

  return { theme, colors, setTheme, toggleTheme };
}
