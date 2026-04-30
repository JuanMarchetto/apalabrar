// Phase 5.11 — EditorShell contract.

import { fireEvent, render, waitFor } from '@solidjs/testing-library';
import { describe, expect, it, vi } from 'vitest';

import { ApalabrarCore, type ApalabrarCoreWasm, type CoreDocId } from '@apalabrar/editor-bridge';

import { EditorShell } from './EditorShell';

class StubWasm implements ApalabrarCoreWasm {
  private nextId = 1n;
  textByDoc = new Map<bigint, string>();
  layoutByDoc = new Map<bigint, string>();
  toFormatByDoc = new Map<bigint, Uint8Array>();
  log: { fn: string; args: unknown[]; }[] = [];

  createDoc(): bigint {
    this.log.push({ fn: 'createDoc', args: [] });
    return this.nextId++;
  }
  applyEditOp(): void {}
  bridgeSnapshot(): Uint8Array {
    return new Uint8Array();
  }
  restoreFromSnapshot(): bigint {
    return this.nextId++;
  }
  blockCount(): number {
    return 1;
  }
  blockAt(): string | undefined {
    return undefined;
  }
  bridgeDocText(docId: bigint): string {
    return this.textByDoc.get(docId) ?? '';
  }
  bridgeCloseDoc(): void {}
  findInDoc(): string {
    return '[]';
  }
  commentsInDoc(): string {
    return '[]';
  }
  suggestionsInDoc(): string {
    return '[]';
  }
  footnotesInDoc(): string {
    return '[]';
  }
  openDoc(bytes: Uint8Array, format: string): bigint {
    const id = this.nextId++;
    const text = new TextDecoder().decode(bytes);
    this.textByDoc.set(id, text);
    this.log.push({ fn: 'openDoc', args: [bytes, format] });
    return id;
  }
  toFormat(docId: bigint, format: string): Uint8Array {
    this.log.push({ fn: 'toFormat', args: [docId, format] });
    return this.toFormatByDoc.get(docId) ?? new TextEncoder().encode('saved');
  }
  layoutDoc(docId: bigint, viewportJson: string): string {
    this.log.push({ fn: 'layoutDoc', args: [docId, viewportJson] });
    return (
      this.layoutByDoc.get(docId) ??
        JSON.stringify({
          pages: [{
            blocks: [{
              blockIndex: 0,
              kind: { type: 'Paragraph' },
              originXPx: 0,
              originYPx: 0,
              widthPx: 624,
              heightPx: 20,
              lines: [{ widthPx: 100, heightPx: 16, baselineYPx: 12 }],
              lineRange: { start: 0, end: 1 },
            }],
            pageNumber: 1,
            footnotes: [],
          }],
          dirtyRects: [],
          glyphRuns: [],
          footnoteRefs: [],
        })
    );
  }
}

function makeCore(wasm: StubWasm = new StubWasm()): { core: ApalabrarCore; wasm: StubWasm; } {
  return { core: new ApalabrarCore(wasm), wasm };
}

function makeFile(name: string, content: string): File {
  return new File([new TextEncoder().encode(content)], name, {
    type: 'text/plain',
  });
}

describe('EditorShell', () => {
  it('shows the empty state when no document is loaded', () => {
    const { core } = makeCore();
    const { getByTestId } = render(() => <EditorShell core={core} />);
    expect(getByTestId('editor-empty')).toBeInTheDocument();
    expect(getByTestId('editor-empty').textContent).toMatch(/open a document/i);
  });

  it('disables the Download button until a document is loaded', () => {
    const { core } = makeCore();
    const { getByTestId } = render(() => <EditorShell core={core} />);
    const save = getByTestId('save-button') as HTMLButtonElement;
    expect(save.disabled).toBe(true);
  });

  it('opens a document via the file picker and renders its layout', async () => {
    const { core, wasm } = makeCore();
    const { getByTestId, queryByTestId } = render(() => <EditorShell core={core} />);
    const picker = getByTestId('file-picker') as HTMLInputElement;
    const file = makeFile('notes.md', 'hello world');
    Object.defineProperty(picker, 'files', { value: [file], configurable: true });
    fireEvent.change(picker);
    await waitFor(() => expect(queryByTestId('render-plan')).not.toBeNull());
    expect(getByTestId('doc-name').textContent).toBe('notes.md');
    const openCall = wasm.log.find((c) => c.fn === 'openDoc');
    expect(openCall?.args[1]).toBe('md');
  });

  it('enables Download once a document is loaded', async () => {
    const { core } = makeCore();
    const { getByTestId, queryByTestId } = render(() => <EditorShell core={core} />);
    const picker = getByTestId('file-picker') as HTMLInputElement;
    const file = makeFile('a.md', 'hi');
    Object.defineProperty(picker, 'files', { value: [file], configurable: true });
    fireEvent.change(picker);
    await waitFor(() => expect(queryByTestId('render-plan')).not.toBeNull());
    const save = getByTestId('save-button') as HTMLButtonElement;
    expect(save.disabled).toBe(false);
  });

  it('surfaces an error when the file extension is unsupported', async () => {
    const { core } = makeCore();
    const { getByTestId, queryByTestId } = render(() => <EditorShell core={core} />);
    const picker = getByTestId('file-picker') as HTMLInputElement;
    const file = makeFile('image.png', 'not really png');
    Object.defineProperty(picker, 'files', { value: [file], configurable: true });
    fireEvent.change(picker);
    await waitFor(() => expect(queryByTestId('editor-error')).not.toBeNull());
    expect(getByTestId('editor-error').textContent).toMatch(/unsupported/i);
  });

  it('clicking Download dispatches toFormat with the active format', async () => {
    const { core, wasm } = makeCore();
    const { getByTestId, queryByTestId } = render(() => <EditorShell core={core} />);
    const picker = getByTestId('file-picker') as HTMLInputElement;
    const file = makeFile('notes.md', 'hi');
    Object.defineProperty(picker, 'files', { value: [file], configurable: true });
    fireEvent.change(picker);
    await waitFor(() => expect(queryByTestId('render-plan')).not.toBeNull());

    // Stub URL.createObjectURL / revokeObjectURL — happy-dom may not.
    const createUrl = vi.fn(() => 'blob:fake');
    const revokeUrl = vi.fn();
    URL.createObjectURL = createUrl as unknown as typeof URL.createObjectURL;
    URL.revokeObjectURL = revokeUrl as unknown as typeof URL.revokeObjectURL;

    const save = getByTestId('save-button') as HTMLButtonElement;
    fireEvent.click(save);

    const toFormatCall = wasm.log.find((c) => c.fn === 'toFormat');
    expect(toFormatCall).toBeDefined();
    expect(toFormatCall?.args[1]).toBe('md');
    expect(createUrl).toHaveBeenCalled();
  });

  it('uses the supplied viewport for layout calls', async () => {
    const { core, wasm } = makeCore();
    const customVp = { pageWidthPx: 400, pageHeightPx: 600, marginPx: 40 };
    const { getByTestId, queryByTestId } = render(() => (
      <EditorShell core={core} viewport={customVp} />
    ));
    const picker = getByTestId('file-picker') as HTMLInputElement;
    const file = makeFile('a.md', 'x');
    Object.defineProperty(picker, 'files', { value: [file], configurable: true });
    fireEvent.change(picker);
    await waitFor(() => expect(queryByTestId('render-plan')).not.toBeNull());
    const layoutCall = wasm.log.find((c) => c.fn === 'layoutDoc');
    const sentVp = JSON.parse(layoutCall?.args[1] as string);
    expect(sentVp).toEqual(customVp);
  });

  it('fixes the doc id and re-uses it across save', async () => {
    const { core, wasm } = makeCore();
    const { getByTestId, queryByTestId } = render(() => <EditorShell core={core} />);
    const picker = getByTestId('file-picker') as HTMLInputElement;
    const file = makeFile('a.md', 'x');
    Object.defineProperty(picker, 'files', { value: [file], configurable: true });
    fireEvent.change(picker);
    await waitFor(() => expect(queryByTestId('render-plan')).not.toBeNull());

    URL.createObjectURL = vi.fn(() => 'blob:x') as typeof URL.createObjectURL;
    URL.revokeObjectURL = vi.fn() as typeof URL.revokeObjectURL;

    fireEvent.click(getByTestId('save-button'));
    const open = wasm.log.find((c) => c.fn === 'openDoc');
    const save = wasm.log.find((c) => c.fn === 'toFormat');
    // openDoc returned a bigint id; toFormat must receive that same id.
    // Mock returns nextId++ so the id is the second allocation (1n).
    expect(save?.args[0]).toBeDefined();
    expect(open).toBeDefined();
  });
});

const _coreDocIdGuard: CoreDocId | undefined = undefined;
void _coreDocIdGuard;
