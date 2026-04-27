import { render, screen } from '@solidjs/testing-library';
import { describe, expect, it } from 'vitest';
import { ComposingEditor } from './ComposingEditor';

// happy-dom drops the `data` field on CompositionEvent / InputEvent
// constructors. Patch it back in so events faithfully model the browser.
function patchData<E extends Event & { data?: unknown; }>(evt: E, data: string): E {
  if (evt.data !== data) {
    Object.defineProperty(evt, 'data', { value: data, configurable: true });
  }
  return evt;
}

const compositionEnd = (data: string): CompositionEvent =>
  patchData(new CompositionEvent('compositionend', { data }), data);

const compositionStart = (): CompositionEvent => new CompositionEvent('compositionstart');

const beforeInputInsert = (data: string): InputEvent =>
  patchData(
    new InputEvent('beforeinput', {
      inputType: 'insertText',
      data,
      cancelable: true,
      bubbles: true,
    }),
    data,
  );

const beforeInputBackspace = (): InputEvent =>
  new InputEvent('beforeinput', {
    inputType: 'deleteContentBackward',
    cancelable: true,
    bubbles: true,
  });

describe('ComposingEditor', () => {
  it('renders an empty contenteditable surface with a stable testid', () => {
    render(() => <ComposingEditor />);
    const editor = screen.getByTestId('composing-editor');
    expect(editor).toBeInTheDocument();
    expect(editor.getAttribute('contenteditable')).toBe('true');
    expect(editor.textContent).toBe('');
  });

  it('renders the composed character after a compositionend with non-empty data', () => {
    render(() => <ComposingEditor />);
    const editor = screen.getByTestId('composing-editor');

    editor.dispatchEvent(compositionStart());
    editor.dispatchEvent(compositionEnd('ñ'));

    expect(editor.textContent).toBe('ñ');
  });

  it('appends composed characters after plain typed text', () => {
    render(() => <ComposingEditor />);
    const editor = screen.getByTestId('composing-editor');

    editor.dispatchEvent(beforeInputInsert('A'));
    editor.dispatchEvent(beforeInputInsert('ñ'));
    editor.dispatchEvent(beforeInputInsert('o'));
    editor.dispatchEvent(compositionStart());
    editor.dispatchEvent(compositionEnd(' nuevo'));

    expect(editor.textContent).toBe('Año nuevo');
  });

  it('removes the last character on a deleteContentBackward beforeInput', () => {
    render(() => <ComposingEditor />);
    const editor = screen.getByTestId('composing-editor');

    editor.dispatchEvent(beforeInputInsert('a'));
    editor.dispatchEvent(beforeInputInsert('b'));
    editor.dispatchEvent(beforeInputInsert('c'));
    editor.dispatchEvent(beforeInputBackspace());

    expect(editor.textContent).toBe('ab');
  });

  it('leaves text unchanged when compositionend fires with empty data', () => {
    render(() => <ComposingEditor />);
    const editor = screen.getByTestId('composing-editor');

    editor.dispatchEvent(beforeInputInsert('h'));
    editor.dispatchEvent(beforeInputInsert('i'));
    editor.dispatchEvent(compositionStart());
    editor.dispatchEvent(compositionEnd(''));

    expect(editor.textContent).toBe('hi');
  });
});
