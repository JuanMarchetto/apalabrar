/**
 * Phase 2.5 RED tests for the `Landing` page.
 *
 * Vitest uses happy-dom which doesn't implement OPFS. The Landing
 * page allows a `bootstrap` prop to be injected for tests; the
 * production wiring uses the default `bootstrapOpfs()`.
 *
 * Coverage:
 * - Skeleton paints immediately (T+0): the blank-doc surface and
 *   the "Apalabrar" brand are visible before any async resolves.
 * - The blank-doc surface is focusable so the keystroke buffer
 *   has somewhere to listen.
 * - When recents resolve to non-empty, the toast appears.
 * - When recents resolve to empty, the toast does NOT appear.
 * - Keystrokes typed before core-ready are buffered.
 * - Once core-ready resolves, the buffer is drained.
 */
import { type DocMetadata, MemoryStorage, parseDocId } from '@apalabrar/editor-bridge';
import { cleanup, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Landing } from './Landing';

afterEach(() => cleanup());

const seedDoc = async (storage: MemoryStorage, id: string): Promise<DocMetadata> => {
  await storage.saveDoc(parseDocId(id), new Uint8Array([1, 2, 3]));
  const list = await storage.listDocs();
  return list[0]!;
};

describe('Landing', () => {
  it('renders the blank-doc skeleton immediately', () => {
    render(() => (
      <Landing
        bootstrap={() => new Promise(() => undefined)}
        bootstrapCore={() => new Promise(() => undefined)}
      />
    ));
    expect(screen.getByTestId('blank-doc')).toBeTruthy();
    expect(screen.getByText(/apalabrar/i)).toBeTruthy();
  });

  it('blank-doc surface is focusable (tabIndex >= 0)', () => {
    render(() => (
      <Landing
        bootstrap={() => new Promise(() => undefined)}
        bootstrapCore={() => new Promise(() => undefined)}
      />
    ));
    const surface = screen.getByTestId('blank-doc');
    const tabIndex = Number.parseInt(surface.getAttribute('tabindex') ?? '-1', 10);
    expect(tabIndex).toBeGreaterThanOrEqual(0);
  });

  it('shows the Continue toast when recents are non-empty', async () => {
    const memory = new MemoryStorage();
    await seedDoc(memory, 'prior');
    const bootstrap = async () => {
      const list = await memory.listDocs();
      return { storage: memory, recents: list };
    };
    render(() => <Landing bootstrap={bootstrap} bootstrapCore={() => Promise.resolve()} />);
    await waitFor(() => expect(screen.queryByRole('status')).not.toBeNull());
  });

  it('does not show the Continue toast when recents are empty', async () => {
    const bootstrap = async () => ({ storage: new MemoryStorage(), recents: [] });
    render(() => <Landing bootstrap={bootstrap} bootstrapCore={() => Promise.resolve()} />);
    // Wait for the bootstrap microtask to settle.
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('keystrokes after core-ready commit directly without going through buffer', async () => {
    const onCommit = vi.fn();
    const bootstrap = async () => ({ storage: new MemoryStorage(), recents: [] });
    render(() => (
      <Landing
        bootstrap={bootstrap}
        bootstrapCore={() => Promise.resolve()}
        onCommit={onCommit}
      />
    ));
    await waitFor(() =>
      expect(screen.getByTestId('blank-doc').getAttribute('data-core-ready'))
        .toBe('true')
    );
    const surface = screen.getByTestId('blank-doc');
    surface.focus();
    surface.dispatchEvent(new KeyboardEvent('keydown', { key: 'x', bubbles: true }));
    expect(onCommit).toHaveBeenCalledWith('x');
  });

  it('non-printable keys (Shift, Tab) are ignored by the buffer', async () => {
    const onCommit = vi.fn();
    const bootstrap = async () => ({ storage: new MemoryStorage(), recents: [] });
    render(() => (
      <Landing
        bootstrap={bootstrap}
        bootstrapCore={() => Promise.resolve()}
        onCommit={onCommit}
      />
    ));
    await waitFor(() =>
      expect(screen.getByTestId('blank-doc').getAttribute('data-core-ready'))
        .toBe('true')
    );
    const surface = screen.getByTestId('blank-doc');
    surface.focus();
    surface.dispatchEvent(new KeyboardEvent('keydown', { key: 'Shift', bubbles: true }));
    surface.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
    expect(onCommit).not.toHaveBeenCalled();
  });

  it('toast Continue forwards the doc id to onContinueDoc and hides the toast', async () => {
    const memory = new MemoryStorage();
    await seedDoc(memory, 'prior');
    const onContinueDoc = vi.fn();
    const bootstrap = async () => {
      const list = await memory.listDocs();
      return { storage: memory, recents: list };
    };
    render(() => (
      <Landing
        bootstrap={bootstrap}
        bootstrapCore={() => Promise.resolve()}
        onContinueDoc={onContinueDoc}
      />
    ));
    await waitFor(() => expect(screen.queryByRole('status')).not.toBeNull());
    const continueBtn = screen.getByRole('button', { name: /continue/i });
    continueBtn.click();
    expect(onContinueDoc).toHaveBeenCalledWith(parseDocId('prior'));
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
  });

  it('buffers keystrokes typed before core-ready and drains them on ready', async () => {
    let resolveCore!: () => void;
    const corePromise = new Promise<void>((r) => {
      resolveCore = r;
    });
    const onCommit = vi.fn();
    const bootstrap = async () => ({ storage: new MemoryStorage(), recents: [] });
    render(() => (
      <Landing
        bootstrap={bootstrap}
        bootstrapCore={() => corePromise}
        onCommit={onCommit}
      />
    ));
    const surface = screen.getByTestId('blank-doc');
    surface.focus();
    surface.dispatchEvent(new KeyboardEvent('keydown', { key: 'h', bubbles: true }));
    surface.dispatchEvent(new KeyboardEvent('keydown', { key: 'i', bubbles: true }));
    expect(onCommit).not.toHaveBeenCalled();
    resolveCore();
    await waitFor(() => expect(onCommit).toHaveBeenCalled());
    expect(onCommit.mock.calls.flat()).toEqual(['h', 'i']);
  });
});
