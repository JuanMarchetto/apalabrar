import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { KobalteModal } from './KobalteModal';

afterEach(cleanup);

describe('KobalteModal', () => {
  it('does not render dialog content when closed', () => {
    render(() => <KobalteModal open={false} title='Hidden'>body</KobalteModal>);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText('Hidden')).toBeNull();
  });

  it('renders title, description, body, and footer when open', () => {
    render(() => (
      <KobalteModal
        open={true}
        title='Insert citation'
        description='Pick a source from your library.'
        footer={<button type='button'>Confirm</button>}
      >
        <p>Body copy</p>
      </KobalteModal>
    ));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Insert citation')).toBeInTheDocument();
    expect(screen.getByText('Pick a source from your library.'))
      .toBeInTheDocument();
    expect(screen.getByText('Body copy')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Confirm' }),
    ).toBeInTheDocument();
  });

  it('omits description when prop missing', () => {
    render(() => <KobalteModal open={true} title='Title only'>body</KobalteModal>);
    // No description element should be in the dialog.
    const dialog = screen.getByRole('dialog');
    expect(dialog.querySelector('[id$="description"]')).toBeNull();
  });

  it('exposes the title as the dialog accessible name', () => {
    render(() => <KobalteModal open={true} title='Settings'>body</KobalteModal>);
    expect(screen.getByRole('dialog', { name: 'Settings' }))
      .toBeInTheDocument();
  });

  it('renders a close button by default labelled "Close dialog"', () => {
    render(() => <KobalteModal open={true} title='With close'>body</KobalteModal>);
    expect(
      screen.getByRole('button', { name: 'Close dialog' }),
    ).toBeInTheDocument();
  });

  it('hides the close button when hideCloseButton is true', () => {
    render(() => (
      <KobalteModal open={true} title='Locked' hideCloseButton>
        body
      </KobalteModal>
    ));
    expect(screen.queryByRole('button', { name: 'Close dialog' })).toBeNull();
  });

  it('fires onOpenChange(false) when the close button is clicked', () => {
    const onOpenChange = vi.fn();
    render(() => (
      <KobalteModal open={true} title='X' onOpenChange={onOpenChange}>
        body
      </KobalteModal>
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Close dialog' }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('mounts when initial open=true and stays mounted when prop flips to true', () => {
    // Note: the inverse direction (true → false) intentionally not asserted.
    // Kobalte's Presence component keeps closed dialogs in the DOM until
    // CSS transitions end; happy-dom never fires `transitionend`. We rely
    // on the close-callback test + the "open=false initial" test to cover
    // the controlled-API contract.
    const [open, setOpen] = createSignal(false);
    render(() => <KobalteModal open={open()} title='Controlled'>body</KobalteModal>);
    expect(screen.queryByRole('dialog')).toBeNull();
    setOpen(true);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('merges contentClass into the dialog content element', () => {
    render(() => (
      <KobalteModal open={true} title='Wide' contentClass='max-w-3xl'>
        body
      </KobalteModal>
    ));
    const dialog = screen.getByRole('dialog');
    expect(dialog.className).toContain('max-w-3xl');
    // Default class still present.
    expect(dialog.className).toContain('rounded-lg');
  });

  it('treats undefined onOpenChange as a no-op (does not throw on close)', () => {
    render(() => <KobalteModal open={true} title='No handler'>body</KobalteModal>);
    expect(() => fireEvent.click(screen.getByRole('button', { name: 'Close dialog' }))).not
      .toThrow();
  });
});
