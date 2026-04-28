/**
 * Phase 2.5 RED tests for `KeystrokeBuffer`. The buffer holds
 * keystrokes during the WASM-loading window and replays them once
 * the editor core is ready. Coverage:
 *
 * - lifecycle (push/drain/size/closed/close)
 * - close idempotency
 * - push-after-close throws (caller bug — UI should already be
 *   wired through to the editor)
 * - drain post-close still works (any buffered key stays drainable)
 * - FIFO ordering
 * - empty drain returns []
 */
import { describe, expect, it } from 'vitest';
import { KeystrokeBuffer } from './keystrokeBuffer';

describe('KeystrokeBuffer', () => {
  it('starts empty, open, size 0', () => {
    const b = new KeystrokeBuffer();
    expect(b.size()).toBe(0);
    expect(b.closed()).toBe(false);
    expect(b.drain()).toEqual([]);
  });

  it('push then drain returns FIFO order', () => {
    const b = new KeystrokeBuffer();
    b.push('h');
    b.push('o');
    b.push('l');
    b.push('a');
    expect(b.size()).toBe(4);
    expect(b.drain()).toEqual(['h', 'o', 'l', 'a']);
  });

  it('drain clears the buffer; second drain is empty', () => {
    const b = new KeystrokeBuffer();
    b.push('a');
    b.push('b');
    expect(b.drain()).toEqual(['a', 'b']);
    expect(b.drain()).toEqual([]);
    expect(b.size()).toBe(0);
  });

  it('size is accurate before and after drain', () => {
    const b = new KeystrokeBuffer();
    expect(b.size()).toBe(0);
    b.push('x');
    expect(b.size()).toBe(1);
    b.push('y');
    expect(b.size()).toBe(2);
    b.drain();
    expect(b.size()).toBe(0);
  });

  it('close() flips closed() to true', () => {
    const b = new KeystrokeBuffer();
    b.close();
    expect(b.closed()).toBe(true);
  });

  it('close() is idempotent', () => {
    const b = new KeystrokeBuffer();
    b.close();
    b.close();
    expect(b.closed()).toBe(true);
  });

  it('push after close throws', () => {
    const b = new KeystrokeBuffer();
    b.close();
    expect(() => b.push('z')).toThrow();
  });

  it('keys buffered before close remain drainable after close', () => {
    const b = new KeystrokeBuffer();
    b.push('h');
    b.push('i');
    b.close();
    expect(b.drain()).toEqual(['h', 'i']);
  });

  it('drain on a closed empty buffer returns []', () => {
    const b = new KeystrokeBuffer();
    b.close();
    expect(b.drain()).toEqual([]);
  });

  it('drain returns a snapshot, not a live view of the internal buffer', () => {
    const b = new KeystrokeBuffer();
    b.push('a');
    const snap = b.drain();
    b.push('b');
    expect(snap).toEqual(['a']);
  });
});
