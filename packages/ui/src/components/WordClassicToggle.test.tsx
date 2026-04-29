import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { WordClassicToggle } from './WordClassicToggle';

afterEach(cleanup);

describe('WordClassicToggle', () => {
  it('renders both Modern and Classic radios', () => {
    render(() => <WordClassicToggle value='modern' />);
    expect(screen.getByRole('radio', { name: 'Modern' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Classic' })).toBeInTheDocument();
  });

  it('exposes the group with an accessible name', () => {
    render(() => <WordClassicToggle value='modern' />);
    expect(
      screen.getByRole('radiogroup', { name: 'Toolbar layout' }),
    ).toBeInTheDocument();
  });

  it('marks the controlled value as checked', () => {
    render(() => <WordClassicToggle value='classic' />);
    const modern = screen.getByRole('radio', { name: 'Modern' });
    const classic = screen.getByRole('radio', { name: 'Classic' });
    expect(modern).not.toBeChecked();
    expect(classic).toBeChecked();
  });

  it('fires onChange("classic") when Classic radio is selected', () => {
    const onChange = vi.fn();
    render(() => <WordClassicToggle value='modern' onChange={onChange} />);
    fireEvent.click(screen.getByRole('radio', { name: 'Classic' }));
    expect(onChange).toHaveBeenCalledWith('classic');
  });

  it('fires onChange("modern") when Modern radio is selected', () => {
    const onChange = vi.fn();
    render(() => <WordClassicToggle value='classic' onChange={onChange} />);
    fireEvent.click(screen.getByRole('radio', { name: 'Modern' }));
    expect(onChange).toHaveBeenCalledWith('modern');
  });

  it('updates the checked radio when controlled value flips', () => {
    const [mode, setMode] = createSignal<'modern' | 'classic'>('modern');
    render(() => <WordClassicToggle value={mode()} />);
    expect(screen.getByRole('radio', { name: 'Modern' })).toBeChecked();
    setMode('classic');
    expect(screen.getByRole('radio', { name: 'Classic' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Modern' })).not.toBeChecked();
  });

  it('treats undefined onChange as a no-op (does not throw on click)', () => {
    render(() => <WordClassicToggle value='modern' />);
    expect(() => fireEvent.click(screen.getByRole('radio', { name: 'Classic' }))).not.toThrow();
  });

  it('merges the class prop into the root element', () => {
    render(() => <WordClassicToggle value='modern' class='ml-auto custom-toggle' />);
    const group = screen.getByRole('radiogroup');
    expect(group.className).toContain('custom-toggle');
    expect(group.className).toContain('ml-auto');
    // Default root class stays.
    expect(group.className).toContain('inline-flex');
  });
});
