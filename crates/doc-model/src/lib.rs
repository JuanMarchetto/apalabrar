#![deny(unsafe_code)]
#![doc = "Document model: Loro CRDT wrapper. Owns the canonical state of a document — text, formatting marks, snapshot/merge."]

use std::ops::Range;

use loro::{
    ExportMode, LoroDoc, LoroMovableList, LoroText, LoroValue, TextDelta, ValueOrContainer,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable position handle. A flat Unicode codepoint offset into the
/// linearised body text (see "Block model: single-linearised" below).
/// Kept as a `usize` alias rather than an opaque struct: Phase A's
/// `EditOp::InsertBlock { at: Position, .. }` signature is already
/// public, so promoting `Position` to a struct would break that
/// surface. Block-aware cursors (if needed for v1 collab UX) will
/// land as a *separate* opaque handle type alongside `Position`,
/// not as a replacement.
pub type Position = usize;

/// Stable container ID for the document body. A LoroDoc holds many
/// containers keyed by string id; we reserve "doc" for the rich-text body
/// so snapshots are interchangeable across instances.
const TEXT_CONTAINER_ID: &str = "doc";

/// Stable container ID for the parallel block-kind list (Phase B).
/// One entry per block; `len(blocks) == count('\n' in body) + 1` is
/// the invariant maintained by every block-level op.
const BLOCKS_CONTAINER_ID: &str = "blocks";

// ─────────────────────────────────────────────────────────────────
// Block model: single-linearised (locked 2026-04-28, Phase B).
// ─────────────────────────────────────────────────────────────────
//
// The doc body is a SINGLE `LoroText`. Block boundaries are `\n`
// codepoints inside that text. Block kinds (paragraph / heading /
// list-item) live in a parallel `LoroMovableList<String>` named
// "blocks" with one entry per block — i.e. always one more entry
// than the number of `\n` chars in the body.
//
// Why single-linearised over multi-container (one `LoroText` per
// block under a `LoroMovableList`):
//
//   1. `EditOp::InsertBlock { at: Position, .. }` and friends were
//      locked in Phase A with `Position = usize` (a flat codepoint
//      offset). Multi-container would force `Position` to become an
//      opaque `(BlockId, offset)` struct, which breaks the public
//      surface and the 31 already-green Phase A edit-op tests.
//      Tests are immutable except when wrong, and Phase A's tests
//      are not wrong.
//
//   2. v0 is single-user. The CRDT-merge advantage of multi-container
//      (block-level causality on concurrent edits across the same
//      paragraph boundary) is theoretical at v0; we have no syncing
//      peers yet. When v1 collab arrives we revisit — likely as a
//      stable-cursor LAYER over the linear text, not a re-architect.
//
//   3. Single-linearised has a smaller mutation-test surface and
//      keeps the existing `insert / delete / format` paths unchanged.
//      Cold/incremental layout already projects from `body.text()`
//      so the layout engine needs no changes either.
//
// Trade-off accepted: concurrent edits at a block boundary on
// peers A and B will land deterministically per Loro's text CRDT,
// but the *block identity* of the inserted text depends on which
// side of the `\n` it lands on. For collab-heavy v1 features
// (per-block comments, scroll-to-block) we will introduce a
// derived "block id" type backed by Loro text cursors at the
// boundary positions; that addition is non-breaking.
//
// Block-kind encoding in the `LoroMovableList`:
//   - `BlockKind::Paragraph`             → "paragraph"
//   - `BlockKind::Heading { level }`     → "heading:{level}" (level ∈ 1..=6)
//   - `BlockKind::ListItem { indent }`   → "list-item:{indent}"
// Out-of-range levels clamp on insert. Unknown strings decode to
// `Paragraph` defensively (forward-compat for future kinds).

/// Inline character formatting marks. Apalabrar v0 ships Bold + Italic;
/// later phases will extend this enum with Underline / Strike / Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    Bold,
    Italic,
}

impl Mark {
    /// The Loro mark key. Keys must be stable across replicas — a Bold
    /// mark from peer A and a Bold mark from peer B are merged on the
    /// same key.
    fn key(self) -> &'static str {
        match self {
            Mark::Bold => "bold",
            Mark::Italic => "italic",
        }
    }
}

/// Apalabrar's CRDT-backed document model.
///
/// Positions are Unicode codepoint offsets — i.e. compatible with Loro's
/// native text-container indexing and with `text.chars().nth(pos)` semantics
/// at the Rust level. UTF-8 byte offsets are intentionally NOT used here;
/// that translation lives one layer up in `editor-core` where the JS bridge
/// hands in string offsets.
///
/// Methods that take a `Range<usize>` interpret it as the half-open range
/// `[start, end)` over codepoints.
pub struct Doc {
    inner: LoroDoc,
}

impl Doc {
    /// Create a fresh empty document.
    pub fn new() -> Self {
        Self {
            inner: LoroDoc::new(),
        }
    }

    /// Get the LoroText handle for the document body. Cheap — re-fetched
    /// per-call rather than cached because LoroDoc is the source of truth
    /// after import/merge and the handle's view is always live.
    fn body(&self) -> LoroText {
        self.inner.get_text(TEXT_CONTAINER_ID)
    }

    /// Insert `text` at codepoint offset `pos`. Clips `pos` into
    /// `[0, text_len_chars]` defensively — out-of-bounds inserts do not
    /// panic. Empty `text` is a no-op.
    pub fn insert(&mut self, pos: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let body = self.body();
        let len = body.len_unicode();
        let pos = pos.min(len);
        body.insert(pos, text)
            .expect("loro text insert after clip should not fail");
    }

    /// Delete codepoint range `[range.start, range.end)`. Out-of-bounds
    /// indices are clipped, not panicked. Empty range is a no-op.
    pub fn delete(&mut self, range: Range<usize>) {
        let body = self.body();
        let len = body.len_unicode();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        if start == end {
            return;
        }
        body.delete(start, end - start)
            .expect("loro text delete after clip should not fail");
    }

    /// Apply `mark` to codepoints in `range`. Multiple marks compose: a
    /// position can carry both Bold and Italic simultaneously.
    pub fn format(&mut self, range: Range<usize>, mark: Mark) {
        let body = self.body();
        let len = body.len_unicode();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        if start == end {
            return;
        }
        body.mark(start..end, mark.key(), true)
            .expect("loro text mark after clip should not fail");
    }

    /// Export the full state as a binary snapshot (for storage or sync).
    /// `commit` first so any pending in-memory ops are included in the
    /// exported state.
    pub fn snapshot(&self) -> Vec<u8> {
        self.inner.commit();
        self.inner
            .export(ExportMode::Snapshot)
            .expect("loro export snapshot should not fail")
    }

    /// Merge another replica's snapshot into self. CRDT semantics:
    /// commutative, associative, idempotent.
    pub fn merge(&mut self, snapshot: &[u8]) {
        self.inner
            .import(snapshot)
            .expect("loro import should accept a previously exported snapshot");
    }

    /// Construct a new doc from a snapshot.
    pub fn from_snapshot(snapshot: &[u8]) -> Self {
        let inner = LoroDoc::from_snapshot(snapshot)
            .expect("loro from_snapshot should accept a previously exported snapshot");
        Self { inner }
    }

    /// Project the doc to a plain UTF-8 string.
    pub fn text(&self) -> String {
        self.body().to_string()
    }

    /// `true` iff the codepoint at `pos` carries `mark`. Returns `false`
    /// when `pos >= text_len_chars` or when no segment of the rich-text
    /// delta covers the position.
    pub fn has_mark(&self, pos: usize, mark: Mark) -> bool {
        let key = mark.key();
        let mut cursor: usize = 0;
        for segment in self.body().to_delta() {
            let TextDelta::Insert { insert, attributes } = segment else {
                continue;
            };
            let chars = insert.chars().count();
            if pos < cursor + chars {
                return attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get(key))
                    .map(|v| matches!(v, LoroValue::Bool(true)))
                    .unwrap_or(false);
            }
            cursor += chars;
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────
// Block model (Phase B): single-linearised body + parallel kind list
// ─────────────────────────────────────────────────────────────────

impl Doc {
    /// Get the parallel block-kinds list. The list is allowed to be
    /// shorter than `block_count()` (Phase A snapshots have no list,
    /// or `Doc::insert/delete` may have changed `\n` count without
    /// touching the list) — accessors fall back to `Paragraph` when
    /// an entry is missing. The list MAY also be longer than
    /// `block_count()` after concurrent merges or Phase A deletes
    /// that crossed boundaries; trailing entries are simply ignored.
    fn blocks_list(&self) -> LoroMovableList {
        self.inner.get_movable_list(BLOCKS_CONTAINER_ID)
    }

    /// Bring the blocks list up to `block_count()` by appending
    /// "paragraph" defaults. Idempotent. Called before any block-
    /// level mutation so the subsequent `.insert(idx, ..)` calls
    /// land at valid positions.
    fn pad_blocks_list(&self) {
        let needed = self.block_count();
        let blocks = self.blocks_list();
        let current = blocks.len();
        for _ in current..needed {
            blocks
                .push("paragraph")
                .expect("blocks list push should not fail");
        }
    }

    /// Block index containing codepoint position `p`. Convention:
    /// positions ON a `\n` codepoint belong to the PRECEDING block
    /// (the `\n` "ends" its preceding block).
    fn block_idx_at(&self, p: Position) -> usize {
        let body_text = self.body().to_string();
        let mut block = 0;
        for (i, ch) in body_text.chars().enumerate() {
            if i == p {
                return block;
            }
            if ch == '\n' {
                block += 1;
            }
        }
        block
    }

    /// `(start, end)` codepoint range of block `idx`, excluding
    /// surrounding `\n`s. Caller must guarantee `idx < block_count()`.
    fn block_range(&self, idx: usize) -> (Position, Position) {
        let body_text = self.body().to_string();
        let segments: Vec<&str> = body_text.split('\n').collect();
        let start: usize = segments[..idx].iter().map(|s| s.chars().count() + 1).sum();
        let end = start + segments[idx].chars().count();
        (start, end)
    }

    /// Read the kind at `idx` from the blocks list, defaulting to
    /// `Paragraph` if the list is shorter or the entry isn't a
    /// recognised string.
    fn block_kind_at(&self, idx: usize) -> BlockKind {
        match self.blocks_list().get(idx) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => str_to_kind(s.as_ref()),
            _ => BlockKind::Paragraph,
        }
    }

    /// Number of blocks in the document. An empty doc has exactly one
    /// block (the implicit empty paragraph). Each `\n` codepoint in
    /// the body adds one more block.
    pub fn block_count(&self) -> usize {
        self.body()
            .to_string()
            .chars()
            .filter(|c| *c == '\n')
            .count()
            + 1
    }

    /// Block at `idx`, or `None` if `idx >= block_count()`. The
    /// returned `Block::text` is the literal codepoint slice between
    /// the two surrounding `\n`s (or doc edges).
    pub fn block(&self, idx: usize) -> Option<Block> {
        let body_text = self.body().to_string();
        let segments: Vec<&str> = body_text.split('\n').collect();
        if idx >= segments.len() {
            return None;
        }
        Some(Block {
            kind: self.block_kind_at(idx),
            text: segments[idx].to_string(),
        })
    }
}

/// Encode a `BlockKind` as the canonical `LoroMovableList` string.
/// Heading levels clamp into 1..=6; ListItem indent is preserved as-is.
fn kind_to_str(kind: &BlockKind) -> String {
    match kind {
        BlockKind::Paragraph => "paragraph".into(),
        BlockKind::Heading { level } => format!("heading:{}", (*level).clamp(1, 6)),
        BlockKind::ListItem { indent } => format!("list-item:{indent}"),
    }
}

/// Decode a kind from its canonical string. Unknown / malformed
/// strings decode to `Paragraph` so forward-compat (a v1 client
/// reading a v2 doc with kinds we don't yet recognise) renders as
/// readable text rather than failing.
fn str_to_kind(s: &str) -> BlockKind {
    if let Some(rest) = s.strip_prefix("heading:") {
        let level = rest.parse::<u8>().unwrap_or(1).clamp(1, 6);
        BlockKind::Heading { level }
    } else if let Some(rest) = s.strip_prefix("list-item:") {
        let indent = rest.parse::<u8>().unwrap_or(0);
        BlockKind::ListItem { indent }
    } else {
        BlockKind::Paragraph
    }
}

impl Default for Doc {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────
// Edit-op surface (Phase 2 prompt 2.2 — Phase A)
// ─────────────────────────────────────────────────────────────────
//
// `EditOp` is the cross-boundary edit verb. The blueprint in
// `blueprint-part3-synthesis.md` Section G enumerates 11 variants.
// Phase A implements the three text-level variants (`InsertText`,
// `DeleteRange`, `FormatRange`) that map onto the existing
// `insert/delete/format` Loro path. The other eight (block-,
// comment-, suggestion-, citation-, footnote-level) are declared
// here so the cross-boundary type is FINAL — only their
// `apply_edit_op` arms return `Error::NotYetImplemented(name)` until
// Phase B/C/D fill them in. This keeps the bridge contract stable
// from the start; consumer code already feature-flags variants by
// catching `NotYetImplemented`.
//
// Inverse semantics (used by the round-trip property test):
//
//   - `InsertText { at, text, marks }` ↔ `DeleteRange { from: at,
//     to: at + text.chars().count() }`. Inverse loses the marks
//     vector — that's fine because Loro's mark deletion isn't a
//     standalone op in the blueprint; round-trip is over the text
//     projection, not the rich-text projection.
//   - `DeleteRange { from, to }` has an inverse only if the deleted
//     bytes were captured first. The property test exposes the
//     `InsertText`→`DeleteRange` direction; the reverse direction
//     is left to Phase B once we have a stable cursor that survives
//     a delete.
//   - `FormatRange` would invert via a `RemoveFormat` op the
//     blueprint doesn't yet declare. Phase 3+ when the blueprint
//     extends.
//
// The EditOp variants and their argument types are deliberately
// rich-and-flat enums (no &str references) so the boundary can
// MessagePack them directly without lifetime gymnastics.

/// Block-level node kind. Mirrors `apalabrar-layout::BlockKind` but
/// serialised at the CRDT level (the layout engine projects from a
/// snapshot of this).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    /// Body paragraph.
    Paragraph,
    /// Heading; `level` ∈ 1..=6 (clamped on insert by Phase B).
    Heading { level: u8 },
    /// Bulleted list item; `indent` is logical depth (0 = top).
    ListItem { indent: u8 },
}

/// Block-level node. Phase A uses this only as the value type of
/// `EditOp::InsertBlock`; Phase B introduces the corresponding Loro
/// container in the doc model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
}

/// A flat list of blocks. Used by `EditOp::InsertFootnote`. The
/// general case is recursive (footnotes containing footnotes), but
/// Phase A keeps it flat — Phase D will switch to a recursive
/// representation when the BlockTree gets its own Loro model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockTree {
    pub blocks: Vec<Block>,
}

/// Cross-boundary edit verb. Variants 1-3 (`InsertText`,
/// `DeleteRange`, `FormatRange`) are implemented; the remaining
/// eight return `Error::NotYetImplemented`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    /// Insert `text` at `at`, optionally applying `marks` to the
    /// inserted span. An empty `text` is a no-op (no marks fire).
    InsertText {
        at: Position,
        text: String,
        marks: Vec<Mark>,
    },
    /// Delete codepoints in `[from, to)`. Out-of-bounds indices
    /// clip; `from > to` is treated as a no-op (matches the
    /// pre-existing `Doc::delete` clip-defensive semantics).
    DeleteRange { from: Position, to: Position },
    /// Apply `mark` to codepoints in `[from, to)`. Same clip
    /// semantics as `Doc::format`.
    FormatRange {
        from: Position,
        to: Position,
        mark: Mark,
    },
    /// Insert a block-level node at `at` (Phase B).
    InsertBlock { at: Position, block: Block },
    /// Split the block containing `at` into two (Phase B).
    SplitBlock { at: Position },
    /// Merge two adjacent blocks (Phase B).
    MergeBlocks { first: Position, second: Position },
    /// Anchor a comment thread to `[from, to)` (Phase C).
    InsertComment {
        from: Position,
        to: Position,
        body: String,
        thread_id: Option<String>,
    },
    /// Propose a replacement for `[from, to)` without applying it
    /// (Phase C — track-changes).
    Suggest {
        from: Position,
        to: Position,
        replacement: String,
    },
    /// Apply a previously-recorded suggestion by id (Phase C).
    AcceptSuggestion { suggestion_id: String },
    /// Anchor a CSL citation key at `at` (Phase D).
    InsertCitation { at: Position, key: String },
    /// Anchor a footnote (its body lives in a sub-doc) at `at`
    /// (Phase D).
    InsertFootnote { at: Position, body: BlockTree },
}

/// Failure modes for `apply_edit_op`. Marked `#[non_exhaustive]` so
/// Phase B/C/D can add new variants (eg. `InvalidRange`,
/// `SuggestionNotFound`, `BlockOutOfBounds`) without a SemVer
/// breaking change for callers.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The op variant is declared at the bridge boundary but the
    /// CRDT model hasn't grown the necessary container yet. The
    /// payload carries the variant name (eg. `"InsertBlock"`) so
    /// callers can branch on which variants are available without
    /// exhaustively matching on `EditOp`.
    #[error("EditOp variant '{0}' is not yet implemented in this phase")]
    NotYetImplemented(&'static str),
}

impl Doc {
    /// Apply an `EditOp` to the document.
    ///
    /// Variants 1-3 (`InsertText`, `DeleteRange`, `FormatRange`)
    /// route to the existing `insert/delete/format` paths and
    /// always succeed (clip-defensive on bounds). Variants 4-11
    /// return `Err(Error::NotYetImplemented(name))` until the
    /// corresponding phase is implemented.
    pub fn apply_edit_op(&mut self, op: EditOp) -> Result<(), Error> {
        match op {
            EditOp::InsertText { at, text, marks } => {
                if text.is_empty() {
                    return Ok(());
                }
                let prior_len = self.body().len_unicode();
                let actual_start = at.min(prior_len);
                let chars_inserted = text.chars().count();
                self.insert(at, &text);
                for mark in marks {
                    self.format(actual_start..actual_start + chars_inserted, mark);
                }
                Ok(())
            }
            EditOp::DeleteRange { from, to } => {
                self.delete(from..to);
                Ok(())
            }
            EditOp::FormatRange { from, to, mark } => {
                self.format(from..to, mark);
                Ok(())
            }
            EditOp::InsertBlock { at, block } => self.handle_insert_block(at, block),
            EditOp::SplitBlock { at } => self.handle_split_block(at),
            EditOp::MergeBlocks { first, second } => self.handle_merge_blocks(first, second),
            EditOp::InsertComment { .. } => Err(Error::NotYetImplemented("InsertComment")),
            EditOp::Suggest { .. } => Err(Error::NotYetImplemented("Suggest")),
            EditOp::AcceptSuggestion { .. } => Err(Error::NotYetImplemented("AcceptSuggestion")),
            EditOp::InsertCitation { .. } => Err(Error::NotYetImplemented("InsertCitation")),
            EditOp::InsertFootnote { .. } => Err(Error::NotYetImplemented("InsertFootnote")),
        }
    }

    /// Insert a new block at codepoint position `at`. The block
    /// containing `at` is split (if `at` is mid-block) so the new
    /// block sits between the two halves. The "before" half retains
    /// the original kind; the new block carries `block.kind`; the
    /// "after" half also retains the original kind. At a block
    /// boundary, the new block is prepended/appended without
    /// generating an empty side block.
    fn handle_insert_block(&mut self, at: Position, block: Block) -> Result<(), Error> {
        let body = self.body();
        let len = body.len_unicode();
        let at = at.min(len);

        let block_idx = self.block_idx_at(at);
        let (block_start, block_end) = self.block_range(block_idx);
        let kind_old_str = kind_to_str(&self.block_kind_at(block_idx));
        let new_kind_str = kind_to_str(&block.kind);

        // Pad before any insert so subsequent .insert(idx, ..) calls
        // land at valid positions (Loro lists allow insert up to len).
        self.pad_blocks_list();
        let blocks = self.blocks_list();

        let offset_in_block = at - block_start;
        let block_len = block_end - block_start;

        if offset_in_block == 0 {
            body.insert(at, &format!("{}\n", block.text))
                .expect("loro insert should not fail");
            blocks
                .insert(block_idx, new_kind_str)
                .expect("loro list insert should not fail");
        } else if offset_in_block == block_len {
            body.insert(at, &format!("\n{}", block.text))
                .expect("loro insert should not fail");
            blocks
                .insert(block_idx + 1, new_kind_str)
                .expect("loro list insert should not fail");
        } else {
            body.insert(at, &format!("\n{}\n", block.text))
                .expect("loro insert should not fail");
            // The "before" half stays at block_idx with kind_old.
            // The new block goes at block_idx+1.
            // The "after" half is at block_idx+2 — needs kind_old duplicated.
            blocks
                .insert(block_idx + 1, new_kind_str)
                .expect("loro list insert should not fail");
            blocks
                .insert(block_idx + 2, kind_old_str)
                .expect("loro list insert should not fail");
        }
        Ok(())
    }

    /// Split the block containing `at` into two at codepoint position
    /// `at`. Both halves keep the original block's kind. Splitting at
    /// a block boundary creates one empty block on the side opposite
    /// the split direction.
    fn handle_split_block(&mut self, at: Position) -> Result<(), Error> {
        let body = self.body();
        let len = body.len_unicode();
        let at = at.min(len);
        let block_idx = self.block_idx_at(at);
        let kind_old_str = kind_to_str(&self.block_kind_at(block_idx));
        self.pad_blocks_list();
        let blocks = self.blocks_list();
        body.insert(at, "\n").expect("loro insert should not fail");
        // Equivalent-mutant note: cargo-mutants reports `block_idx + 1`
        // → `block_idx` as a survivor here. It is mathematically
        // equivalent: `kind_old_str` was just read from `blocks[block_idx]`,
        // so inserting it at `block_idx` (mutated) or `block_idx + 1`
        // (original) yields lists with identical contents at every
        // index — the value already at `block_idx` is the same value
        // we are inserting. Killing this would require a SplitBlock
        // semantics change (eg. a sentinel "after-half" kind), which
        // is out of scope for v0.
        blocks
            .insert(block_idx + 1, kind_old_str)
            .expect("loro list insert should not fail");
        Ok(())
    }

    /// Merge the blocks containing `first` and `second` if they are
    /// directly adjacent (i.e. block_idx(second) == block_idx(first)+1).
    /// Otherwise, no-op. The resulting block carries the kind of the
    /// FIRST block (left wins).
    fn handle_merge_blocks(&mut self, first: Position, second: Position) -> Result<(), Error> {
        let body = self.body();
        let len = body.len_unicode();
        let f = first.min(len);
        let s = second.min(len);
        let i_first = self.block_idx_at(f);
        let i_second = self.block_idx_at(s);
        if i_first + 1 != i_second {
            return Ok(());
        }
        // Find the codepoint position of the `\n` that separates
        // block i_first from block i_first+1.
        let body_text = body.to_string();
        let mut block = 0;
        let mut newline_pos: Option<usize> = None;
        for (i, ch) in body_text.chars().enumerate() {
            if ch == '\n' {
                if block == i_first {
                    newline_pos = Some(i);
                    break;
                }
                block += 1;
            }
        }
        // Adjacency check guarantees at least one `\n` between the
        // two blocks, so `newline_pos` is always Some here. The let-
        // else makes that explicit and removes a defensive arm.
        let Some(nl) = newline_pos else {
            return Ok(());
        };
        // Pad BEFORE the body delete so the list has at least
        // i_second + 1 entries. That removes the need for a
        // defensive `if blocks.len() > i_second` guard around the
        // list delete (which would otherwise harbour a surviving
        // mutant).
        self.pad_blocks_list();
        let blocks = self.blocks_list();
        body.delete(nl, 1).expect("loro delete should not fail");
        blocks
            .delete(i_second, 1)
            .expect("loro list delete should not fail");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
