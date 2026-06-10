export type Theme = 'dark' | 'light' | 'terminal' | 'midnight' | 'ocean';

export interface ThemeColors {
  // Backgrounds
  bgPrimary: string;
  bgSecondary: string;
  bgTertiary: string;
  bgElevated: string;
  bgOverlay: string;
  
  // Surfaces
  surfaceDefault: string;
  surfaceHover: string;
  surfaceActive: string;
  surfaceDisabled: string;
  
  // Borders
  borderDefault: string;
  borderHover: string;
  borderFocus: string;
  borderError: string;
  
  // Text
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  textMuted: string;
  textInverse: string;
  
  // Accents
  accentPrimary: string;
  accentPrimaryHover: string;
  accentPrimaryActive: string;
  accentSecondary: string;
  accentSuccess: string;
  accentWarning: string;
  accentError: string;
  accentInfo: string;
  
  // Terminal-specific
  terminalGreen: string;
  terminalGreenDim: string;
  terminalCursor: string;
  terminalSelection: string;
  
  // Syntax highlighting (for SQL editor)
  syntaxKeyword: string;
  syntaxString: string;
  syntaxNumber: string;
  syntaxComment: string;
  syntaxFunction: string;
  syntaxType: string;
  syntaxVariable: string;
  syntaxOperator: string;
}

export const themes: Record<Theme, ThemeColors> = {
  dark: {
    bgPrimary: '#0a0a0a',
    bgSecondary: '#111111',
    bgTertiary: '#1a1a1a',
    bgElevated: '#222222',
    bgOverlay: 'rgba(0,0,0,0.8)',
    surfaceDefault: '#1a1a1a',
    surfaceHover: '#252525',
    surfaceActive: '#2a2a2a',
    surfaceDisabled: '#0f0f0f',
    borderDefault: '#2a2a2a',
    borderHover: '#3a3a3a',
    borderFocus: '#4a4a4a',
    borderError: '#ff4444',
    textPrimary: '#e0e0e0',
    textSecondary: '#a0a0a0',
    textTertiary: '#707070',
    textMuted: '#505050',
    textInverse: '#0a0a0a',
    accentPrimary: '#00d4aa',
    accentPrimaryHover: '#00e6b8',
    accentPrimaryActive: '#00bf9a',
    accentSecondary: '#6b8aff',
    accentSuccess: '#00d4aa',
    accentWarning: '#ffaa00',
    accentError: '#ff4444',
    accentInfo: '#6b8aff',
    terminalGreen: '#00d4aa',
    terminalGreenDim: '#008866',
    terminalCursor: '#00d4aa',
    terminalSelection: 'rgba(0,212,170,0.2)',
    syntaxKeyword: '#ff7b72',
    syntaxString: '#a5d6ff',
    syntaxNumber: '#79c0ff',
    syntaxComment: '#8b949e',
    syntaxFunction: '#d2a8ff',
    syntaxType: '#ffa657',
    syntaxVariable: '#e0e0e0',
    syntaxOperator: '#ff7b72',
  },
  
  light: {
    bgPrimary: '#ffffff',
    bgSecondary: '#f5f5f5',
    bgTertiary: '#eeeeee',
    bgElevated: '#ffffff',
    bgOverlay: 'rgba(0,0,0,0.4)',
    surfaceDefault: '#f5f5f5',
    surfaceHover: '#e8e8e8',
    surfaceActive: '#e0e0e0',
    surfaceDisabled: '#f0f0f0',
    borderDefault: '#e0e0e0',
    borderHover: '#c0c0c0',
    borderFocus: '#00d4aa',
    borderError: '#ff4444',
    textPrimary: '#1a1a1a',
    textSecondary: '#4a4a4a',
    textTertiary: '#6a6a6a',
    textMuted: '#9a9a9a',
    textInverse: '#ffffff',
    accentPrimary: '#00a884',
    accentPrimaryHover: '#00bf9a',
    accentPrimaryActive: '#008f72',
    accentSecondary: '#4a6cf7',
    accentSuccess: '#00a884',
    accentWarning: '#e6a700',
    accentError: '#ff4444',
    accentInfo: '#4a6cf7',
    terminalGreen: '#00a884',
    terminalGreenDim: '#006b55',
    terminalCursor: '#00a884',
    terminalSelection: 'rgba(0,168,132,0.15)',
    syntaxKeyword: '#d73a49',
    syntaxString: '#032f62',
    syntaxNumber: '#005cc5',
    syntaxComment: '#6a737d',
    syntaxFunction: '#6f42c1',
    syntaxType: '#e36209',
    syntaxVariable: '#1a1a1a',
    syntaxOperator: '#d73a49',
  },
  
  terminal: {
    bgPrimary: '#000000',
    bgSecondary: '#0a0a0a',
    bgTertiary: '#111111',
    bgElevated: '#1a1a1a',
    bgOverlay: 'rgba(0,0,0,0.9)',
    surfaceDefault: '#111111',
    surfaceHover: '#1a1a1a',
    surfaceActive: '#222222',
    surfaceDisabled: '#0a0a0a',
    borderDefault: '#222222',
    borderHover: '#333333',
    borderFocus: '#00ff88',
    borderError: '#ff3333',
    textPrimary: '#00ff88',
    textSecondary: '#00cc6a',
    textTertiary: '#009955',
    textMuted: '#006633',
    textInverse: '#000000',
    accentPrimary: '#00ff88',
    accentPrimaryHover: '#33ffaa',
    accentPrimaryActive: '#00cc6a',
    accentSecondary: '#00aaff',
    accentSuccess: '#00ff88',
    accentWarning: '#ffaa00',
    accentError: '#ff3333',
    accentInfo: '#00aaff',
    terminalGreen: '#00ff88',
    terminalGreenDim: '#008844',
    terminalCursor: '#00ff88',
    terminalSelection: 'rgba(0,255,136,0.25)',
    syntaxKeyword: '#ff6b6b',
    syntaxString: '#ffd93d',
    syntaxNumber: '#6bcb77',
    syntaxComment: '#555555',
    syntaxFunction: '#4d96ff',
    syntaxType: '#ff9f43',
    syntaxVariable: '#00ff88',
    syntaxOperator: '#ff6b6b',
  },
  
  midnight: {
    bgPrimary: '#0d1117',
    bgSecondary: '#161b22',
    bgTertiary: '#21262d',
    bgElevated: '#30363d',
    bgOverlay: 'rgba(13,17,23,0.9)',
    surfaceDefault: '#21262d',
    surfaceHover: '#30363d',
    surfaceActive: '#484f58',
    surfaceDisabled: '#161b22',
    borderDefault: '#30363d',
    borderHover: '#484f58',
    borderFocus: '#58a6ff',
    borderError: '#f85149',
    textPrimary: '#c9d1d9',
    textSecondary: '#8b949e',
    textTertiary: '#6e7681',
    textMuted: '#484f58',
    textInverse: '#0d1117',
    accentPrimary: '#58a6ff',
    accentPrimaryHover: '#79b8ff',
    accentPrimaryActive: '#388bfd',
    accentSecondary: '#a371f7',
    accentSuccess: '#3fb950',
    accentWarning: '#d29922',
    accentError: '#f85149',
    accentInfo: '#58a6ff',
    terminalGreen: '#3fb950',
    terminalGreenDim: '#238636',
    terminalCursor: '#58a6ff',
    terminalSelection: 'rgba(88,166,255,0.2)',
    syntaxKeyword: '#ff7b72',
    syntaxString: '#a5d6ff',
    syntaxNumber: '#79c0ff',
    syntaxComment: '#8b949e',
    syntaxFunction: '#d2a8ff',
    syntaxType: '#ffa657',
    syntaxVariable: '#c9d1d9',
    syntaxOperator: '#ff7b72',
  },
  
  ocean: {
    bgPrimary: '#0a1628',
    bgSecondary: '#0f1d32',
    bgTertiary: '#162544',
    bgElevated: '#1e3260',
    bgOverlay: 'rgba(10,22,40,0.9)',
    surfaceDefault: '#162544',
    surfaceHover: '#1e3260',
    surfaceActive: '#264070',
    surfaceDisabled: '#0f1d32',
    borderDefault: '#1e3260',
    borderHover: '#264070',
    borderFocus: '#00d4ff',
    borderError: '#ff6b6b',
    textPrimary: '#e0f0ff',
    textSecondary: '#90b8d4',
    textTertiary: '#5a8cb0',
    textMuted: '#3a5a7a',
    textInverse: '#0a1628',
    accentPrimary: '#00d4ff',
    accentPrimaryHover: '#33e0ff',
    accentPrimaryActive: '#00a8cc',
    accentSecondary: '#7b61ff',
    accentSuccess: '#00e5a0',
    accentWarning: '#ffb800',
    accentError: '#ff6b6b',
    accentInfo: '#00d4ff',
    terminalGreen: '#00e5a0',
    terminalGreenDim: '#009960',
    terminalCursor: '#00d4ff',
    terminalSelection: 'rgba(0,212,255,0.2)',
    syntaxKeyword: '#ff7b9d',
    syntaxString: '#a5e8ff',
    syntaxNumber: '#79d0ff',
    syntaxComment: '#5a7a9a',
    syntaxFunction: '#c2a8ff',
    syntaxType: '#ffd4a3',
    syntaxVariable: '#e0f0ff',
    syntaxOperator: '#ff7b9d',
  },
};

export const defaultTheme: Theme = 'dark';

export function getThemeColors(theme: Theme): ThemeColors {
  return themes[theme];
}

export function getAllThemes(): { id: Theme; name: string; description: string }[] {
  return [
    { id: 'dark', name: 'Dark', description: 'Default dark theme with green accents' },
    { id: 'light', name: 'Light', description: 'Clean light theme for daytime use' },
    { id: 'terminal', name: 'Terminal', description: 'Classic terminal green on black' },
    { id: 'midnight', name: 'Midnight', description: 'GitHub-inspired dark blue' },
    { id: 'ocean', name: 'Ocean', description: 'Deep blue with cyan accents' },
  ];
}
