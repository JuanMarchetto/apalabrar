import type { Component } from 'solid-js';
import { Match, Switch } from 'solid-js';
import { ComposingEditor } from './components/ComposingEditor';
import { EditorShell } from './components/EditorShell';
import { Demo } from './pages/Demo';
import { Landing } from './pages/Landing';
import { StorageHarness } from './pages/StorageHarness';
import { Styleguide } from './pages/Styleguide';

const route = () => typeof window === 'undefined' ? '/' : window.location.pathname;

// Phase 5.11 — `/` mounts the editor itself; the dev / fixture pages
// stay reachable for hand-testing layers in isolation. The old
// "Landing" page (a blank-doc keystroke buffer placeholder) lives on
// at `/landing` until Phase 5.8 retires it for good — keeping the
// route alive avoids breaking links from the autonomous-prompts
// scaffolding.
export const App: Component = () => {
  return (
    <Switch fallback={<EditorShell />}>
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
      <Match when={route() === '/landing'}>
        <Landing />
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
