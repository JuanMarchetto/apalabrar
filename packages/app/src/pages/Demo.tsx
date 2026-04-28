import {
  closeDoc,
  type DocId,
  type DocMetadata,
  docText,
  init,
  openDocx,
  OpfsStorage,
  parseDocId,
  wasmUrl,
} from '@apalabrar/editor-bridge';
import { type Component, createSignal, For, onMount, Show } from 'solid-js';

// Vite-import the canonical fixture from tests-corpus/. The `?url` suffix
// emits the file as a build asset and gives us a hashed URL we can fetch
// at runtime. `vite.config.ts` opens `server.fs.allow` to the workspace
// root so the dev server can serve files from outside packages/app/.
import sampleUrl from '../../../../tests-corpus/demo/sample.docx?url';

export const Demo: Component = () => {
  const [text, setText] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [savedList, setSavedList] = createSignal<DocMetadata[]>([]);
  const [storage, setStorage] = createSignal<OpfsStorage | null>(null);
  const [currentBytes, setCurrentBytes] = createSignal<Uint8Array | null>(null);
  const [savedAt, setSavedAt] = createSignal<string | null>(null);

  const SLOT_ID: DocId = parseDocId('demo-current');

  async function refreshList(s: OpfsStorage): Promise<void> {
    const list = await s.listDocs();
    setSavedList(list);
  }

  async function handleSave(): Promise<void> {
    const s = storage();
    const bytes = currentBytes();
    if (!s || !bytes) return;
    await s.saveDoc(SLOT_ID, bytes);
    setSavedAt(new Date().toLocaleTimeString());
    await refreshList(s);
  }

  async function handleLoad(): Promise<void> {
    const s = storage();
    if (!s) return;
    const bytes = await s.loadDoc(SLOT_ID);
    if (bytes === null) {
      setError(`No saved document at ${SLOT_ID}`);
      return;
    }
    setCurrentBytes(bytes);
    const docId = openDocx(bytes);
    try {
      setText(docText(docId));
      setError(null);
    } finally {
      closeDoc(docId);
    }
  }

  async function handleDelete(): Promise<void> {
    const s = storage();
    if (!s) return;
    await s.deleteDoc(SLOT_ID);
    setSavedAt(null);
    await refreshList(s);
  }

  onMount(async () => {
    try {
      // Pass the wasm URL explicitly. The wasm-pack glue's default
      // resolution (`new URL(..., import.meta.url)`) ends up pointing
      // at the bundled JS chunk under Vite, not the .wasm asset, which
      // makes the browser fetch index.html instead of the binary and
      // throw "WebAssembly.instantiate(): expected magic word ...".
      await init({ module_or_path: wasmUrl });

      const s = await OpfsStorage.create();
      setStorage(s);
      await refreshList(s);

      const response = await fetch(sampleUrl);
      const bytes = new Uint8Array(await response.arrayBuffer());
      setCurrentBytes(bytes);
      const docId = openDocx(bytes);
      try {
        setText(docText(docId));
      } finally {
        closeDoc(docId);
      }
    } catch (cause) {
      // Surface the error to the user instead of failing silently. The
      // Demo page is a smoke test of the full editor-core pipeline; if
      // any layer breaks, this is where it shows up first.
      setError(String(cause));
    }
  });

  return (
    <main class='mx-auto max-w-3xl space-y-4 p-8'>
      <header>
        <h1 class='text-2xl font-semibold tracking-tight'>
          Apalabrar — Validation Gate 2 + 2.1 Demo
        </h1>
        <p class='mt-1 text-sm text-neutral-500'>
          WASM core <code>open_docx → doc_text</code> + persistencia local con{' '}
          <code>OpfsStorage</code> (OPFS + IndexedDB + WAL).
        </p>
      </header>

      <Show
        when={text() !== null}
        fallback={
          <Show
            when={error() === null}
            fallback={
              <div
                class='rounded border border-red-300 bg-red-50 p-4 text-red-700'
                data-testid='demo-error'
              >
                Error cargando el documento: {error()}
              </div>
            }
          >
            <div class='text-neutral-500' data-testid='demo-loading'>
              Cargando WASM…
            </div>
          </Show>
        }
      >
        <Show when={storage() !== null}>
          <section
            class='flex flex-wrap items-center gap-2 rounded border border-neutral-300 bg-white p-4 text-sm'
            data-testid='demo-storage-controls'
          >
            <button
              class='rounded bg-neutral-900 px-3 py-1.5 font-medium text-white hover:bg-neutral-700 disabled:opacity-50'
              data-testid='demo-save-button'
              disabled={currentBytes() === null}
              onClick={() => handleSave()}
              type='button'
            >
              Save to OPFS
            </button>
            <button
              class='rounded border border-neutral-300 px-3 py-1.5 font-medium text-neutral-800 hover:bg-neutral-50'
              data-testid='demo-load-button'
              onClick={() => handleLoad()}
              type='button'
            >
              Load from OPFS
            </button>
            <button
              class='rounded border border-red-300 px-3 py-1.5 font-medium text-red-700 hover:bg-red-50'
              data-testid='demo-delete-button'
              onClick={() => handleDelete()}
              type='button'
            >
              Delete saved
            </button>
            <Show when={savedAt() !== null}>
              <span class='text-neutral-500' data-testid='demo-saved-at'>
                Last save: {savedAt()}
              </span>
            </Show>
          </section>

          <section
            class='rounded border border-neutral-200 bg-neutral-50 p-4 text-xs font-mono'
            data-testid='demo-storage-list'
          >
            <div class='mb-1 font-sans text-sm font-medium text-neutral-700'>
              Saved documents ({savedList().length})
            </div>
            <Show
              when={savedList().length > 0}
              fallback={<div class='text-neutral-500'>(none yet)</div>}
            >
              <ul class='list-disc pl-5'>
                <For each={savedList()}>
                  {(meta) => (
                    <li>
                      <code>{meta.id}</code> — {meta.sizeBytes} bytes,{' '}
                      {new Date(meta.lastModified).toISOString()}
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </section>
        </Show>

        <article
          class='whitespace-pre-wrap rounded border border-neutral-300 bg-neutral-50 p-6 leading-relaxed'
          data-testid='demo-rendered-text'
        >
          {text()}
        </article>
      </Show>
    </main>
  );
};
