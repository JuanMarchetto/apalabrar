import type { Component } from 'solid-js';
import { Match, Switch } from 'solid-js';
import { ComposingEditor } from './components/ComposingEditor';
import { Demo } from './pages/Demo';

const route = () => typeof window === 'undefined' ? '/' : window.location.pathname;

export const App: Component = () => {
  return (
    <main class='min-h-screen bg-neutral-50 text-neutral-900'>
      <Switch
        fallback={
          <div class='flex min-h-screen flex-col items-center justify-center space-y-2 text-center'>
            <h1 class='text-4xl font-semibold tracking-tight'>Apalabrar</h1>
            <p class='text-neutral-600'>
              Coming soon — el editor de documentos académicos.
            </p>
            <p class='text-xs text-neutral-400'>
              <a class='underline' href='/composer'>/composer</a>
              {' · '}
              <a class='underline' href='/demo'>/demo</a>
            </p>
          </div>
        }
      >
        <Match when={route() === '/composer'}>
          <div class='flex min-h-screen items-center justify-center'>
            <ComposingEditor />
          </div>
        </Match>
        <Match when={route() === '/demo'}>
          <Demo />
        </Match>
      </Switch>
    </main>
  );
};
