import js from '@eslint/js';
import solid from 'eslint-plugin-solid/configs/typescript';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/target/**',
      '**/.turbo/**',
      '**/coverage/**',
      '**/playwright-report/**',
      '**/test-results/**',
      '**/*.config.{js,mjs,cjs,ts}',
      // wasm-pack output is committed (so CI can pnpm install) but it is
      // a generated artifact — never our code, no lint signal.
      'crates/editor-core/pkg/**',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      // Honor the `_` prefix as an explicit "intentionally unused" marker,
      // matching TypeScript's own noUnusedParameters/noUnusedLocals behavior.
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' },
      ],
    },
  },
  {
    files: ['**/*.{tsx,jsx}'],
    ...solid,
  },
);
