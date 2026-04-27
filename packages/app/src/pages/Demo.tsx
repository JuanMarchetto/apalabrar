import { closeDoc, docText, init, openDocx, wasmUrl } from '@apalabrar/editor-bridge';
import { type Component, createSignal, onMount, Show } from 'solid-js';

// Vite-import the canonical fixture from tests-corpus/. The `?url` suffix
// emits the file as a build asset and gives us a hashed URL we can fetch
// at runtime. `vite.config.ts` opens `server.fs.allow` to the workspace
// root so the dev server can serve files from outside packages/app/.
import sampleUrl from '../../../../tests-corpus/demo/sample.docx?url';

export const Demo: Component = () => {
  const [text, setText] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      // Pass the wasm URL explicitly. The wasm-pack glue's default
      // resolution (`new URL(..., import.meta.url)`) ends up pointing
      // at the bundled JS chunk under Vite, not the .wasm asset, which
      // makes the browser fetch index.html instead of the binary and
      // throw "WebAssembly.instantiate(): expected magic word ...".
      await init({ module_or_path: wasmUrl });
      const response = await fetch(sampleUrl);
      const bytes = new Uint8Array(await response.arrayBuffer());
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
          Apalabrar — Validation Gate 2 Demo
        </h1>
        <p class='mt-1 text-sm text-neutral-500'>
          WASM core: <code>open_docx</code> → <code>doc_text</code>{' '}
          a través de Loro CRDT + docx-rs sobre WebAssembly.
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
