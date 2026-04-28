/**
 * Phase 2.5 — keystroke buffer for the WASM-loading window.
 *
 * The page-load timeline (blueprint Section D) puts the UI skeleton
 * on screen at T+80 ms but the editor core ready at T+200 ms. If
 * the user types during that 100-200 ms window, those keystrokes
 * must NOT be lost — they're held in this buffer and replayed once
 * the editor is ready.
 *
 * Contract:
 * - `push(key)` enqueues a keystroke. Throws if the buffer is closed.
 * - `drain()` returns and clears the buffered keystrokes. Safe to
 *   call repeatedly; subsequent calls return an empty array until
 *   new keys are pushed.
 * - `close()` permanently disables `push`. Idempotent.
 * - `closed()` reports whether close was called.
 * - `size()` reports the current buffered count.
 */
export class KeystrokeBuffer {
  #keys: string[] = [];
  #closed = false;

  push(key: string): void {
    if (this.#closed) {
      throw new Error('KeystrokeBuffer is closed');
    }
    this.#keys.push(key);
  }

  drain(): readonly string[] {
    const snapshot = this.#keys;
    this.#keys = [];
    return snapshot;
  }

  size(): number {
    return this.#keys.length;
  }

  closed(): boolean {
    return this.#closed;
  }

  close(): void {
    this.#closed = true;
  }
}
