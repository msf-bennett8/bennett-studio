import fs from 'fs'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

const bennettBranding = () => ({
  name: 'bennett-branding',
  configureServer(server) {
    const originalPrintUrls = server.printUrls;
    server.printUrls = () => {
      console.log();
      console.log('  \x1b[38;2;0;212;170m╔══════════════════════════════════════════════════════════╗\x1b[0m');
      console.log('  \x1b[38;2;0;212;170m║              B E N N E T T   S T U D I O                 ║\x1b[0m');
      console.log('  \x1b[38;2;0;212;170m║     silicon swimming ducks isotope foundation            ║\x1b[0m');
      console.log('  \x1b[38;2;0;212;170m╚══════════════════════════════════════════════════════════╝\x1b[0m');
      console.log();
      originalPrintUrls();
    };
  }
});

export default defineConfig({
  plugins: [react(), bennettBranding()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@bennett/shared': path.resolve(__dirname, '../shared/dist'),
      '@bennett/sdk': path.resolve(__dirname, '../shared/sdk/typescript/dist'),
    },
  },
  server: {
    port: 5173,
    host: true,
    proxy: {
      '/api': {
        target: `http://localhost:${fs.readFileSync('/tmp/bennett-engine.port', 'utf8').trim() || 3001}`,
        changeOrigin: true,
      },
    },
  },
  assetsInclude: ['**/*.json'],
});
