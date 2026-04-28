/**
 * Phase 2.5 — OPFS bootstrap helper for the page-load timeline.
 *
 * At T+50 ms the Service Worker registers and OPFS is opened (async);
 * at T+150 ms the OPFS scan completes and — if a session exists —
 * the Landing page surfaces a "Continue last doc?" toast.
 *
 * `bootstrapOpfs` does that scan in one call:
 * - Creates the storage backend (default `OpfsStorage.create()`,
 *   override-able for tests via the `factory` parameter).
 * - Lists documents, sorted newest-first by `lastModified`.
 * - Returns `{ storage, recents }` so the caller can both render
 *   the toast/menu AND retain the storage handle for later loads.
 * - On unavailable platforms (no OPFS, denied permission, factory
 *   rejection) returns `{ storage: null, recents: [] }` without
 *   throwing. The Landing page treats that as "no prior session",
 *   which is the correct UX (private windows, first visit, etc.).
 */
import { type DocMetadata, OpfsStorage, type StorageBackend } from '@apalabrar/editor-bridge';

export interface BootstrapResult {
  readonly storage: StorageBackend | null;
  readonly recents: readonly DocMetadata[];
}

export type StorageFactory = () => Promise<StorageBackend>;

const defaultFactory: StorageFactory = () => OpfsStorage.create();

export async function bootstrapOpfs(
  factory: StorageFactory = defaultFactory,
): Promise<BootstrapResult> {
  let storage: StorageBackend;
  try {
    storage = await factory();
  } catch {
    return { storage: null, recents: [] };
  }

  const list = await storage.listDocs();
  const recents = [...list].sort((a, b) => b.lastModified - a.lastModified);
  return { storage, recents };
}
