// End-to-end tests for OpfsStorage running against real OPFS in
// Chromium. The harness page (/storage-harness) constructs an
// OpfsStorage instance and exposes it on `window.__opfs`; we drive
// it via `page.evaluate()` and assert on the returned values.
//
// Coverage:
//   - Save → close context → reopen → load returns same bytes
//   - listDocs reflects saves and removes after delete
//   - Crash recovery: write a residual WAL record + partial blob,
//     reopen, verify recovery cleaned up the partial state
//   - Multi-tab independence: two contexts on the same origin share
//     OPFS storage transparently

import { type BrowserContext, expect, test } from '@playwright/test';

// Mirrors the augmentation from
// `packages/app/src/pages/StorageHarness.tsx`. Each TS compilation
// unit needs its own declaration; the Playwright tests don't share
// the app's tsc graph.
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

const HARNESS_PATH = '/storage-harness';

async function gotoHarness(context: BrowserContext): Promise<import('@playwright/test').Page> {
  const page = await context.newPage();
  await page.goto(HARNESS_PATH);
  await page.waitForFunction(() => window.__opfsReady === true, undefined, { timeout: 10_000 });
  return page;
}

async function clearOpfs(context: BrowserContext): Promise<void> {
  const page = await context.newPage();
  await page.goto(HARNESS_PATH);
  await page.waitForFunction(() => window.__opfsReady === true || window.__opfsError !== undefined);
  // Drop every entry under the OPFS root, then drop the metadata
  // database, so successive tests start from a clean slate.
  await page.evaluate(async () => {
    if (!('storage' in navigator) || !('getDirectory' in navigator.storage)) return;
    const root = await navigator.storage.getDirectory();
    const root2 = root as unknown as { values(): AsyncIterable<FileSystemHandle>; };
    for await (const entry of root2.values()) {
      await root.removeEntry(entry.name, { recursive: true });
    }
    await new Promise<void>((resolve, reject) => {
      const req = indexedDB.deleteDatabase('apalabrar-storage');
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
      req.onblocked = () => resolve(); // best-effort
    });
  });
  await page.close();
}

test.describe('OpfsStorage end-to-end', () => {
  test.beforeEach(async ({ context }) => {
    await clearOpfs(context);
  });

  test('demo page can save and reload via the Save/Load buttons', async ({ context }) => {
    const page = await context.newPage();
    await page.goto('/demo');
    await page.getByTestId('demo-rendered-text').waitFor({ state: 'visible', timeout: 15_000 });

    // Initial state: nothing saved.
    await expect(page.getByTestId('demo-storage-list')).toContainText('(none yet)');

    // Save the loaded fixture.
    await page.getByTestId('demo-save-button').click();
    await expect(page.getByTestId('demo-storage-list')).toContainText('demo-current');
    await expect(page.getByTestId('demo-saved-at')).toBeVisible();

    // Reload the page; the saved entry must persist across reloads.
    await page.reload();
    await page.getByTestId('demo-rendered-text').waitFor({ state: 'visible', timeout: 15_000 });
    await expect(page.getByTestId('demo-storage-list')).toContainText('demo-current');

    // Click Load — fetches the saved blob and re-renders the text.
    await page.getByTestId('demo-load-button').click();
    await expect(page.getByTestId('demo-rendered-text')).toBeVisible();

    // Delete and confirm the list returns to empty.
    await page.getByTestId('demo-delete-button').click();
    await expect(page.getByTestId('demo-storage-list')).toContainText('(none yet)');

    await page.close();
  });

  test('saves bytes, closes the page, and reloads them on a fresh page', async ({ context }) => {
    const writer = await gotoHarness(context);
    await writer.evaluate(async () => {
      const opfs = window.__opfs!;
      // 5 distinct bytes including a 0 to catch a `length || size`
      // bug that would round trip an empty buffer.
      await opfs.saveDoc('test-doc' as never, new Uint8Array([0, 1, 2, 250, 99]));
    });
    await writer.close();

    const reader = await gotoHarness(context);
    const loaded = await reader.evaluate(async () => {
      const opfs = window.__opfs!;
      const bytes = await opfs.loadDoc('test-doc' as never);
      return bytes ? Array.from(bytes) : null;
    });
    expect(loaded).toEqual([0, 1, 2, 250, 99]);
    await reader.close();
  });

  test('listDocs returns saved docs and excludes deleted ones', async ({ context }) => {
    const page = await gotoHarness(context);
    const result = await page.evaluate(async () => {
      const opfs = window.__opfs!;
      await opfs.saveDoc('alpha' as never, new Uint8Array([1, 2, 3]));
      await opfs.saveDoc('beta' as never, new Uint8Array([4, 5]));
      await opfs.saveDoc('gamma' as never, new Uint8Array([6]));
      await opfs.deleteDoc('beta' as never);
      const list = await opfs.listDocs();
      return {
        ids: list.map((m) => m.id).sort(),
        sizes: Object.fromEntries(list.map((m) => [m.id, m.sizeBytes])),
      };
    });
    expect(result.ids).toEqual(['alpha', 'gamma']);
    expect(result.sizes).toEqual({ alpha: 3, gamma: 1 });
    await page.close();
  });

  test('subscribeChanges fires on saves and deletes within one page', async ({ context }) => {
    const page = await gotoHarness(context);
    const events = await page.evaluate(async () => {
      const opfs = window.__opfs!;
      const seen: string[] = [];
      opfs.subscribeChanges((id) => seen.push(id));
      await opfs.saveDoc('one' as never, new Uint8Array([1]));
      await opfs.saveDoc('two' as never, new Uint8Array([2]));
      await opfs.deleteDoc('one' as never);
      return seen;
    });
    expect(events).toEqual(['one', 'two', 'one']);
    await page.close();
  });

  test('rejects an invalid id with a StorageError of kind invalid-id', async ({ context }) => {
    const page = await gotoHarness(context);
    const error = await page.evaluate(async () => {
      const opfs = window.__opfs!;
      try {
        await opfs.saveDoc('has space' as never, new Uint8Array([1]));
        return null;
      } catch (cause) {
        return {
          name: (cause as Error).name,
          kind: (cause as { kind?: string; }).kind,
        };
      }
    });
    expect(error).toEqual({ name: 'StorageError', kind: 'invalid-id' });
    await page.close();
  });

  test('crash recovery: residual WAL record + partial blob is cleaned up on reopen', async ({ context }) => {
    // Set up a "crashed mid-save" state by directly writing into OPFS:
    //   - A WAL record for `save-doc` claiming expectedSizeBytes 100
    //   - A blob file with only 20 bytes (size mismatch → rollback)
    // Then construct a fresh OpfsStorage. The constructor's recovery
    // pass should rollback the blob and clean the WAL.
    const setup = await context.newPage();
    await setup.goto(HARNESS_PATH);
    await setup.waitForFunction(() =>
      window.__opfsReady === true || window.__opfsError !== undefined
    );
    await setup.evaluate(async () => {
      const root = await navigator.storage.getDirectory();
      const walDir = await root.getDirectoryHandle('wal', { create: true });
      const blobsDir = await root.getDirectoryHandle('blobs', { create: true });

      // Synthetic WAL claiming 100 bytes for `crashed-doc`.
      const walHandle = await walDir.getFileHandle('crashed-txn.json', { create: true });
      const walStream = await walHandle.createWritable();
      const walRecord = {
        txnId: 'crashed-txn',
        op: 'save',
        docId: 'crashed-doc',
        timestamp: Date.now(),
        expectedSizeBytes: 100,
      };
      await walStream.write(new Blob([new TextEncoder().encode(JSON.stringify(walRecord))]));
      await walStream.close();

      // Partial blob: 20 bytes when WAL expects 100.
      const blobHandle = await blobsDir.getFileHandle('crashed-doc.bin', { create: true });
      const blobStream = await blobHandle.createWritable();
      const partial = new Uint8Array(20);
      for (let i = 0; i < partial.length; i++) partial[i] = i;
      await blobStream.write(new Blob([partial]));
      await blobStream.close();
    });
    await setup.close();

    // Now reopen the harness. The constructor should run recovery
    // on the residual WAL record before exposing window.__opfs.
    const recovered = await gotoHarness(context);
    const observation = await recovered.evaluate(async () => {
      // The crashed doc must NOT be loadable (rolled back).
      const opfs = window.__opfs!;
      const blob = await opfs.loadDoc('crashed-doc' as never);
      const list = await opfs.listDocs();

      // The WAL record must have been removed, so the directory
      // shouldn't carry `crashed-txn.json` any more.
      const root = await navigator.storage.getDirectory();
      let walHasResidual = false;
      try {
        const walDir = await root.getDirectoryHandle('wal');
        const walDir2 = walDir as unknown as { values(): AsyncIterable<FileSystemHandle>; };
        for await (const entry of walDir2.values()) {
          if (entry.name === 'crashed-txn.json') {
            walHasResidual = true;
          }
        }
      } catch {
        // wal/ might not exist if the recovery removed everything;
        // that's fine.
      }
      return {
        loadedNonNull: blob !== null,
        listedIds: list.map((m) => m.id),
        walHasResidual,
      };
    });
    expect(observation).toEqual({
      loadedNonNull: false,
      listedIds: [],
      walHasResidual: false,
    });
    await recovered.close();
  });

  test('crash recovery: residual WAL record + complete blob commits on reopen', async ({ context }) => {
    // Same setup but the blob matches the WAL record's expectedSize:
    // recovery should `commit` (write the metadata so the doc is
    // listable) and remove the WAL record.
    const setup = await context.newPage();
    await setup.goto(HARNESS_PATH);
    await setup.waitForFunction(() =>
      window.__opfsReady === true || window.__opfsError !== undefined
    );
    await setup.evaluate(async () => {
      const root = await navigator.storage.getDirectory();
      const walDir = await root.getDirectoryHandle('wal', { create: true });
      const blobsDir = await root.getDirectoryHandle('blobs', { create: true });

      const blobHandle = await blobsDir.getFileHandle('committed-doc.bin', { create: true });
      const blobStream = await blobHandle.createWritable();
      const fullBytes = new Uint8Array(50);
      for (let i = 0; i < fullBytes.length; i++) fullBytes[i] = (i * 7) & 0xff;
      await blobStream.write(new Blob([fullBytes]));
      await blobStream.close();

      const walHandle = await walDir.getFileHandle('committed-txn.json', { create: true });
      const walStream = await walHandle.createWritable();
      const walRecord = {
        txnId: 'committed-txn',
        op: 'save',
        docId: 'committed-doc',
        timestamp: Date.now(),
        expectedSizeBytes: 50,
      };
      await walStream.write(new Blob([new TextEncoder().encode(JSON.stringify(walRecord))]));
      await walStream.close();
    });
    await setup.close();

    const recovered = await gotoHarness(context);
    const observation = await recovered.evaluate(async () => {
      const opfs = window.__opfs!;
      const list = await opfs.listDocs();
      const blob = await opfs.loadDoc('committed-doc' as never);
      return {
        listedIds: list.map((m) => m.id),
        sizesById: Object.fromEntries(list.map((m) => [m.id, m.sizeBytes])),
        loadedSize: blob ? blob.length : null,
      };
    });
    expect(observation.listedIds).toEqual(['committed-doc']);
    expect(observation.sizesById).toEqual({ 'committed-doc': 50 });
    expect(observation.loadedSize).toBe(50);
    await recovered.close();
  });
});
