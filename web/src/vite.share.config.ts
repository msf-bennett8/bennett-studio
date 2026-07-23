import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@bennettstudio/shared': path.resolve(__dirname, '../shared/dist'),
      '@bennettstudio/sdk': path.resolve(__dirname, '../shared/sdk/typescript/dist'),
    },
  },
  build: {
    outDir: 'dist-share',
    rollupOptions: {
      input: './share.html',
    },
  },
});
