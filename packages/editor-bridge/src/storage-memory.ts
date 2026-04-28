// Reference in-memory `StorageBackend` implementation.
//
// Used by:
//   - The unit-test contract suite as the canonical "what should
//     happen" backend.
//   - Property tests as the reference model the OPFS backend's
//     observed behaviour is checked against.
//   - Environments without OPFS (eg. SSR smoke tests).
//
// Defensive-copies blobs on both save and load so callers can't
// poison the store by mutating an array they kept a reference to.
// This matches the OPFS backend's semantics, which always returns a
// fresh Uint8Array from disk.

import {
  type ChangeListener,
  type DocId,
  type DocMetadata,
  parseDocId,
  type StorageBackend,
  type Unsubscribe,
} from './storage';

interface StoredDoc {
  blob: Uint8Array;
  lastModified: number;
}

export class MemoryStorage implements StorageBackend {
  private readonly blobs = new Map<DocId, StoredDoc>();
  private readonly listeners = new Set<ChangeListener>();

  async saveDoc(id: DocId, blob: Uint8Array): Promise<void> {
    const validId = parseDocId(id);
    this.blobs.set(validId, {
      blob: new Uint8Array(blob), // copy
      lastModified: Date.now(),
    });
    this.notify(validId);
  }

  async loadDoc(id: DocId): Promise<Uint8Array | null> {
    const validId = parseDocId(id);
    const entry = this.blobs.get(validId);
    if (entry === undefined) return null;
    return new Uint8Array(entry.blob); // copy on read
  }

  async listDocs(): Promise<DocMetadata[]> {
    const result: DocMetadata[] = [];
    for (const [id, entry] of this.blobs) {
      result.push({
        id,
        sizeBytes: entry.blob.length,
        lastModified: entry.lastModified,
      });
    }
    return result;
  }

  async deleteDoc(id: DocId): Promise<void> {
    const validId = parseDocId(id);
    this.blobs.delete(validId); // idempotent: Map.delete on missing returns false silently
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

  /**
   * Fire all subscribers with the given id. Errors thrown by a
   * subscriber are isolated — the caller's saveDoc / deleteDoc
   * promise still resolves cleanly.
   */
  private notify(id: DocId): void {
    for (const listener of this.listeners) {
      try {
        listener(id);
      } catch {
        // Subscribers' errors do not propagate to the writer; the
        // storage operation is already committed by the time we
        // notify.
      }
    }
  }
}
