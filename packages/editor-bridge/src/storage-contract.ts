// Shared contract suite for any `StorageBackend` implementation.
// Both `MemoryStorage` and (eventually) `OpfsStorage` import this and
// run it against their own factory so a backend cannot drift from the
// contract without flipping a test.

import { describe, expect, it, vi } from 'vitest';
import { parseDocId, type StorageBackend, type StorageError } from './storage';

const ID_A = parseDocId('alpha');
const ID_B = parseDocId('beta');
const ID_C = parseDocId('gamma');

function bytes(...values: number[]): Uint8Array {
  return new Uint8Array(values);
}

/**
 * Run the full contract suite against `factory`. Each call to
 * `factory` MUST return a freshly-initialized backend that is empty
 * — the tests assume no shared state between cases.
 */
export function defineContractSuite(name: string, factory: () => StorageBackend): void {
  describe(`${name}: StorageBackend contract`, () => {
    describe('saveDoc + loadDoc', () => {
      it('returns the same bytes that were saved', async () => {
        const storage = factory();
        const blob = bytes(1, 2, 3, 4, 5);
        await storage.saveDoc(ID_A, blob);
        const loaded = await storage.loadDoc(ID_A);
        expect(loaded).not.toBeNull();
        expect(Array.from(loaded!)).toEqual([1, 2, 3, 4, 5]);
      });

      it('overwrites an existing document on second save', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1, 2, 3));
        await storage.saveDoc(ID_A, bytes(9, 9, 9, 9));
        const loaded = await storage.loadDoc(ID_A);
        expect(Array.from(loaded!)).toEqual([9, 9, 9, 9]);
      });

      it('keeps independent documents independent', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1, 1, 1));
        await storage.saveDoc(ID_B, bytes(2, 2));
        const a = await storage.loadDoc(ID_A);
        const b = await storage.loadDoc(ID_B);
        expect(Array.from(a!)).toEqual([1, 1, 1]);
        expect(Array.from(b!)).toEqual([2, 2]);
      });

      it('roundtrips an empty blob', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, new Uint8Array(0));
        const loaded = await storage.loadDoc(ID_A);
        expect(loaded).not.toBeNull();
        expect(loaded!.length).toBe(0);
      });

      it('roundtrips a 1 MB blob', async () => {
        const storage = factory();
        const big = new Uint8Array(1024 * 1024);
        for (let i = 0; i < big.length; i++) big[i] = i & 0xff;
        await storage.saveDoc(ID_A, big);
        const loaded = await storage.loadDoc(ID_A);
        expect(loaded).not.toBeNull();
        const view = loaded!;
        expect(view.length).toBe(big.length);
        // Spot-check first / middle / last bytes — full memcmp is in
        // property tests, here we just guard against truncation or
        // padding bugs.
        expect(view[0]).toBe(0);
        expect(view[512_000]).toBe(big[512_000]);
        expect(view[big.length - 1]).toBe(big[big.length - 1]);
      });
    });

    describe('loadDoc on missing', () => {
      it('returns null for an unknown id', async () => {
        const storage = factory();
        const loaded = await storage.loadDoc(parseDocId('nonexistent'));
        expect(loaded).toBeNull();
      });

      it('returns null after deleteDoc', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1, 2, 3));
        await storage.deleteDoc(ID_A);
        expect(await storage.loadDoc(ID_A)).toBeNull();
      });
    });

    describe('listDocs', () => {
      it('starts empty', async () => {
        const storage = factory();
        expect(await storage.listDocs()).toEqual([]);
      });

      it('lists every saved document exactly once', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1));
        await storage.saveDoc(ID_B, bytes(1, 2));
        await storage.saveDoc(ID_C, bytes(1, 2, 3));
        const ids = (await storage.listDocs()).map((m) => m.id).sort();
        expect(ids).toEqual([ID_A, ID_B, ID_C].sort());
      });

      it('reports the saved size in metadata', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1, 2, 3, 4, 5));
        const list = await storage.listDocs();
        expect(list).toHaveLength(1);
        const [first] = list;
        expect(first?.id).toBe(ID_A);
        expect(first?.sizeBytes).toBe(5);
      });

      it('updates the size after an overwrite', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1, 2, 3));
        await storage.saveDoc(ID_A, bytes(1, 2, 3, 4, 5, 6, 7));
        const list = await storage.listDocs();
        expect(list).toHaveLength(1);
        expect(list[0]?.sizeBytes).toBe(7);
      });

      it('reports a lastModified that is never in the future', async () => {
        const storage = factory();
        const before = Date.now();
        await storage.saveDoc(ID_A, bytes(1));
        const after = Date.now();
        const list = await storage.listDocs();
        const meta = list[0];
        expect(meta).toBeDefined();
        expect(meta!.lastModified).toBeGreaterThanOrEqual(before);
        expect(meta!.lastModified).toBeLessThanOrEqual(after);
      });

      it('removes a document from the list after deleteDoc', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1));
        await storage.saveDoc(ID_B, bytes(2));
        await storage.deleteDoc(ID_A);
        const ids = (await storage.listDocs()).map((m) => m.id);
        expect(ids).toEqual([ID_B]);
      });
    });

    describe('deleteDoc', () => {
      it('is idempotent on missing ids (no throw)', async () => {
        const storage = factory();
        await expect(storage.deleteDoc(ID_A)).resolves.toBeUndefined();
      });

      it('removes the blob so subsequent loadDoc returns null', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1, 2, 3));
        await storage.deleteDoc(ID_A);
        expect(await storage.loadDoc(ID_A)).toBeNull();
      });

      it('does not affect other documents', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1));
        await storage.saveDoc(ID_B, bytes(2));
        await storage.deleteDoc(ID_A);
        expect(await storage.loadDoc(ID_B)).not.toBeNull();
      });
    });

    describe('subscribeChanges', () => {
      it('fires after saveDoc with the saved id', async () => {
        const storage = factory();
        const cb = vi.fn();
        storage.subscribeChanges(cb);
        await storage.saveDoc(ID_A, bytes(1));
        expect(cb).toHaveBeenCalledWith(ID_A);
      });

      it('fires after deleteDoc with the deleted id', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1));
        const cb = vi.fn();
        storage.subscribeChanges(cb);
        await storage.deleteDoc(ID_A);
        expect(cb).toHaveBeenCalledWith(ID_A);
      });

      it('does not fire for loadDoc or listDocs', async () => {
        const storage = factory();
        await storage.saveDoc(ID_A, bytes(1));
        const cb = vi.fn();
        storage.subscribeChanges(cb);
        await storage.loadDoc(ID_A);
        await storage.listDocs();
        expect(cb).not.toHaveBeenCalled();
      });

      it('fires multiple subscribers in registration order', async () => {
        const storage = factory();
        const calls: number[] = [];
        storage.subscribeChanges(() => calls.push(1));
        storage.subscribeChanges(() => calls.push(2));
        storage.subscribeChanges(() => calls.push(3));
        await storage.saveDoc(ID_A, bytes(1));
        expect(calls).toEqual([1, 2, 3]);
      });

      it('detaches the callback when the unsubscribe fn runs', async () => {
        const storage = factory();
        const cb = vi.fn();
        const unsubscribe = storage.subscribeChanges(cb);
        unsubscribe();
        await storage.saveDoc(ID_A, bytes(1));
        expect(cb).not.toHaveBeenCalled();
      });

      it('is safe to unsubscribe twice', async () => {
        const storage = factory();
        const cb = vi.fn();
        const unsubscribe = storage.subscribeChanges(cb);
        unsubscribe();
        // Second call must not throw and must not reattach the
        // callback if a future save fires.
        unsubscribe();
        await storage.saveDoc(ID_A, bytes(1));
        expect(cb).not.toHaveBeenCalled();
      });

      it('isolates subscriber errors from the writer', async () => {
        // A subscriber that throws should not propagate to saveDoc's
        // resolved promise — the storage operation already committed.
        const storage = factory();
        storage.subscribeChanges(() => {
          throw new Error('subscriber boom');
        });
        await expect(storage.saveDoc(ID_A, bytes(1))).resolves.toBeUndefined();
        // And the doc must have been saved despite the throw.
        expect(await storage.loadDoc(ID_A)).not.toBeNull();
      });
    });

    describe('error paths', () => {
      it('rejects saveDoc with an invalid id', async () => {
        const storage = factory();
        const bad = 'has space' as never; // bypass branding for this test
        await expect(storage.saveDoc(bad, bytes(1))).rejects.toMatchObject(
          {
            name: 'StorageError',
            kind: 'invalid-id',
          } satisfies Partial<StorageError>,
        );
      });

      it('rejects loadDoc with an invalid id', async () => {
        const storage = factory();
        const bad = 'has space' as never;
        await expect(storage.loadDoc(bad)).rejects.toMatchObject({
          kind: 'invalid-id',
        });
      });

      it('rejects deleteDoc with an invalid id', async () => {
        const storage = factory();
        const bad = '../etc' as never;
        await expect(storage.deleteDoc(bad)).rejects.toMatchObject({
          kind: 'invalid-id',
        });
      });
    });
  });
}
