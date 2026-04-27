import solid from 'vite-plugin-solid';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [solid()],
  server: {
    port: 5173,
    headers: {
      // Required for SharedArrayBuffer + WASM threads
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
    fs: {
      // Allow Vite to read fixtures from the repo-wide tests-corpus/
      // directory (lives outside packages/app/). Bundled assets land
      // back in packages/app/dist/ at build time.
      allow: ['..', '../..'],
    },
  },
  // Vite needs to recognise .docx as an asset import target so
  // `import url from '...sample.docx?url'` emits the file rather than
  // failing the parse step.
  assetsInclude: ['**/*.docx'],
  build: {
    target: 'es2022',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          solid: ['solid-js'],
        },
      },
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./src/test-setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      exclude: [
        '**/node_modules/**',
        '**/dist/**',
        '**/*.config.*',
        // Bootstrap entry: just calls render(); no testable logic.
        'src/main.tsx',
        // Test setup: registers jest-dom matchers.
        'src/test-setup.ts',
      ],
    },
  },
});
