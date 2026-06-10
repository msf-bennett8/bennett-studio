/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: { primary: 'var(--bgPrimary)', secondary: 'var(--bgSecondary)', tertiary: 'var(--bgTertiary)', elevated: 'var(--bgElevated)', overlay: 'var(--bgOverlay)' },
        surface: { DEFAULT: 'var(--surfaceDefault)', hover: 'var(--surfaceHover)', active: 'var(--surfaceActive)', disabled: 'var(--surfaceDisabled)' },
        border: { DEFAULT: 'var(--borderDefault)', hover: 'var(--borderHover)', focus: 'var(--borderFocus)', error: 'var(--borderError)' },
        text: { primary: 'var(--textPrimary)', secondary: 'var(--textSecondary)', tertiary: 'var(--textTertiary)', muted: 'var(--textMuted)', inverse: 'var(--textInverse)' },
        accent: { primary: 'var(--accentPrimary)', 'primary-hover': 'var(--accentPrimaryHover)', 'primary-active': 'var(--accentPrimaryActive)', secondary: 'var(--accentSecondary)', success: 'var(--accentSuccess)', warning: 'var(--accentWarning)', error: 'var(--accentError)', info: 'var(--accentInfo)' },
        terminal: { green: 'var(--terminalGreen)', 'green-dim': 'var(--terminalGreenDim)', cursor: 'var(--terminalCursor)', selection: 'var(--terminalSelection)' },
      },
      fontFamily: { sans: ['Inter', 'SF Pro Display', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'], mono: ['JetBrains Mono', 'Fira Code', 'Cascadia Code', 'monospace'] },
    },
  },
  plugins: [],
}
