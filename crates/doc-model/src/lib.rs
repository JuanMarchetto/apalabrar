#![deny(unsafe_code)]
#![doc = "Document model: Loro CRDT wrapper. Owns the canonical state of a document — text, formatting marks, snapshot/merge."]

use std::ops::Range;

use loro::{ExportMode, LoroDoc, LoroText, LoroValue, TextDelta};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
