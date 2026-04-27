#![deny(unsafe_code)]
#![doc = "Apalabrar editor core — umbrella public API exposing the WASM-callable surface."]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

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

// -----------------------------------------------------------------------------
// Registry
// -----------------------------------------------------------------------------
//
// The registry maps DocId -> doc-model::Doc. It lives behind a static Mutex
// because the WASM bridge uses free functions (no `&self`); JS callers
// thread their handle through every call. AtomicU64 hands out monotonic ids.

struct Document {
    doc: apalabrar_doc_model::Doc,
}

static REGISTRY: OnceLock<Mutex<HashMap<u64, Document>>> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn registry() -> &'static Mutex<HashMap<u64, Document>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn allocate_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

/// Parse an OOXML byte stream into a fresh in-memory document and return an
/// opaque handle. Errors with `EmptyInput` for `&[]` and `InvalidOoxml` if
/// the bytes are not a valid `.docx` zip.
pub fn open_docx(bytes: &[u8]) -> Result<DocId, Error> {
    if bytes.is_empty() {
        return Err(Error::EmptyInput);
    }
    let text = apalabrar_format_docx::parse_text(bytes).map_err(|_| Error::InvalidOoxml)?;
    let mut doc = apalabrar_doc_model::Doc::new();
    if !text.is_empty() {
        doc.insert(0, &text);
    }
    let id_value = allocate_id();
    registry()
        .lock()
        .expect("registry mutex must not be poisoned")
        .insert(id_value, Document { doc });
    Ok(DocId(id_value))
}

/// Apply an edit to the plain-text projection of the doc.
pub fn apply_op(doc_id: DocId, op: EditOp) -> Result<(), Error> {
    let mut reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get_mut(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    let current = entry.doc.text();
    match op {
        EditOp::InsertText { offset, text } => {
            validate_insert_offset(&current, offset)?;
            let char_offset = byte_to_char_offset(&current, offset);
            entry.doc.insert(char_offset, &text);
        }
        EditOp::DeleteRange { start, end } => {
            validate_delete_range(&current, start, end)?;
            let char_start = byte_to_char_offset(&current, start);
            let char_end = byte_to_char_offset(&current, end);
            entry.doc.delete(char_start..char_end);
        }
    }
    Ok(())
}

/// Serialize the current state of the doc back to OOXML bytes.
pub fn to_docx(doc_id: DocId) -> Result<Vec<u8>, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    let text = entry.doc.text();
    apalabrar_format_docx::serialize_text(&text).map_err(|e| Error::SerializeFailed {
        reason: e.to_string(),
    })
}

/// Project the document state to a plain-text string.
pub fn doc_text(doc_id: DocId) -> Result<String, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    Ok(entry.doc.text())
}

/// Drop a document from the registry.
pub fn close_doc(doc_id: DocId) -> Result<(), Error> {
    let mut reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    if reg.remove(&doc_id.0).is_some() {
        Ok(())
    } else {
        Err(Error::UnknownDoc(doc_id))
    }
}

// -----------------------------------------------------------------------------
// Validation helpers
// -----------------------------------------------------------------------------

fn validate_insert_offset(text: &str, offset: usize) -> Result<(), Error> {
    let len = text.len();
    if offset > len {
        return Err(Error::OffsetOutOfBounds { offset, len });
    }
    if !text.is_char_boundary(offset) {
        return Err(Error::OffsetNotOnCharBoundary { offset });
    }
    Ok(())
}

fn validate_delete_range(text: &str, start: usize, end: usize) -> Result<(), Error> {
    let len = text.len();
    if start > end || end > len {
        return Err(Error::InvalidRange { start, end, len });
    }
    if !text.is_char_boundary(start) {
        return Err(Error::OffsetNotOnCharBoundary { offset: start });
    }
    if !text.is_char_boundary(end) {
        return Err(Error::OffsetNotOnCharBoundary { offset: end });
    }
    Ok(())
}

/// Translate a UTF-8 byte offset on a char boundary into a codepoint
/// (Unicode scalar) offset. Caller guarantees the byte offset is on a
/// char boundary, otherwise the slice indexing panics.
fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_pinned_to_workspace_zero_zero_zero() {
        assert_eq!(VERSION, "0.0.0");
    }
}
