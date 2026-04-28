/**
 * Phase 2.5 RED tests for `bootstrapOpfs`. Uses an in-memory
 * `MemoryStorage` factory to avoid OPFS entirely (happy-dom doesn't
 * implement OPFS, and even if it did the tests would race the
 * filesystem). Coverage:
 *
 * - happy path: factory returns storage with N docs → `recents`
 *   carries them, sorted newest-first.
 * - empty storage → `recents: []`.
 * - factory rejects (OPFS unavailable, denied permission) →
 *   `{ storage: null, recents: [] }` without throwing.
 * - default factory path is exercised by the production wiring;
 *   here we only test the injection seam.
 */
import {
  type DocId,
  MemoryStorage,
  parseDocId,
  type StorageBackend,
} from '@apalabrar/editor-bridge';
import { describe, expect, it } from 'vitest';
import { bootstrapOpfs } from './opfsBootstrap';

const id = (s: string): DocId => parseDocId(s);

async function seed(storage: StorageBackend, ids: string[]): Promise<void> {
  for (let i = 0; i < ids.length; i++) {
    await storage.saveDoc(id(ids[i] as string), new Uint8Array([i]));
    // Wait one ms between saves so `lastModified` differs across docs
    // and the newest-first sort has a stable ordering.
    await new Promise((r) => setTimeout(r, 1));
  }
}

describe('bootstrapOpfs', () => {
  it('returns the injected storage handle', async () => {
    const memory = new MemoryStorage();
    const result = await bootstrapOpfs(async () => memory);
    expect(result.storage).toBe(memory);
  });

  it('returns empty recents when storage is empty', async () => {
    const result = await bootstrapOpfs(async () => new MemoryStorage());
    expect(result.recents).toEqual([]);
  });

  it('returns all recents from a populated storage', async () => {
    const memory = new MemoryStorage();
    await seed(memory, ['a', 'b', 'c']);
    const result = await bootstrapOpfs(async () => memory);
    expect(result.recents.length).toBe(3);
    expect(result.recents.map((d) => d.id).sort()).toEqual(['a', 'b', 'c']);
  });

  it('sorts recents newest-first by lastModified', async () => {
    const memory = new MemoryStorage();
    await seed(memory, ['oldest', 'middle', 'newest']);
    const result = await bootstrapOpfs(async () => memory);
    expect(result.recents.map((d) => d.id)).toEqual(['newest', 'middle', 'oldest']);
  });

  it('returns { storage: null, recents: [] } when factory rejects', async () => {
    const result = await bootstrapOpfs(async () => {
      throw new Error('OPFS unavailable');
    });
    expect(result.storage).toBe(null);
    expect(result.recents).toEqual([]);
  });
});
