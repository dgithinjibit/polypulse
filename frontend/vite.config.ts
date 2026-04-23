import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      include: ['src/context/**', 'src/lib/stellar-helper.ts'],
    },
  },
  plugins: [
    react(),
    nodePolyfills({
      // Polyfill global, process, Buffer — needed by Stellar SDK
      globals: { global: true, process: true, Buffer: true },
      protocolImports: true,
      // Exclude eval-based polyfills to comply with strict CSP on IPFS gateways
      exclude: ['vm'],
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    target: 'es2020', // Modern browsers only - reduces bundle size
    rollupOptions: {
      output: {
        manualChunks: {
          // Split Stellar SDK into separate chunk (lazy loaded)
          'stellar': ['@stellar/stellar-sdk'],
          // Split React vendor libs
          'react-vendor': ['react', 'react-dom', 'react-router-dom'],
          // Split other heavy dependencies
          'vendor': ['axios'],
        },
      },
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:8000',
        ws: true,
      },
    },
  },
})
