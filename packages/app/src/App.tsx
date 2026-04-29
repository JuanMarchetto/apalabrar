import type { Component } from 'solid-js';
import { Match, Switch } from 'solid-js';
import { ComposingEditor } from './components/ComposingEditor';
import { Demo } from './pages/Demo';
import { Landing } from './pages/Landing';
import { StorageHarness } from './pages/StorageHarness';
import { Styleguide } from './pages/Styleguide';

const route = () => typeof window === 'undefined' ? '/' : window.location.pathname;

export const App: Component = () => {
  return (
    <Switch fallback={<Landing />}>
      <Match when={route() === '/composer'}>
        <main class='min-h-screen bg-neutral-50 text-neutral-900'>
          <div class='flex min-h-screen items-center justify-center'>
            <ComposingEditor />
          </div>
        </main>
      </Match>
      <Match when={route() === '/demo'}>
        <Demo />
      </Match>
      <Match when={route() === '/storage-harness'}>
        <StorageHarness />
      </Match>
      <Match when={route() === '/styleguide'}>
        <Styleguide />
      </Match>
    </Switch>
  );
};
