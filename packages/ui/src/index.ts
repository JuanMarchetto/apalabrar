// Shared UI components for Apalabrar (toolbar, sidebars, modals, dialogs).
// Built on Solid + kobalte (headless WAI-ARIA) + Tailwind 4.

export const VERSION = '0.0.0' as const;

export { KobalteModal } from './components/KobalteModal';
export type { KobalteModalProps } from './components/KobalteModal';

export { WordClassicToggle } from './components/WordClassicToggle';
export type { WordClassicMode, WordClassicToggleProps } from './components/WordClassicToggle';

export { OutlinePane } from './components/OutlinePane';
export type {
  OutlineHeading,
  OutlineHeadingLevel,
  OutlinePaneProps,
} from './components/OutlinePane';

export { CommentsSidebar } from './components/CommentsSidebar';
export type {
  CommentReply,
  CommentsSidebarProps,
  CommentStatus,
  CommentThread,
} from './components/CommentsSidebar';

export { FindReplaceBar } from './components/FindReplaceBar';
export type { FindReplaceBarProps } from './components/FindReplaceBar';

export { FloatingSelectionToolbar } from './components/FloatingSelectionToolbar';
export type { FloatingSelectionToolbarProps } from './components/FloatingSelectionToolbar';

export { GDocsToolbar } from './components/GDocsToolbar';
export type {
  Alignment,
  GDocsToolbarProps,
  ListKind,
  ParagraphStyle,
} from './components/GDocsToolbar';
