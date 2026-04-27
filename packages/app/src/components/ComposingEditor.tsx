import type { Component } from 'solid-js';

// Stub. The composition event handlers are wired in the GREEN phase.
// data-testid is the stable handle Playwright e2e tests bind to.
export const ComposingEditor: Component = () => {
  return (
    <div
      data-testid='composing-editor'
      contentEditable={true}
      class='min-h-[8rem] w-full max-w-2xl rounded border border-neutral-300 p-4 outline-none focus:border-neutral-500'
    />
  );
};
