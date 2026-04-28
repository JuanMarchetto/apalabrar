// Pure WAL state machine for the OPFS storage backend.
//
// The save flow is:
//   1. Append a `WalRecord` to the WAL (status: 'pending').
//   2. Write the blob to OPFS at `blobs/<docId>.bin`.
//   3. Update the IndexedDB metadata row.
//   4. Mark the WAL record 'committed' (or delete it).
//
// A crash between any two steps leaves the system in an inconsistent
// state that recovery on next boot must reconcile. `decideRecovery`
// is the pure function that maps observed state (does the WAL record
// exist? does the blob exist? what size? does the metadata exist?)
// to a recovery action. Keeping it pure lets us unit-test every
// branch without a real browser, then trust the OPFS adapter to
// faithfully report the observed state.
//
// Action names:
//   - `commit`: write metadata to match the existing blob, then
//      delete the WAL record. Use when the blob completed but the
//      metadata update was lost.
//   - `rollback`: delete the (partial) blob and the WAL record.
//      Use when the blob is missing or its size disagrees with the
//      WAL record.
//   - `cleanup`: just delete the WAL record (the operation already
//      committed both the blob and the metadata; the WAL record was
//      written but never erased).
//   - `apply-delete`: drop the metadata row and the blob (a 'delete'
//      operation that crashed midway).

import type { DocId } from './storage';

/** A single entry in the OPFS-resident WAL. */
export interface WalRecord {
  /** Unique transaction id. Used to de-duplicate replays. */
  readonly txnId: string;
  /** What this transaction is doing. */
  readonly op: 'save' | 'delete';
  /** Document under change. */
  readonly docId: DocId;
  /** Epoch ms when the record was written. */
  readonly timestamp: number;
  /**
   * Expected on-disk size for `op === 'save'`. Used to verify the
   * blob completed flushing. Undefined for delete operations.
   */
  readonly expectedSizeBytes?: number;
}

/** What `decideRecovery` tells the OPFS adapter to do. */
export type RecoveryAction = 'commit' | 'rollback' | 'cleanup' | 'apply-delete';

export interface RecoveryDecision {
  readonly action: RecoveryAction;
  readonly reason: string;
}

/** Observable state at recovery time, fed to `decideRecovery`. */
export interface ObservedState {
  /** True if `blobs/<docId>.bin` exists in OPFS. */
  readonly blobPresent: boolean;
  /**
   * The size of the blob in bytes, or `null` if not present.
   * Used to verify the blob fully flushed before the crash.
   */
  readonly blobSizeBytes: number | null;
  /** True if a metadata row keyed by `docId` exists in IndexedDB. */
  readonly metadataPresent: boolean;
}

/**
 * Map a (record, observed-state) pair to a recovery action.
 *
 * Save flow ordering: WAL record → blob write → metadata update →
 * WAL delete. Possible crash points and their actions:
 *
 * | crash after... | blob? | meta?  | action       |
 * |----------------|-------|--------|--------------|
 * | WAL write       | no    | no     | rollback     |
 * | WAL write       | yes (size mismatch) | no | rollback (partial) |
 * | blob write      | yes (size match)    | no | commit (writes meta) |
 * | meta update     | yes   | yes    | cleanup      |
 *
 * Delete flow: WAL → blob remove → metadata remove → WAL delete.
 *
 * | crash after...   | blob? | meta?  | action       |
 * |------------------|-------|--------|--------------|
 * | WAL write         | yes   | yes    | apply-delete |
 * | blob remove       | no    | yes    | apply-delete |
 * | meta remove       | no    | no     | cleanup      |
 *
 * If we see an inconsistent state not covered above (eg. metadata
 * exists but blob is gone for a `save` op), the safe action is
 * `rollback`: the next save will replace the missing blob.
 */
export function decideRecovery(record: WalRecord, observed: ObservedState): RecoveryDecision {
  if (record.op === 'save') {
    return decideSaveRecovery(record, observed);
  }
  return decideDeleteRecovery(observed);
}

function decideSaveRecovery(record: WalRecord, observed: ObservedState): RecoveryDecision {
  const expected = record.expectedSizeBytes;
  if (!observed.blobPresent) {
    return {
      action: 'rollback',
      reason: 'save crashed before blob fully written',
    };
  }
  if (expected !== undefined && observed.blobSizeBytes !== expected) {
    return {
      action: 'rollback',
      reason: `blob size ${observed.blobSizeBytes ?? 'null'} mismatched expected ${expected}`,
    };
  }
  if (!observed.metadataPresent) {
    return {
      action: 'commit',
      reason: 'blob committed but metadata update was lost',
    };
  }
  return {
    action: 'cleanup',
    reason: 'save fully committed; only WAL record remains to remove',
  };
}

function decideDeleteRecovery(observed: ObservedState): RecoveryDecision {
  if (!observed.blobPresent && !observed.metadataPresent) {
    return {
      action: 'cleanup',
      reason: 'delete fully committed; only WAL record remains to remove',
    };
  }
  return {
    action: 'apply-delete',
    reason: 'delete crashed before all artifacts were removed',
  };
}

/**
 * Generate a fresh transaction id. Implementation note: we don't pull
 * in `crypto.randomUUID` directly because the WAL module is tested
 * in a Node environment where `crypto.randomUUID` exists but has
 * been observed to be slow under heavy fast-check fuzzing. A
 * timestamp+counter scheme is enough — the id is opaque to recovery
 * (only `txnId` uniqueness within a single recovery scan matters).
 */
let _txnCounter = 0;
export function newTxnId(now: () => number = Date.now): string {
  _txnCounter = (_txnCounter + 1) | 0;
  return `${now().toString(36)}-${_txnCounter.toString(36)}`;
}
