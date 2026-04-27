import type { Component } from 'solid-js';
import { Show } from 'solid-js';
import { ComposingEditor } from './components/ComposingEditor';

const isComposerRoute = () =>
  typeof window !== 'undefined' && window.location.pathname === '/composer';

export const App: Component = () => {
  return (
    <main class='min-h-screen flex items-center justify-center bg-neutral-50 text-neutral-900'>
      <Show
        when={isComposerRoute()}
        fallback={
          <div class='text-center space-y-2'>
            <h1 class='text-4xl font-semibold tracking-tight'>Apalabrar</h1>
            <p class='text-neutral-600'>Coming soon — el editor de documentos académicos.</p>
          </div>
        }
      >
        <ComposingEditor />
      </Show>
    </main>
  );
};
