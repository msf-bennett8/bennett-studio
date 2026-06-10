/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: ['class', '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        // Dynamic theme colors - these map to CSS variables
        bg: {
          primary: 'var(--bgPrimary)',
          secondary: 'var(--bgSecondary)',
          tertiary: 'var(--bgTertiary)',
          elevated: 'var(--bgElevated)',
          overlay: 'var(--bgOverlay)',
        },
        surface: {
          DEFAULT: 'var(--surfaceDefault)',
          hover: 'var(--surfaceHover)',
          active: 'var(--surfaceActive)',
          disabled: 'var(--surfaceDisabled)',
        },
        border: {
          DEFAULT: 'var(--borderDefault)',
          hover: 'var(--borderHover)',
          focus: 'var(--borderFocus)',
          error: 'var(--borderError)',
        },
        text: {
          primary: 'var(--textPrimary)',
          secondary: 'var(--textSecondary)',
          tertiary: 'var(--textTertiary)',
          muted: 'var(--textMuted)',
          inverse: 'var(--textInverse)',
        },
        accent: {
          primary: 'var(--accentPrimary)',
          'primary-hover': 'var(--accentPrimaryHover)',
          'primary-active': 'var(--accentPrimaryActive)',
          secondary: 'var(--accentSecondary)',
          success: 'var(--accentSuccess)',
          warning: 'var(--accentWarning)',
          error: 'var(--accentError)',
          info: 'var(--accentInfo)',
        },
        terminal: {
          green: 'var(--terminalGreen)',
          'green-dim': 'var(--terminalGreenDim)',
          cursor: 'var(--terminalCursor)',
          selection: 'var(--terminalSelection)',
        },
        syntax: {
          keyword: 'var(--syntaxKeyword)',
          string: 'var(--syntaxString)',
          number: 'var(--syntaxNumber)',
          comment: 'var(--syntaxComment)',
          function: 'var(--syntaxFunction)',
          type: 'var(--syntaxType)',
          variable: 'var(--syntaxVariable)',
          operator: 'var(--syntaxOperator)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'SF Pro Display', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'Cascadia Code', 'monospace'],
      },
      animation: {
        'blink': 'blink 1s step-end infinite',
        'fade-in': 'fadeIn 0.2s ease-out',
        'slide-in': 'slideIn 0.3s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideIn: {
          '0%': { transform: 'translateX(-10px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
}
