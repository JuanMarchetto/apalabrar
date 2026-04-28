// Storage layer for Apalabrar documents.
//
// Two backends ship from this package, both behind the same
// `StorageBackend` interface:
//
//   - `MemoryStorage` (`storage-memory.ts`): pure in-memory `Map`.
//      Useful as a contract reference, in tests, and for environments
//      without OPFS (eg. Server-Side Rendering smoke tests).
//   - `OpfsStorage` (`storage-opfs.ts`): production. Blobs in OPFS,
//      metadata in IndexedDB, a WAL on OPFS for crash-recovery between
//      blob write and metadata commit.
//
// Both share the same set of contract tests in
// `storage-memory.test.ts` and a fast-check property suite in
// `storage-properties.test.ts`. OPFS-specific behaviour (real
// `navigator.storage.getDirectory()` round-trips, kill-the-tab crash
// recovery) is exercised end-to-end in
// `tests-e2e/tests/storage.spec.ts`.
//
// The pure WAL state machine lives in `wal.ts` and is a function
// `decideRecovery(record, blobPresent, blobSize, metadataPresent)`.
// Keeping recovery decisions pure means the OPFS backend's
// constructor-time replay can be verified without a real browser.

/**
 * A document identifier. Branded so callers can't pass arbitrary
 * strings without going through `parseDocId`. Validated to match
 * `^[a-zA-Z0-9_-]{1,128}$` so it's safe as both an OPFS file name and
 * an IndexedDB key.
 */
export type DocId = string & { readonly __brand: 'DocId'; };

/** Metadata returned by `listDocs`. Mirrors the IndexedDB record. */
export interface DocMetadata {
  /** The document identifier. */
  readonly id: DocId;
  /** Size in bytes of the most recently-committed blob. */
  readonly sizeBytes: number;
  /** Epoch milliseconds at which the most recent commit completed. */
  readonly lastModified: number;
}

/** A handle returned by `subscribeChanges` that detaches the callback. */
export type Unsubscribe = () => void;

/** Callback signature for `subscribeChanges`. */
export type ChangeListener = (id: DocId) => void;

/**
 * Storage contract that any backend (Memory, OPFS, or future remote
 * backends) must satisfy. All methods are asynchronous; concurrent
 * calls on the same `id` are serialized internally so callers can
 * fire-and-forget without worrying about overlap.
 */
export interface StorageBackend {
  /**
   * Persist `blob` under `id`. Overwrites any existing document with
   * the same id. Resolves once the write is durable (blob committed
   * and metadata visible to a fresh `listDocs` call).
   *
   * @throws `StorageError` with `kind: 'invalid-id'` if `id` doesn't
   *   parse, or `kind: 'quota-exceeded'` if the platform refuses the
   *   write.
   */
  saveDoc(id: DocId, blob: Uint8Array): Promise<void>;

  /**
   * Read the document stored under `id`. Resolves to `null` if no
   * such document exists. Does NOT throw on missing documents.
   *
   * @throws `StorageError` with `kind: 'invalid-id'` if `id` doesn't
   *   parse, or `kind: 'corruption'` if the blob and metadata
   *   disagree on size.
   */
  loadDoc(id: DocId): Promise<Uint8Array | null>;

  /**
   * List all currently-committed documents. Order is unspecified —
   * callers should sort by whatever criterion they need.
   */
  listDocs(): Promise<DocMetadata[]>;

  /**
   * Remove the document stored under `id`. Idempotent — deleting a
   * non-existent document succeeds silently. Subscribers are notified.
   *
   * @throws `StorageError` with `kind: 'invalid-id'` if `id` doesn't
   *   parse.
   */
  deleteDoc(id: DocId): Promise<void>;

  /**
   * Register a callback that fires once after every successful
   * `saveDoc` or `deleteDoc` on this backend instance. The
   * notification is fired AFTER the operation has been committed —
   * subscribers can immediately call `loadDoc` and observe the new
   * state.
   *
   * Returns a function that detaches the callback. Calling it more
   * than once is a no-op.
   *
   * Cross-tab notifications are out of scope for this interface;
   * `OpfsStorage` may layer a `BroadcastChannel` on top in a future
   * iteration but the contract here covers single-instance only.
   */
  subscribeChanges(callback: ChangeListener): Unsubscribe;
}

/**
 * Discriminated error union. Backends throw `StorageError` (a regular
 * `Error` subclass with a `kind` field) so callers can `catch` and
 * branch on `kind` without `instanceof`-ing per backend.
 */
export type StorageErrorKind =
  | 'invalid-id'
  | 'quota-exceeded'
  | 'corruption'
  | 'backend-unavailable';

export class StorageError extends Error {
  readonly kind: StorageErrorKind;
  readonly id: string | null;

  constructor(kind: StorageErrorKind, message: string, id: string | null = null) {
    super(message);
    this.name = 'StorageError';
    this.kind = kind;
    this.id = id;
  }
}

const DOC_ID_PATTERN = /^[a-zA-Z0-9_-]{1,128}$/;

/**
 * Validate a candidate document id. Returns the branded `DocId` if
 * valid, or throws `StorageError({kind: 'invalid-id'})` otherwise.
 *
 * Accepts: 1–128 chars of `[A-Za-z0-9_-]`. Rejects empty strings,
 * path separators, dots (no `..` traversal), spaces, unicode.
 *
 * Rationale: doc ids are used as OPFS file names and IndexedDB keys.
 * The regex picks the intersection of "safe everywhere" so we don't
 * have to worry about platform escaping rules.
 */
export function parseDocId(candidate: string): DocId {
  if (typeof candidate !== 'string' || !DOC_ID_PATTERN.test(candidate)) {
    throw new StorageError(
      'invalid-id',
      `Invalid DocId ${JSON.stringify(candidate)}: must match ${DOC_ID_PATTERN.source}`,
      typeof candidate === 'string' ? candidate : null,
    );
  }
  return candidate as DocId;
}

/** Type-guard variant of `parseDocId`. */
export function isDocId(candidate: unknown): candidate is DocId {
  return typeof candidate === 'string' && DOC_ID_PATTERN.test(candidate);
}
