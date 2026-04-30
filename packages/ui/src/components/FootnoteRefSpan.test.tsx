// Phase 5.1 RED — FootnoteRefSpan visual contract.

import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FootnoteRefSpan } from './FootnoteRefSpan';

afterEach(cleanup);

const baseProps = {
  id: 'fn1',
  displayNumber: 1,
};

describe('FootnoteRefSpan', () => {
  it('renders the display number as text content', () => {
    render(() => <FootnoteRefSpan {...baseProps} displayNumber={3} />);
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('uses a <sup> element so the number renders as superscript', () => {
    const { container } = render(() => <FootnoteRefSpan {...baseProps} />);
    expect(container.querySelector('sup')).not.toBeNull();
  });

  it('exposes the footnote id via data-footnote-id', () => {
    render(() => <FootnoteRefSpan {...baseProps} id='fn-42' />);
    expect(screen.getByRole('button')).toHaveAttribute('data-footnote-id', 'fn-42');
  });

  it('aria-label includes the display number for screen readers', () => {
    render(() => <FootnoteRefSpan {...baseProps} displayNumber={7} />);
    expect(screen.getByRole('button')).toHaveAttribute('aria-label', 'Footnote 7');
  });

  it('clicking fires onClick with the footnote id', () => {
    const onClick = vi.fn();
    render(() => <FootnoteRefSpan {...baseProps} id='fn-x' onClick={onClick} />);
    fireEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalledWith('fn-x');
  });
});
