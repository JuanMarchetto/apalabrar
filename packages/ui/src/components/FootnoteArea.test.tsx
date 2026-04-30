// Phase 5.1 RED — FootnoteArea visual contract.

import { cleanup, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it } from 'vitest';
import { FootnoteArea, type FootnoteAreaItem } from './FootnoteArea';

afterEach(cleanup);

const items: FootnoteAreaItem[] = [
  { id: 'fn1', displayNumber: 1, bodyText: 'first footnote body' },
  { id: 'fn2', displayNumber: 2, bodyText: 'second footnote body' },
];

describe('FootnoteArea', () => {
  it('renders a region landmark labelled "Footnotes"', () => {
    render(() => <FootnoteArea items={items} />);
    expect(screen.getByRole('region', { name: 'Footnotes' })).toBeInTheDocument();
  });

  it('renders one entry per item', () => {
    const { container } = render(() => <FootnoteArea items={items} />);
    expect(container.querySelectorAll('[data-footnote-id]')).toHaveLength(2);
  });

  it('prefixes each entry with its display number', () => {
    render(() => <FootnoteArea items={items} />);
    expect(screen.getByText('1.')).toBeInTheDocument();
    expect(screen.getByText('2.')).toBeInTheDocument();
  });

  it('shows the body text of each footnote', () => {
    render(() => <FootnoteArea items={items} />);
    expect(screen.getByText('first footnote body')).toBeInTheDocument();
    expect(screen.getByText('second footnote body')).toBeInTheDocument();
  });

  it('renders nothing in the list when items is empty', () => {
    const { container } = render(() => <FootnoteArea items={[]} />);
    expect(container.querySelectorAll('[data-footnote-id]')).toHaveLength(0);
    // The section landmark itself still renders (so there's a stable
    // mount point to swap items in/out without re-creating the DOM).
    expect(screen.getByRole('region', { name: 'Footnotes' })).toBeInTheDocument();
  });
});
