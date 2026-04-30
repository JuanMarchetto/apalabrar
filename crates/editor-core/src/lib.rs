#![deny(unsafe_code)]
#![doc = "Apalabrar editor core — umbrella public API exposing the WASM-callable surface."]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use thiserror::Error;

pub mod bridge;
pub mod dispatch;
pub mod find;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Opaque handle to an opened document held in the in-WASM document registry.
///
/// `DocId` values are produced by `open_docx` and invalidated by `close_doc`.
/// The internal representation is intentionally hidden; callers should treat
/// it as a magic cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocId(u64);

#[cfg(target_arch = "wasm32")]
impl DocId {
    /// Internal-only ctor used by the wasm bridge to reconstruct a handle
    /// from a JS-side cookie.
    pub(crate) fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Internal-only accessor exposing the cookie value to the wasm bridge.
    pub(crate) fn raw(self) -> u64 {
        self.0
    }
}

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
    /// `bridge::apply_edit_op_json` couldn't parse the incoming JSON
    /// into a doc-model `EditOp`. Phase 2.3 introduces this variant.
    #[error("EditOp JSON parse failed: {reason}")]
    JsonParseFailed { reason: String },
    /// The doc-model dispatcher returned an error (eg. AcceptSuggestion
    /// with an unknown id). Carries the variant name + a human-readable
    /// reason so JS callers can branch without re-deriving doc-model
    /// internals.
    #[error("EditOp dispatch failed for {kind}: {reason}")]
    EditOpFailed { kind: String, reason: String },
}

// -----------------------------------------------------------------------------
// Registry
// -----------------------------------------------------------------------------
//
// The registry maps DocId -> doc-model::Doc. It lives behind a static Mutex
// because the WASM bridge uses free functions (no `&self`); JS callers
// thread their handle through every call. AtomicU64 hands out monotonic ids.

pub(crate) struct Document {
    pub(crate) doc: apalabrar_doc_model::Doc,
}

static REGISTRY: OnceLock<Mutex<HashMap<u64, Document>>> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn registry() -> &'static Mutex<HashMap<u64, Document>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn allocate_id() -> u64 {
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

/// Find all non-overlapping occurrences of `needle` in the doc's
/// plain-text projection. Returns codepoint ranges (matching
/// [`apalabrar_doc_model::Position`]).
///
/// Phase 4.5: this wraps [`find::find`] against the registry. The
/// jumprope substring-search optimisation lands in step 5 (polish)
/// once the engine is GREEN.
pub fn find(
    doc_id: DocId,
    needle: &str,
    opts: find::FindOptions,
) -> Result<Vec<find::Match>, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    let text = entry.doc.text();
    Ok(find::find(&text, needle, opts))
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

// -----------------------------------------------------------------------------
// WASM bridge
// -----------------------------------------------------------------------------
//
// Thin pass-through wrappers exposed via wasm-bindgen so the JS side of the
// editor (the Solid shell) can drive the same API used by the Rust tests.
// Gated on cfg(target_arch = "wasm32") so the host-side `cargo test` run
// never sees the JS-only types. The wrappers are pure routing — every
// behaviour they expose is already covered by the Rust integration suite,
// so wasm-bindgen-test setup is intentionally deferred.
//
// IMPORTANT for Validation Gate 2: at least one wasm-bindgen export must
// reach each of {open_docx, apply_op, to_docx, doc_text, close_doc} so
// the WASM linker keeps the loro / docx-rs / doc-model code paths alive.
// Otherwise dead-code elimination would strip them and the bundle-size
// measurement would not reflect realistic editor cost.

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    use crate::{DocId, EditOp, Error, apply_op, close_doc, doc_text, open_docx, to_docx};

    fn err(e: Error) -> JsValue {
        JsValue::from_str(&e.to_string())
    }

    #[wasm_bindgen(js_name = openDocx)]
    pub fn js_open_docx(bytes: &[u8]) -> Result<u64, JsValue> {
        open_docx(bytes).map(DocId::raw).map_err(err)
    }

    #[wasm_bindgen(js_name = applyInsert)]
    pub fn js_apply_insert(doc_id: u64, offset: usize, text: &str) -> Result<(), JsValue> {
        apply_op(
            DocId::from_raw(doc_id),
            EditOp::InsertText {
                offset,
                text: text.into(),
            },
        )
        .map_err(err)
    }

    #[wasm_bindgen(js_name = applyDelete)]
    pub fn js_apply_delete(doc_id: u64, start: usize, end: usize) -> Result<(), JsValue> {
        apply_op(DocId::from_raw(doc_id), EditOp::DeleteRange { start, end }).map_err(err)
    }

    #[wasm_bindgen(js_name = toDocx)]
    pub fn js_to_docx(doc_id: u64) -> Result<Vec<u8>, JsValue> {
        to_docx(DocId::from_raw(doc_id)).map_err(err)
    }

    #[wasm_bindgen(js_name = docText)]
    pub fn js_doc_text(doc_id: u64) -> Result<String, JsValue> {
        doc_text(DocId::from_raw(doc_id)).map_err(err)
    }

    #[wasm_bindgen(js_name = closeDoc)]
    pub fn js_close_doc(doc_id: u64) -> Result<(), JsValue> {
        close_doc(DocId::from_raw(doc_id)).map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_pinned_to_workspace_zero_zero_zero() {
        assert_eq!(VERSION, "0.0.0");
    }
}
