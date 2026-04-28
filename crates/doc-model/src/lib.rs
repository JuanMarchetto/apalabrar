#![deny(unsafe_code)]
#![doc = "Document model: Loro CRDT wrapper. Owns the canonical state of a document — text, formatting marks, snapshot/merge."]

use std::ops::Range;

use loro::{ExportMode, LoroDoc, LoroText, LoroValue, TextDelta};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable position handle. Phase A is text-only and represents
/// positions as Unicode codepoint offsets so the surface stays
/// compatible with the existing `insert/delete/format` API. Phase B
/// (block model) will replace this alias with an opaque struct that
/// survives concurrent edits across blocks; that change is
/// non-breaking for callers that already speak codepoint offsets
/// because the new type will have a `From<usize>` constructor.
pub type Position = usize;

/// Stable container ID for the document body. A LoroDoc holds many
/// containers keyed by string id; we reserve "doc" for the rich-text body
/// so snapshots are interchangeable across instances.
const TEXT_CONTAINER_ID: &str = "doc";

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
            EditOp::InsertBlock { .. } => Err(Error::NotYetImplemented("InsertBlock")),
            EditOp::SplitBlock { .. } => Err(Error::NotYetImplemented("SplitBlock")),
            EditOp::MergeBlocks { .. } => Err(Error::NotYetImplemented("MergeBlocks")),
            EditOp::InsertComment { .. } => Err(Error::NotYetImplemented("InsertComment")),
            EditOp::Suggest { .. } => Err(Error::NotYetImplemented("Suggest")),
            EditOp::AcceptSuggestion { .. } => Err(Error::NotYetImplemented("AcceptSuggestion")),
            EditOp::InsertCitation { .. } => Err(Error::NotYetImplemented("InsertCitation")),
            EditOp::InsertFootnote { .. } => Err(Error::NotYetImplemented("InsertFootnote")),
        }
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
