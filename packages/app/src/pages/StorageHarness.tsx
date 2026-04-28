import { OpfsStorage } from '@apalabrar/editor-bridge';
import { type Component, createSignal, onMount } from 'solid-js';

// Test-only harness route. Construct an OpfsStorage instance and
// expose it on `window.__opfs` so Playwright can drive it from
// `page.evaluate()`. Not linked from the public UI.
//
// The window augmentation uses a structural interface (not the
// `OpfsStorage` class) so the `tests-e2e/tests/storage.spec.ts`
// duck-typed declaration is compatible. tsc compiles both as part
// of the workspace; mismatched augmentations produce TS2717.

declare global {
  interface Window {
    __opfs?: {
      saveDoc(id: string, blob: Uint8Array): Promise<void>;
      loadDoc(id: string): Promise<Uint8Array | null>;
      listDocs(): Promise<{ id: string; sizeBytes: number; lastModified: number; }[]>;
      deleteDoc(id: string): Promise<void>;
      subscribeChanges(callback: (id: string) => void): () => void;
    };
    __opfsReady?: boolean;
    __opfsError?: string;
  }
}

export const StorageHarness: Component = () => {
  const [ready, setReady] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const storage = await OpfsStorage.create();
      window.__opfs = storage;
      window.__opfsReady = true;
      setReady(true);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      window.__opfsError = message;
      setError(message);
    }
  });

  return (
    <main class='mx-auto max-w-3xl space-y-4 p-8 font-mono text-sm'>
      <h1 class='text-2xl font-semibold'>OPFS storage harness</h1>
      <p class='text-neutral-500'>
        Internal route used by Playwright to exercise OpfsStorage. Drives writes through{' '}
        <code>window.__opfs</code>.
      </p>
      <div data-testid='harness-status'>
        {ready() ? 'ready' : error() ? `error: ${error()}` : 'initialising'}
      </div>
    </main>
  );
};
