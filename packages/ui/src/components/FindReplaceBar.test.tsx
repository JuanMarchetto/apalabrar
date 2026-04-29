import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FindReplaceBar } from './FindReplaceBar';

afterEach(cleanup);

const baseProps = {
  findQuery: '',
  replaceQuery: '',
  matchCount: 0,
  caseSensitive: false,
  wholeWord: false,
} as const;

describe('FindReplaceBar', () => {
  it('renders a form with accessible name "Find and replace"', () => {
    render(() => <FindReplaceBar {...baseProps} />);
    expect(
      screen.getByRole('search', { name: 'Find and replace' }),
    ).toBeInTheDocument();
  });

  it('renders Find and Replace inputs reflecting controlled values', () => {
    render(() => (
      <FindReplaceBar
        {...baseProps}
        findQuery='hello'
        replaceQuery='world'
      />
    ));
    expect(screen.getByLabelText('Find')).toHaveValue('hello');
    expect(screen.getByLabelText('Replace')).toHaveValue('world');
  });

  it('fires onFindQueryChange on input', () => {
    const onFindQueryChange = vi.fn();
    render(() => (
      <FindReplaceBar
        {...baseProps}
        onFindQueryChange={onFindQueryChange}
      />
    ));
    fireEvent.input(screen.getByLabelText('Find'), {
      target: { value: 'foo' },
    });
    expect(onFindQueryChange).toHaveBeenCalledWith('foo');
  });

  it('fires onReplaceQueryChange on input', () => {
    const onReplaceQueryChange = vi.fn();
    render(() => (
      <FindReplaceBar
        {...baseProps}
        onReplaceQueryChange={onReplaceQueryChange}
      />
    ));
    fireEvent.input(screen.getByLabelText('Replace'), {
      target: { value: 'bar' },
    });
    expect(onReplaceQueryChange).toHaveBeenCalledWith('bar');
  });

  it('shows "0 of 0" when there are no matches', () => {
    render(() => <FindReplaceBar {...baseProps} matchCount={0} />);
    expect(screen.getByText('0 of 0')).toBeInTheDocument();
  });

  it('shows "{current+1} of {count}" with 1-indexed display', () => {
    render(() => (
      <FindReplaceBar
        {...baseProps}
        matchCount={12}
        currentMatch={2}
      />
    ));
    expect(screen.getByText('3 of 12')).toBeInTheDocument();
  });

  it('falls back to "1 of N" when currentMatch is undefined and matches exist', () => {
    render(() => <FindReplaceBar {...baseProps} matchCount={5} />);
    expect(screen.getByText('1 of 5')).toBeInTheDocument();
  });

  it('disables Prev/Next/Replace/ReplaceAll when no matches', () => {
    render(() => <FindReplaceBar {...baseProps} matchCount={0} />);
    expect(screen.getByLabelText('Previous match')).toBeDisabled();
    expect(screen.getByLabelText('Next match')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Replace' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Replace all' })).toBeDisabled();
  });

  it('fires onPrev/onNext/onReplace/onReplaceAll on click', () => {
    const onPrev = vi.fn();
    const onNext = vi.fn();
    const onReplace = vi.fn();
    const onReplaceAll = vi.fn();
    render(() => (
      <FindReplaceBar
        {...baseProps}
        matchCount={3}
        onPrev={onPrev}
        onNext={onNext}
        onReplace={onReplace}
        onReplaceAll={onReplaceAll}
      />
    ));
    fireEvent.click(screen.getByLabelText('Previous match'));
    fireEvent.click(screen.getByLabelText('Next match'));
    fireEvent.click(screen.getByRole('button', { name: 'Replace' }));
    fireEvent.click(screen.getByRole('button', { name: 'Replace all' }));
    expect(onPrev).toHaveBeenCalledTimes(1);
    expect(onNext).toHaveBeenCalledTimes(1);
    expect(onReplace).toHaveBeenCalledTimes(1);
    expect(onReplaceAll).toHaveBeenCalledTimes(1);
  });

  it('case-sensitive toggle reflects prop and fires onCaseSensitiveChange', () => {
    const onCaseSensitiveChange = vi.fn();
    render(() => (
      <FindReplaceBar
        {...baseProps}
        caseSensitive={true}
        onCaseSensitiveChange={onCaseSensitiveChange}
      />
    ));
    const toggle = screen.getByLabelText('Match case');
    expect(toggle).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(toggle);
    expect(onCaseSensitiveChange).toHaveBeenCalledWith(false);
  });

  it('whole-word toggle reflects prop and fires onWholeWordChange', () => {
    const onWholeWordChange = vi.fn();
    render(() => (
      <FindReplaceBar
        {...baseProps}
        wholeWord={false}
        onWholeWordChange={onWholeWordChange}
      />
    ));
    const toggle = screen.getByLabelText('Whole word');
    expect(toggle).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(toggle);
    expect(onWholeWordChange).toHaveBeenCalledWith(true);
  });

  it('Enter in the find input fires onNext (form submit)', () => {
    const onNext = vi.fn();
    render(() => <FindReplaceBar {...baseProps} matchCount={1} onNext={onNext} />);
    const form = screen.getByRole('search');
    fireEvent.submit(form);
    expect(onNext).toHaveBeenCalledTimes(1);
  });

  it('renders close button only when onClose is provided', () => {
    render(() => <FindReplaceBar {...baseProps} />);
    expect(screen.queryByLabelText('Close find and replace')).toBeNull();
    cleanup();
    const onClose = vi.fn();
    render(() => <FindReplaceBar {...baseProps} onClose={onClose} />);
    fireEvent.click(screen.getByLabelText('Close find and replace'));
    expect(onClose).toHaveBeenCalled();
  });

  it('merges class prop into the root form', () => {
    render(() => <FindReplaceBar {...baseProps} class='sticky top-0' />);
    const form = screen.getByRole('search');
    expect(form.className).toContain('sticky');
    expect(form.className).toContain('flex'); // default still present
  });
});
