/**
 * Phase 2.5 — Apalabrar landing page implementing the page-load
 * timeline from blueprint Section D.
 *
 * The route is `/`. Lifecycle:
 *
 * - T+0   HTML loads, paint root shell. `index.html` inlines the
 *         critical skeleton CSS so the blank-doc surface paints
 *         before any JS runs.
 * - T+80  This component mounts; UI skeleton is now interactive
 *         (the `data-blank-doc` div is focusable and accepts input).
 * - T+100 `bootstrapCore` (WASM `init`) starts on a microtask.
 * - T+150 `bootstrap` (OPFS scan) resolves; if recents exist, the
 *         toast renders.
 * - T+200 `bootstrapCore` resolves; the keystroke buffer drains
 *         into the editor and live editing takes over.
 *
 * Anything typed during the T+100→T+200 window goes through the
 * `KeystrokeBuffer`, which `Landing` drains into the editor on
 * core-ready. This is the "no keystrokes lost" guarantee.
 *
 * The `bootstrap`, `bootstrapCore`, and `onCommit` props are
 * dependency-injection seams used by the unit tests; production
 * wiring uses the defaults (`bootstrapOpfs`, the wasm `init`,
 * and a real editor commit handler — wired in Phase 2.5 part 2
 * when the bridge consumes `RenderDelta`).
 */
import type { DocId, DocMetadata } from '@apalabrar/editor-bridge';
import { type Component, createSignal, onCleanup, onMount, Show } from 'solid-js';
import { ContinueToast } from '../components/ContinueToast';
import { RecentDocsMenu } from '../components/RecentDocsMenu';
import { KeystrokeBuffer } from '../lib/keystrokeBuffer';
import { bootstrapOpfs, type BootstrapResult } from '../lib/opfsBootstrap';

export interface LandingProps {
  readonly bootstrap?: () => Promise<BootstrapResult>;
  readonly bootstrapCore?: () => Promise<void>;
  readonly onCommit?: (key: string) => void;
  readonly onContinueDoc?: (id: DocId) => void;
}

export const Landing: Component<LandingProps> = (props) => {
  const [recents, setRecents] = createSignal<readonly DocMetadata[]>([]);
  const [toastVisible, setToastVisible] = createSignal(false);
  const [coreReady, setCoreReady] = createSignal(false);

  const buffer = new KeystrokeBuffer();

  const commit = (key: string) => {
    props.onCommit?.(key);
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key.length !== 1 && event.key !== 'Enter' && event.key !== 'Backspace') {
      return;
    }
    if (buffer.closed()) {
      commit(event.key);
    } else {
      buffer.push(event.key);
    }
  };

  const runOpfsBootstrap = async (fn: () => Promise<BootstrapResult>) => {
    const result = await fn();
    setRecents(result.recents);
    if (result.recents.length > 0) {
      setToastVisible(true);
    }
  };

  const runCoreBootstrap = async (fn: () => Promise<void>) => {
    await fn();
    for (const key of buffer.drain()) {
      commit(key);
    }
    buffer.close();
    setCoreReady(true);
  };

  onMount(() => {
    void runOpfsBootstrap(props.bootstrap ?? bootstrapOpfs);
    void runCoreBootstrap(props.bootstrapCore ?? (() => Promise.resolve()));
  });

  onCleanup(() => {
    if (!buffer.closed()) buffer.close();
  });

  return (
    <main class='min-h-screen bg-neutral-50 text-neutral-900'>
      <header class='flex items-center justify-between border-b border-neutral-200 bg-white px-6 py-3'>
        <h1 class='text-lg font-semibold tracking-tight'>Apalabrar</h1>
        <RecentDocsMenu
          docs={recents()}
          onSelect={(id) => props.onContinueDoc?.(id)}
        />
      </header>
      <div class='mx-auto max-w-3xl p-8'>
        <div
          data-testid='blank-doc'
          data-core-ready={coreReady() ? 'true' : 'false'}
          tabIndex={0}
          role='textbox'
          aria-label='Document'
          aria-multiline='true'
          class='min-h-[60vh] rounded-md border border-neutral-200 bg-white p-6 shadow-sm focus:outline-none focus:ring-2 focus:ring-neutral-900'
          onKeyDown={handleKeyDown}
        />
      </div>
      <Show when={toastVisible()}>
        <ContinueToast
          docs={recents()}
          onContinue={(id) => {
            setToastVisible(false);
            props.onContinueDoc?.(id);
          }}
          onDismiss={() => setToastVisible(false)}
        />
      </Show>
    </main>
  );
};
