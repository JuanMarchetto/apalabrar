// Phase 4.5 RED — FindController integration.
//
// Two tests:
// 1. Typing fires `props.find` with the current query + options and
//    updates the bar's match counter.
// 2. Clicking Next cycles `currentMatch` through the matches and wraps
//    around at the last index.

import type { FindOptions, Match } from '@apalabrar/editor-bridge';
import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FindController } from './FindController';

afterEach(cleanup);

describe('FindController', () => {
  it('typing in the find input fires props.find and updates match counter', () => {
    const findFn = vi.fn<(n: string, o: FindOptions) => Match[]>(() => [
      { start: 0, end: 5 },
      { start: 12, end: 17 },
    ]);
    render(() => <FindController find={findFn} />);

    fireEvent.input(screen.getByLabelText('Find'), {
      target: { value: 'hello' },
    });

    expect(findFn).toHaveBeenCalled();
    const lastCall = findFn.mock.calls.at(-1);
    expect(lastCall?.[0]).toBe('hello');
    // After find returns 2 matches, counter should show "1 of 2".
    expect(screen.getByText('1 of 2')).toBeInTheDocument();
  });

  it('clicking Next cycles currentMatch through matches with wraparound', () => {
    const matches: Match[] = [
      { start: 0, end: 1 },
      { start: 5, end: 6 },
      { start: 10, end: 11 },
    ];
    render(() => <FindController find={() => matches} initialQuery='x' />);

    // initialQuery triggers a find on mount → counter shows "1 of 3".
    expect(screen.getByText('1 of 3')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Next match'));
    expect(screen.getByText('2 of 3')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Next match'));
    expect(screen.getByText('3 of 3')).toBeInTheDocument();

    // Wraparound back to first.
    fireEvent.click(screen.getByLabelText('Next match'));
    expect(screen.getByText('1 of 3')).toBeInTheDocument();
  });
});
