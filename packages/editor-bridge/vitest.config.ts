import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      // OPFS only exists in browsers; storage-opfs.ts is exercised
      // end-to-end by tests-e2e/tests/storage.spec.ts in real
      // Chromium and intentionally excluded from the Node-side
      // coverage report. The contract suite (against MemoryStorage)
      // and the pure WAL state machine (against decideRecovery)
      // give us the test discipline; the OPFS adapter is a thin
      // glue layer over real APIs.
      exclude: [
        '**/node_modules/**',
        '**/dist/**',
        '**/*.config.*',
        'src/storage-opfs.ts',
      ],
      thresholds: {
        lines: 90,
        functions: 90,
        branches: 90,
        statements: 90,
      },
    },
  },
});
