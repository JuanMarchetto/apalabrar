#![deny(unsafe_code)]
#![doc = "Document model: Loro CRDT wrapper. Owns the canonical state of a document — text, formatting marks, snapshot/merge."]

use std::ops::Range;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Inline character formatting marks. Apalabrar v0 ships Bold + Italic;
/// later phases will extend this enum with Underline / Strike / Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    Bold,
    Italic,
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
    _internal: (),
}

impl Doc {
    /// Create a fresh empty document.
    pub fn new() -> Self {
        todo!("Step 4 GREEN: instantiate a fresh LoroDoc")
    }

    /// Insert `text` at codepoint offset `pos`. The implementation must
    /// clip `pos` into `[0, text_len_chars]` defensively rather than
    /// panic — randomised property tests rely on that contract.
    pub fn insert(&mut self, _pos: usize, _text: &str) {
        todo!("Step 4 GREEN: forward to LoroDoc text container insert")
    }

    /// Delete codepoint range `[range.start, range.end)`. Out-of-bounds
    /// indices must be clipped, not panicked.
    pub fn delete(&mut self, _range: Range<usize>) {
        todo!("Step 4 GREEN: forward to LoroDoc text container delete")
    }

    /// Apply `mark` to codepoints in `range`. Multiple marks compose:
    /// a position can carry both Bold and Italic simultaneously.
    pub fn format(&mut self, _range: Range<usize>, _mark: Mark) {
        todo!("Step 4 GREEN: forward to LoroDoc rich-text mark API")
    }

    /// Export the full state as a binary snapshot (for storage or sync).
    pub fn snapshot(&self) -> Vec<u8> {
        todo!("Step 4 GREEN: LoroDoc::export(ExportMode::Snapshot)")
    }

    /// Merge another replica's snapshot into self. CRDT semantics:
    /// commutative, associative, idempotent.
    pub fn merge(&mut self, _snapshot: &[u8]) {
        todo!("Step 4 GREEN: LoroDoc::import(snapshot)")
    }

    /// Construct a new doc from a snapshot. Equivalent in observable
    /// effect to `Doc::new()` followed by `merge(snapshot)`.
    pub fn from_snapshot(_snapshot: &[u8]) -> Self {
        todo!("Step 4 GREEN: new() + import(snapshot)")
    }

    /// Project the doc to a plain UTF-8 string.
    pub fn text(&self) -> String {
        todo!("Step 4 GREEN: LoroDoc text container .to_string()")
    }

    /// `true` iff the codepoint at `pos` carries `mark`. Returns `false`
    /// when `pos >= text_len_chars`.
    pub fn has_mark(&self, _pos: usize, _mark: Mark) -> bool {
        todo!("Step 4 GREEN: query LoroDoc rich-text marks at pos")
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
