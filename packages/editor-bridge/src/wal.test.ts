// Pure WAL state machine tests. Exhaustively cover the (op,
// blobPresent, blobSizeMatches?, metadataPresent) lattice for the
// recovery decision so any sign-flip in `decideRecovery` flips a test.

import { describe, expect, it } from 'vitest';
import type { DocId } from './storage';
import { decideRecovery, newTxnId, type ObservedState, type WalRecord } from './wal';

const ID = 'fixture-doc' as DocId;

function saveRecord(expectedSizeBytes: number): WalRecord {
  return {
    txnId: 'txn-1',
    op: 'save',
    docId: ID,
    timestamp: 1_700_000_000,
    expectedSizeBytes,
  };
}

function deleteRecord(): WalRecord {
  return {
    txnId: 'txn-2',
    op: 'delete',
    docId: ID,
    timestamp: 1_700_000_001,
  };
}

function observed(partial: Partial<ObservedState>): ObservedState {
  return {
    blobPresent: false,
    blobSizeBytes: null,
    metadataPresent: false,
    ...partial,
  };
}

describe('decideRecovery — save flow', () => {
  it('returns rollback when crash occurred before any blob was written', () => {
    const decision = decideRecovery(saveRecord(100), observed({ blobPresent: false }));
    expect(decision.action).toBe('rollback');
    expect(decision.reason).toContain('blob');
  });

  it('returns rollback when the blob is partial (size mismatch)', () => {
    const decision = decideRecovery(
      saveRecord(100),
      observed({ blobPresent: true, blobSizeBytes: 42 }),
    );
    expect(decision.action).toBe('rollback');
    expect(decision.reason).toContain('mismatch');
  });

  it('returns commit when the blob completed but metadata never updated', () => {
    const decision = decideRecovery(
      saveRecord(100),
      observed({
        blobPresent: true,
        blobSizeBytes: 100,
        metadataPresent: false,
      }),
    );
    expect(decision.action).toBe('commit');
  });

  it('returns cleanup when blob and metadata are both present (only WAL record left)', () => {
    const decision = decideRecovery(
      saveRecord(100),
      observed({
        blobPresent: true,
        blobSizeBytes: 100,
        metadataPresent: true,
      }),
    );
    expect(decision.action).toBe('cleanup');
  });
});

describe('decideRecovery — delete flow', () => {
  it('returns apply-delete when blob still exists', () => {
    const decision = decideRecovery(
      deleteRecord(),
      observed({ blobPresent: true, blobSizeBytes: 50 }),
    );
    expect(decision.action).toBe('apply-delete');
  });

  it('returns apply-delete when only metadata still exists (blob removed first)', () => {
    const decision = decideRecovery(
      deleteRecord(),
      observed({ blobPresent: false, metadataPresent: true }),
    );
    expect(decision.action).toBe('apply-delete');
  });

  it('returns cleanup when both blob and metadata are gone', () => {
    const decision = decideRecovery(deleteRecord(), observed({}));
    expect(decision.action).toBe('cleanup');
  });
});

describe('decideRecovery — edge: zero-byte blob', () => {
  it('treats a zero-byte blob as a valid completed save when expected size is 0', () => {
    const decision = decideRecovery(
      saveRecord(0),
      observed({ blobPresent: true, blobSizeBytes: 0, metadataPresent: false }),
    );
    expect(decision.action).toBe('commit');
  });
});

describe('decideRecovery — save without expectedSizeBytes (defensive)', () => {
  it('skips the size check when expectedSizeBytes is undefined', () => {
    // Older WAL records without the size field should still be
    // recoverable without claiming a size mismatch.
    const record: WalRecord = {
      txnId: 'legacy',
      op: 'save',
      docId: ID,
      timestamp: 0,
    };
    const decision = decideRecovery(
      record,
      observed({ blobPresent: true, blobSizeBytes: 999, metadataPresent: false }),
    );
    expect(decision.action).toBe('commit');
  });
});

describe('newTxnId', () => {
  it('returns distinct ids on successive calls', () => {
    const a = newTxnId();
    const b = newTxnId();
    expect(a).not.toBe(b);
  });

  it('uses the provided clock', () => {
    const id = newTxnId(() => 1_700_000_000_000);
    // 1_700_000_000_000 in base 36 ends with a known suffix; assert
    // the id starts with the clock-derived prefix to catch any sign
    // flip in the encoding.
    const expectedPrefix = (1_700_000_000_000).toString(36);
    expect(id.startsWith(expectedPrefix)).toBe(true);
  });

  it('produces ids with a separator between clock and counter', () => {
    expect(newTxnId()).toMatch(/^[a-z0-9]+-[a-z0-9]+$/);
  });
});
