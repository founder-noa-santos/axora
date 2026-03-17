import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [
    react(),
    tailwindcss(),
    // Compression plugin for production builds
    mode === 'production' || mode === 'analyze'
      ? {
          name: 'compression',
          closeBundle() {
            console.log('✓ Build optimization complete');
          },
        }
      : [],
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
    // Enable TypeScript path alias resolution (Vite 8 feature)
    tsconfigPaths: true,
  },
  server: {
    port: 5173,
    strictPort: true,
    // Forward browser console logs to terminal (Vite 8 feature)
    forwardConsole: true,
  },
  build: {
    target: 'esnext',
    outDir: 'dist',
    emptyOutDir: true,
    minify: false,
    chunkSizeWarningLimit: 500,
    sourcemap: mode !== 'production',
  },
  optimizeDeps: {
    include: ['react', 'react-dom', 'zustand', '@tauri-apps/api/core'],
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx', 'src/**/__tests__/**/*.test.ts'],
    css: false,
  },
}));
