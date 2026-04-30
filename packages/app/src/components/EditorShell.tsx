// Phase 5.11 — EditorShell.
//
// Mounted at the root route. Owns the active document state — wasm
// core, doc id, source format, and the current `RenderPlan` —
// plus the read paths the painter needs. v0 ships read-only:
// open a docx/md/html/rtf/odt, render its plain-text projection
// against the Phase 4 layout engine, save back to the same format
// (or any other supported one). The input layer in Phase 5.8 wires
// keystrokes on top of this same state machine.
//
// Test seam: `core` may be passed in directly. Production callers
// omit it and the component awaits `initWasm()` on mount. The test
// suite uses the prop to inject a stub `ApalabrarCore` so vitest
// never has to load the real wasm binary.

import {
  type ApalabrarCore,
  type CoreDocId,
  type DocFormat,
  initWasm,
  LETTER_AT_96DPI,
  type RenderPlan,
  type Viewport,
} from '@apalabrar/editor-bridge';
import { type Component, createSignal, onMount, Show, untrack } from 'solid-js';

import { detectFormat, mimeFor } from './detectFormat';
import { RenderPlanView } from './RenderPlanView';

export interface EditorShellProps {
  /** Test seam — when supplied, skips `initWasm()` on mount. */
  readonly core?: ApalabrarCore;
  /** Override the page geometry. Defaults to `LETTER_AT_96DPI`. */
  readonly viewport?: Viewport;
}

const ACCEPTED_EXTENSIONS = '.docx,.md,.markdown,.html,.htm,.rtf,.odt';

export const EditorShell: Component<EditorShellProps> = (props) => {
  // `props.core` is the test-only injection seam — production callers
  // omit it. Reading it once via `untrack` documents the intentional
  // non-reactivity and silences the `solid/reactivity` lint that
  // would otherwise flag the bare prop read at signal-init time.
  const [core, setCore] = createSignal<ApalabrarCore | null>(
    untrack(() => props.core ?? null),
  );
  const [docId, setDocId] = createSignal<CoreDocId | null>(null);
  const [docName, setDocName] = createSignal('');
  const [docFormat, setDocFormat] = createSignal<DocFormat>('docx');
  const [plan, setPlan] = createSignal<RenderPlan | null>(null);
  const [docTextSnapshot, setDocTextSnapshot] = createSignal('');
  const [error, setError] = createSignal<string | null>(null);

  const viewport = (): Viewport => props.viewport ?? LETTER_AT_96DPI;

  onMount(() => {
    if (core() !== null) return;
    void (async () => {
      try {
        const c = await initWasm();
        setCore(c);
      } catch (e) {
        setError(`Failed to initialise editor core: ${String(e)}`);
      }
    })();
  });

  async function handleFile(file: File): Promise<void> {
    const c = core();
    if (!c) {
      setError('Editor core not initialised yet — wait a moment and retry.');
      return;
    }
    const format = detectFormat(file.name);
    if (!format) {
      setError(`Unsupported file extension: ${file.name}`);
      return;
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const id = c.openDoc(bytes, format);
      setDocId(id);
      setDocName(file.name);
      setDocFormat(format);
      setPlan(c.layout(id, viewport()));
      setDocTextSnapshot(c.docText(id));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  function handleSave(): void {
    const c = core();
    const id = docId();
    if (!c || !id) return;
    try {
      const bytes = c.toFormat(id, docFormat());
      // `Blob` rejects `Uint8Array<ArrayBufferLike>` under strict
      // TypeScript libs (the buffer might be a `SharedArrayBuffer`
      // in some contexts). Slicing produces a fresh
      // `Uint8Array<ArrayBuffer>`, which the Blob constructor
      // accepts without further casting.
      const blob = new Blob([bytes.slice()], { type: mimeFor(docFormat()) });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = docName() || `document.${defaultExt(docFormat())}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main class='apalabrar-editor min-h-screen bg-neutral-100' data-testid='editor-shell'>
      <header class='flex items-center gap-3 border-b border-neutral-300 bg-white px-6 py-3'>
        <h1 class='text-lg font-semibold tracking-tight'>Apalabrar</h1>
        <div class='flex flex-1 items-center gap-2'>
          <label class='cursor-pointer rounded bg-neutral-900 px-3 py-1.5 text-sm font-medium text-white hover:bg-neutral-700'>
            Open document
            <input
              type='file'
              class='hidden'
              accept={ACCEPTED_EXTENSIONS}
              data-testid='file-picker'
              onChange={(e) => {
                const file = e.currentTarget.files?.[0];
                if (file) void handleFile(file);
              }}
            />
          </label>
          <button
            type='button'
            class='rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-800 hover:bg-neutral-50 disabled:opacity-40'
            data-testid='save-button'
            disabled={docId() === null}
            onClick={() => handleSave()}
          >
            Download
          </button>
          <Show when={docName()} keyed>
            {(name) => (
              <span class='ml-2 text-sm text-neutral-500' data-testid='doc-name'>{name}</span>
            )}
          </Show>
        </div>
      </header>
      <Show when={error()} keyed>
        {(msg) => (
          <div
            class='mx-auto mt-4 max-w-3xl rounded border border-red-300 bg-red-50 p-4 text-red-700'
            data-testid='editor-error'
          >
            {msg}
          </div>
        )}
      </Show>
      <Show
        when={plan()}
        keyed
        fallback={
          <div
            class='flex min-h-[60vh] items-center justify-center text-neutral-500'
            data-testid='editor-empty'
          >
            <p>Open a document (.docx, .md, .html, .rtf, .odt) to begin.</p>
          </div>
        }
      >
        {(p) => (
          <div class='py-6'>
            <RenderPlanView plan={p} text={docTextSnapshot()} viewport={viewport()} />
          </div>
        )}
      </Show>
    </main>
  );
};

function defaultExt(format: DocFormat): string {
  return format === 'markdown' ? 'md' : format;
}
