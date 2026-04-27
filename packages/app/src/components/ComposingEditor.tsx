import { type Component, createSignal } from 'solid-js';
import { applyComposition } from './composition';

export const ComposingEditor: Component = () => {
  const [text, setText] = createSignal('');

  // Snapshot of `text()` taken at compositionstart. The browser commits the
  // composed string into the contenteditable's DOM before compositionend
  // fires, so applyComposition needs the *pre*-composition state to avoid
  // double-counting.
  let textBeforeComposition = '';

  const onCompositionStart = () => {
    textBeforeComposition = text();
  };

  const onCompositionEnd = (e: CompositionEvent) => {
    setText(applyComposition(textBeforeComposition, e));
  };

  // Controlled input for non-composition keystrokes. Without this, plain
  // typing would leave text() out of sync with the DOM and the next
  // composition would clobber what the user typed.
  const onBeforeInput = (e: InputEvent) => {
    if (e.isComposing) return;
    if (e.inputType === 'insertText' && e.data !== null) {
      e.preventDefault();
      setText(text() + e.data);
    } else if (e.inputType === 'deleteContentBackward') {
      e.preventDefault();
      setText(text().slice(0, -1));
    }
  };

  return (
    <div
      data-testid='composing-editor'
      contentEditable={true}
      onBeforeInput={onBeforeInput}
      onCompositionStart={onCompositionStart}
      onCompositionEnd={onCompositionEnd}
      class='min-h-[8rem] w-full max-w-2xl rounded border border-neutral-300 p-4 outline-none focus:border-neutral-500'
    >
      {text()}
    </div>
  );
};
