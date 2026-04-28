// Production storage backend.
//
// Layout on disk:
//   - OPFS (origin-private file system, real durable bytes):
//       wal/<txnId>.json   — WAL records, one file per pending op
//       blobs/<docId>.bin  — committed document blobs
//   - IndexedDB (database `apalabrar-storage`, object store `metadata`):
//       metadata: { id, sizeBytes, lastModified } keyed by `id`
//
// The two stores together model a single logical document. OPFS holds
// the bytes (cheap to write, slow to enumerate); IndexedDB holds the
// projection used by `listDocs` (fast to query, easy to keep
// transactional). The WAL closes the gap between them — every write
// records its intent first, so a crash between the OPFS commit and
// the IDB update is recoverable on next boot.
//
// Save flow (atomic against power-loss / kill-tab):
//   1. Append a WAL record to OPFS.
//   2. Write the blob to `blobs/<docId>.bin` using
//      `FileSystemFileHandle.createWritable() + write + close`. The
//      stream's `close()` is the durability barrier — until it
//      resolves, the file is invisible to readers.
//   3. Update the IndexedDB metadata row.
//   4. Delete the WAL record.
//
// Recovery (constructor-time):
//   - Scan `wal/` for residual records.
//   - For each, observe (blobPresent, blobSizeBytes, metadataPresent)
//     and feed it to the pure `decideRecovery` from `wal.ts`.
//   - Execute the decision: rollback (drop partial blob + WAL),
//     commit (write metadata + drop WAL), apply-delete (remove blob
//     + metadata + WAL), or cleanup (just drop the WAL record).
//
// What's intentionally NOT in this v0:
//   - Cross-tab notifications (BroadcastChannel). Subscribers see
//     only their own instance's writes.
//   - Per-id concurrency control. Concurrent saveDoc calls on the
//     same id are serialized by OPFS (each `createWritable()` claims
//     an exclusive lock on the file) but interleaving is allowed —
//     the last `close()` wins.
//   - Quota probing. Quota errors surface as the platform-thrown
//     `DOMException` re-wrapped into a `StorageError`.

import {
  type ChangeListener,
  type DocId,
  type DocMetadata,
  parseDocId,
  type StorageBackend,
  StorageError,
  type Unsubscribe,
} from './storage';
import { decideRecovery, newTxnId, type ObservedState, type WalRecord } from './wal';

const IDB_NAME = 'apalabrar-storage';
const IDB_VERSION = 1;
const METADATA_STORE = 'metadata';

const WAL_DIR = 'wal';
const BLOBS_DIR = 'blobs';

function blobFilename(id: DocId): string {
  return `${id}.bin`;
}

function walFilename(txnId: string): string {
  return `${txnId}.json`;
}

/** Wraps an IndexedDB callback API in a `Promise`. */
function openIdb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, IDB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(METADATA_STORE)) {
        db.createObjectStore(METADATA_STORE, { keyPath: 'id' });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error('indexedDB.open failed'));
  });
}

function idbRequest<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('IDB request failed'));
  });
}

function idbTxComplete(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error ?? new Error('IDB tx failed'));
    tx.onabort = () => reject(tx.error ?? new Error('IDB tx aborted'));
  });
}

async function getOptionalFileHandle(
  dir: FileSystemDirectoryHandle,
  name: string,
): Promise<FileSystemFileHandle | null> {
  try {
    return await dir.getFileHandle(name, { create: false });
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'NotFoundError') {
      return null;
    }
    throw cause;
  }
}

async function removeOptionalEntry(dir: FileSystemDirectoryHandle, name: string): Promise<void> {
  try {
    await dir.removeEntry(name);
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'NotFoundError') {
      return;
    }
    throw cause;
  }
}

async function writeFileBytes(
  dir: FileSystemDirectoryHandle,
  name: string,
  bytes: Uint8Array,
): Promise<void> {
  const handle = await dir.getFileHandle(name, { create: true });
  const stream = await handle.createWritable();
  try {
    if (bytes.length > 0) {
      // Copy into a fresh ArrayBuffer-backed view so the BlobPart
      // type narrows to `ArrayBufferView<ArrayBuffer>` (TypeScript
      // 5.7+ removed the `Uint8Array<ArrayBufferLike>` ⇒
      // `ArrayBufferView<ArrayBuffer>` assignability that the older
      // signature relied on).
      const fresh = new Uint8Array(bytes.byteLength);
      fresh.set(bytes);
      await stream.write(new Blob([fresh]));
    }
  } finally {
    // close() commits the write. Without it the file is empty even
    // after the writer goes out of scope.
    await stream.close();
  }
}

async function readFileBytes(handle: FileSystemFileHandle): Promise<Uint8Array> {
  const file = await handle.getFile();
  const buffer = await file.arrayBuffer();
  return new Uint8Array(buffer);
}

async function writeJson(
  dir: FileSystemDirectoryHandle,
  name: string,
  value: unknown,
): Promise<void> {
  const json = JSON.stringify(value);
  const bytes = new TextEncoder().encode(json);
  await writeFileBytes(dir, name, bytes);
}

async function readJson<T>(handle: FileSystemFileHandle): Promise<T> {
  const bytes = await readFileBytes(handle);
  const text = new TextDecoder().decode(bytes);
  return JSON.parse(text) as T;
}

export class OpfsStorage implements StorageBackend {
  private readonly listeners = new Set<ChangeListener>();

  private constructor(
    private readonly walDir: FileSystemDirectoryHandle,
    private readonly blobsDir: FileSystemDirectoryHandle,
    private readonly idb: IDBDatabase,
  ) {}

  /**
   * Construct + recover. The async factory pattern keeps the
   * `await navigator.storage.getDirectory()` call out of the
   * constructor (constructors cannot be async).
   */
  static async create(): Promise<OpfsStorage> {
    if (
      typeof navigator === 'undefined' ||
      !('storage' in navigator) ||
      typeof navigator.storage.getDirectory !== 'function'
    ) {
      throw new StorageError(
        'backend-unavailable',
        'OPFS is not available in this environment',
      );
    }
    const root = await navigator.storage.getDirectory();
    const walDir = await root.getDirectoryHandle(WAL_DIR, { create: true });
    const blobsDir = await root.getDirectoryHandle(BLOBS_DIR, { create: true });
    const idb = await openIdb();
    const storage = new OpfsStorage(walDir, blobsDir, idb);
    await storage.recoverWal();
    return storage;
  }

  // ----------------------------------------------------------------
  // StorageBackend
  // ----------------------------------------------------------------

  async saveDoc(id: DocId, blob: Uint8Array): Promise<void> {
    const validId = parseDocId(id);
    const txnId = newTxnId();
    const record: WalRecord = {
      txnId,
      op: 'save',
      docId: validId,
      timestamp: Date.now(),
      expectedSizeBytes: blob.length,
    };

    try {
      // 1. WAL.
      await writeJson(this.walDir, walFilename(txnId), record);
      // 2. Blob.
      await writeFileBytes(this.blobsDir, blobFilename(validId), blob);
      // 3. IDB metadata.
      await this.writeMetadata({
        id: validId,
        sizeBytes: blob.length,
        lastModified: record.timestamp,
      });
      // 4. WAL cleanup.
      await removeOptionalEntry(this.walDir, walFilename(txnId));
    } catch (cause) {
      throw this.wrapPlatformError(cause, validId, 'saveDoc');
    }

    this.notify(validId);
  }

  async loadDoc(id: DocId): Promise<Uint8Array | null> {
    const validId = parseDocId(id);
    try {
      const handle = await getOptionalFileHandle(this.blobsDir, blobFilename(validId));
      if (handle === null) return null;
      return await readFileBytes(handle);
    } catch (cause) {
      throw this.wrapPlatformError(cause, validId, 'loadDoc');
    }
  }

  async listDocs(): Promise<DocMetadata[]> {
    try {
      const tx = this.idb.transaction(METADATA_STORE, 'readonly');
      const store = tx.objectStore(METADATA_STORE);
      const rows = await idbRequest(store.getAll());
      await idbTxComplete(tx);
      return rows.map((row) => row as DocMetadata);
    } catch (cause) {
      throw this.wrapPlatformError(cause, null, 'listDocs');
    }
  }

  async deleteDoc(id: DocId): Promise<void> {
    const validId = parseDocId(id);
    const txnId = newTxnId();
    const record: WalRecord = {
      txnId,
      op: 'delete',
      docId: validId,
      timestamp: Date.now(),
    };

    try {
      await writeJson(this.walDir, walFilename(txnId), record);
      await removeOptionalEntry(this.blobsDir, blobFilename(validId));
      await this.deleteMetadata(validId);
      await removeOptionalEntry(this.walDir, walFilename(txnId));
    } catch (cause) {
      throw this.wrapPlatformError(cause, validId, 'deleteDoc');
    }

    this.notify(validId);
  }

  subscribeChanges(callback: ChangeListener): Unsubscribe {
    this.listeners.add(callback);
    let detached = false;
    return () => {
      if (detached) return;
      detached = true;
      this.listeners.delete(callback);
    };
  }

  // ----------------------------------------------------------------
  // Recovery
  // ----------------------------------------------------------------

  /** Scan the WAL directory and reconcile every residual record. */
  private async recoverWal(): Promise<void> {
    // `FileSystemDirectoryHandle.values()` is part of the File System
    // Access API but is not yet in the standard lib.dom.d.ts. The
    // cast lets us iterate without pulling in a polyfill type
    // package; if a browser ships without `values()` we fail loudly
    // here, which is fine because OPFS support implies the iterator.
    const dir = this.walDir as unknown as {
      values(): AsyncIterable<FileSystemHandle>;
    };

    // First pass: collect parsed records so we don't mutate the
    // directory while iterating it (some browser implementations are
    // sensitive to that).
    const records: { name: string; record: WalRecord; }[] = [];
    for await (const entry of dir.values()) {
      if (entry.kind !== 'file') continue;
      if (!entry.name.endsWith('.json')) continue;
      try {
        const record = await readJson<WalRecord>(entry as FileSystemFileHandle);
        records.push({ name: entry.name, record });
      } catch {
        // A WAL record we can't parse is unrecoverable; remove it so
        // it doesn't keep firing on every boot.
        await removeOptionalEntry(this.walDir, entry.name);
      }
    }

    for (const { name, record } of records) {
      const observed = await this.observeForRecovery(record);
      const decision = decideRecovery(record, observed);
      await this.applyRecovery(record, decision.action);
      await removeOptionalEntry(this.walDir, name);
    }
  }

  private async observeForRecovery(record: WalRecord): Promise<ObservedState> {
    const blobHandle = await getOptionalFileHandle(this.blobsDir, blobFilename(record.docId));
    let blobSize: number | null = null;
    if (blobHandle !== null) {
      const file = await blobHandle.getFile();
      blobSize = file.size;
    }
    const metadata = await this.readMetadata(record.docId);
    return {
      blobPresent: blobHandle !== null,
      blobSizeBytes: blobSize,
      metadataPresent: metadata !== null,
    };
  }

  private async applyRecovery(
    record: WalRecord,
    action: 'commit' | 'rollback' | 'cleanup' | 'apply-delete',
  ): Promise<void> {
    switch (action) {
      case 'cleanup':
        return; // caller deletes the WAL record afterwards
      case 'rollback':
        await removeOptionalEntry(this.blobsDir, blobFilename(record.docId));
        return;
      case 'commit': {
        const handle = await getOptionalFileHandle(
          this.blobsDir,
          blobFilename(record.docId),
        );
        if (handle === null) return; // shouldn't happen — decideRecovery says blob is present
        const file = await handle.getFile();
        await this.writeMetadata({
          id: record.docId,
          sizeBytes: file.size,
          lastModified: record.timestamp,
        });
        return;
      }
      case 'apply-delete':
        await removeOptionalEntry(this.blobsDir, blobFilename(record.docId));
        await this.deleteMetadata(record.docId);
        return;
    }
  }

  // ----------------------------------------------------------------
  // IndexedDB plumbing
  // ----------------------------------------------------------------

  private async readMetadata(id: DocId): Promise<DocMetadata | null> {
    const tx = this.idb.transaction(METADATA_STORE, 'readonly');
    const store = tx.objectStore(METADATA_STORE);
    const value = await idbRequest(store.get(id));
    await idbTxComplete(tx);
    return (value as DocMetadata | undefined) ?? null;
  }

  private async writeMetadata(meta: DocMetadata): Promise<void> {
    const tx = this.idb.transaction(METADATA_STORE, 'readwrite');
    const store = tx.objectStore(METADATA_STORE);
    store.put(meta);
    await idbTxComplete(tx);
  }

  private async deleteMetadata(id: DocId): Promise<void> {
    const tx = this.idb.transaction(METADATA_STORE, 'readwrite');
    const store = tx.objectStore(METADATA_STORE);
    store.delete(id);
    await idbTxComplete(tx);
  }

  // ----------------------------------------------------------------
  // Helpers
  // ----------------------------------------------------------------

  private notify(id: DocId): void {
    for (const listener of this.listeners) {
      try {
        listener(id);
      } catch {
        // Subscriber errors do not propagate to the writer.
      }
    }
  }

  private wrapPlatformError(cause: unknown, id: string | null, op: string): StorageError {
    if (cause instanceof StorageError) return cause;
    if (cause instanceof DOMException && cause.name === 'QuotaExceededError') {
      return new StorageError(
        'quota-exceeded',
        `OPFS quota exceeded during ${op}: ${cause.message}`,
        id,
      );
    }
    const message = cause instanceof Error ? cause.message : String(cause);
    return new StorageError('corruption', `OPFS ${op} failed: ${message}`, id);
  }
}
