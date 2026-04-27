#![deny(unsafe_code)]
#![doc = "Apalabrar editor core — umbrella public API exposing the WASM-callable surface."]

use thiserror::Error;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Opaque handle to an opened document held in the in-WASM document registry.
///
/// `DocId` values are produced by `open_docx` and invalidated by `close_doc`.
/// The internal representation is intentionally hidden; callers should treat
/// it as a magic cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocId(u64);

/// Editing operation against the plain-text projection of a document.
///
/// Offsets are UTF-8 byte offsets into the plain-text projection.
/// They MUST fall on character boundaries; otherwise `apply_op` returns
/// `Error::OffsetNotOnCharBoundary`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    InsertText { offset: usize, text: String },
    DeleteRange { start: usize, end: usize },
}

/// Failure modes for the editor-core public API.
#[derive(Debug, Error)]
pub enum Error {
    #[error("input bytes are empty")]
    EmptyInput,
    #[error("input bytes are not a valid OOXML/.docx file")]
    InvalidOoxml,
    #[error("doc id {0:?} is unknown to the registry")]
    UnknownDoc(DocId),
    #[error("offset {offset} is past document length {len}")]
    OffsetOutOfBounds { offset: usize, len: usize },
    #[error("offset {offset} does not fall on a UTF-8 character boundary")]
    OffsetNotOnCharBoundary { offset: usize },
    #[error("range {start}..{end} is invalid (start > end or end > len {len})")]
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    #[error("OOXML serialization failed: {reason}")]
    SerializeFailed { reason: String },
}

/// Parse an OOXML byte stream into a fresh in-memory document and return an
/// opaque handle. Errors with `EmptyInput` for `&[]` and `InvalidOoxml` if
/// the bytes are not a valid `.docx` zip.
pub fn open_docx(_bytes: &[u8]) -> Result<DocId, Error> {
    todo!("Step 4 GREEN: parse OOXML via apalabrar-format-docx and seed Loro doc")
}

/// Apply an edit to the plain-text projection of the doc.
/// Returns `UnknownDoc` if the handle has been closed, `OffsetOutOfBounds`
/// for offsets past the doc length, `OffsetNotOnCharBoundary` for offsets
/// inside a multi-byte UTF-8 codepoint, and `InvalidRange` for malformed
/// delete ranges.
pub fn apply_op(_doc: DocId, _op: EditOp) -> Result<(), Error> {
    todo!("Step 4 GREEN: route InsertText/DeleteRange into the Loro-backed doc-model")
}

/// Serialize the current state of the doc back to OOXML bytes. The output
/// must round-trip: `open_docx(to_docx(d))` reproduces the same plain text.
pub fn to_docx(_doc: DocId) -> Result<Vec<u8>, Error> {
    todo!("Step 4 GREEN: serialize the doc back to OOXML via apalabrar-format-docx")
}

/// Project the document state to a plain-text string. Used for tests and
/// for the demo page render path. Returns `UnknownDoc` on a closed handle.
pub fn doc_text(_doc: DocId) -> Result<String, Error> {
    todo!("Step 4 GREEN: project the doc-model state to plain text")
}

/// Drop a document from the registry. Subsequent calls with the same `DocId`
/// return `UnknownDoc`. Closing an already-closed handle also returns
/// `UnknownDoc`.
pub fn close_doc(_doc: DocId) -> Result<(), Error> {
    todo!("Step 4 GREEN: remove the entry from the registry")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_pinned_to_workspace_zero_zero_zero() {
        assert_eq!(VERSION, "0.0.0");
    }
}
