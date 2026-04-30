//! Phase 2.3 — JS↔Rust bridge for the doc-model `EditOp` surface.
//!
//! This module exposes the full doc-model edit-op API (Phase 2.2's 11
//! variants) through editor-core's registry, with a JSON dispatcher
//! suitable for crossing the wasm-bindgen boundary. Designed so the JS
//! side of the editor can call `applyEditOp(docId, JSON.stringify(op))`
//! and receive a typed `Result` back.
//!
//! ## Wire format
//!
//! - `EditOp` and its value types (`Mark`, `Block`, `BlockKind`,
//!   `BlockTree`, etc.) derive `Serialize + Deserialize` with
//!   `#[serde(tag = "kind")]` so JSON shape matches the blueprint's
//!   Section G TypeScript declaration:
//!
//!   ```json
//!   {"kind":"InsertText","at":5,"text":"hello","marks":["Bold"]}
//!   ```
//!
//! - Block kinds use `#[serde(tag = "type")]` to disambiguate the
//!   nested `Block.kind` field from the EditOp's outer `kind` tag:
//!
//!   ```json
//!   {"kind":"InsertBlock","at":0,"block":{"kind":{"type":"Heading","level":1},"text":"hi"}}
//!   ```
//!
//! ## Why JSON over MessagePack
//!
//! The blueprint suggests MessagePack for wire compactness; v0 ships
//! JSON because (a) the serde_json dep is already in the workspace,
//! (b) JSON is debuggable in browser devtools without extra tooling,
//! and (c) per-rAF batching means the bridge isn't on the keystroke
//! hot path. Migration to MessagePack is non-breaking — same serde
//! shape, different encoder.

use apalabrar_doc_model::{Doc, EditOp};
use serde::{Deserialize, Serialize};

use crate::find;
use crate::{DocId, Document, Error, allocate_id, registry};

/// Create a fresh empty document and return an opaque handle.
/// Distinct from `open_docx` which seeds the body from an OOXML blob.
pub fn create_doc() -> DocId {
    let id_value = allocate_id();
    registry()
        .lock()
        .expect("registry mutex must not be poisoned")
        .insert(id_value, Document { doc: Doc::new() });
    DocId(id_value)
}

/// Apply a doc-model `EditOp` to the document. Native-Rust path —
/// callers in Rust use this directly; the JS side comes through
/// `apply_edit_op_json`.
pub fn apply_edit_op(doc_id: DocId, op: EditOp) -> Result<(), Error> {
    let mut reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get_mut(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    let kind = edit_op_kind(&op);
    entry
        .doc
        .apply_edit_op(op)
        .map_err(|e| Error::EditOpFailed {
            kind: kind.to_string(),
            reason: e.to_string(),
        })
}

/// Apply a JSON-encoded `EditOp`. Rejects malformed JSON with
/// `JsonParseFailed`; dispatches valid ops through `apply_edit_op`.
pub fn apply_edit_op_json(doc_id: DocId, op_json: &str) -> Result<(), Error> {
    let op: EditOp = serde_json::from_str(op_json).map_err(|e| Error::JsonParseFailed {
        reason: e.to_string(),
    })?;
    apply_edit_op(doc_id, op)
}

/// Export the doc as a Loro CRDT snapshot blob (suitable for the
/// storage layer). `from_snapshot` round-trips this blob back into a
/// fresh `DocId`.
pub fn snapshot(doc_id: DocId) -> Result<Vec<u8>, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    Ok(entry.doc.snapshot())
}

/// Restore a doc from a snapshot blob. Errors with `SerializeFailed`
/// on garbage bytes — `Doc::from_snapshot` panics inside Loro on
/// malformed input, so we catch that with a defensive landing pad.
pub fn restore_from_snapshot(snapshot: &[u8]) -> Result<DocId, Error> {
    let doc = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Doc::from_snapshot(snapshot)
    }))
    .map_err(|_| Error::SerializeFailed {
        reason: "snapshot bytes are not a valid Loro CRDT snapshot".into(),
    })?;
    let id_value = allocate_id();
    registry()
        .lock()
        .expect("registry mutex must not be poisoned")
        .insert(id_value, Document { doc });
    Ok(DocId(id_value))
}

/// Number of blocks in the doc.
pub fn block_count(doc_id: DocId) -> Result<usize, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    Ok(entry.doc.block_count())
}

/// Block at `idx` serialised as JSON, or `None` if out of range.
/// Returning JSON keeps the wasm-bindgen surface simple — JS gets a
/// string and `JSON.parse`s it into the typed `Block`.
pub fn block_at_json(doc_id: DocId, idx: usize) -> Result<Option<String>, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    match entry.doc.block(idx) {
        Some(block) => Ok(Some(serialize_or_panic(&block))),
        None => Ok(None),
    }
}

// ─────────────────────────────────────────────────────────────────
// Phase 4.5 — Find (JSON wrapper for the wasm bridge)
// ─────────────────────────────────────────────────────────────────

/// JSON shape for [`find::FindOptions`] on the wire.
/// Matches the TypeScript `FindOptions` interface in
/// `packages/editor-bridge/src/core.ts`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FindOptionsJson {
    #[serde(rename = "caseSensitive")]
    pub case_sensitive: bool,
    #[serde(rename = "wholeWord")]
    pub whole_word: bool,
}

impl From<FindOptionsJson> for find::FindOptions {
    fn from(j: FindOptionsJson) -> Self {
        Self {
            case_sensitive: j.case_sensitive,
            whole_word: j.whole_word,
        }
    }
}

/// JSON shape for one [`find::Match`] on the wire. `start` and `end`
/// are CODEPOINT indices, half-open `[start, end)`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchJson {
    pub start: usize,
    pub end: usize,
}

impl From<find::Match> for MatchJson {
    fn from(m: find::Match) -> Self {
        Self {
            start: m.start,
            end: m.end,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Phase 4.6 — Comments accessor (JSON wrapper for the wasm bridge)
// ─────────────────────────────────────────────────────────────────

/// All comment threads in the doc, JSON-encoded as a `Comment[]`.
/// The shape matches the TypeScript `Comment` interface in
/// `packages/editor-bridge/src/core.ts`. Threads are returned sorted
/// by `thread_id` (matches `Doc::comments()`).
pub fn comments_json(doc_id: DocId) -> Result<String, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    let threads = entry.doc.comments();
    Ok(serialize_or_panic(&threads))
}

/// Find all non-overlapping matches; return them as a JSON array.
/// Errors propagate as [`Error::JsonParseFailed`] (malformed
/// `opts_json`) or [`Error::UnknownDoc`] (registry miss).
pub fn find_json(doc_id: DocId, needle: &str, opts_json: &str) -> Result<String, Error> {
    let opts: FindOptionsJson =
        serde_json::from_str(opts_json).map_err(|e| Error::JsonParseFailed {
            reason: e.to_string(),
        })?;
    let matches = crate::find(doc_id, needle, opts.into())?;
    let json: Vec<MatchJson> = matches.into_iter().map(MatchJson::from).collect();
    Ok(serde_json::to_string(&json).expect("Vec<MatchJson> serialises without error"))
}

/// Test-only escape hatch: borrow the underlying `Doc` to read
/// transient state (eg. `last_suggestion_id`) that isn't yet exposed
/// through dedicated bridge accessors. Public so integration tests
/// in `tests/bridge.rs` can read it; not part of the v0 stable
/// public API.
pub fn with_doc<T>(doc_id: DocId, f: impl FnOnce(&Doc) -> T) -> Result<T, Error> {
    let reg = registry()
        .lock()
        .expect("registry mutex must not be poisoned");
    let entry = reg.get(&doc_id.0).ok_or(Error::UnknownDoc(doc_id))?;
    Ok(f(&entry.doc))
}

/// Serialize any value that `serde::Serialize`s; panics on failure
/// because all our value types are simple enough that serialization
/// cannot legitimately fail. Wrapping the panic message keeps the
/// wasm bundle from pulling in serde's full error formatting tree.
fn serialize_or_panic<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("apalabrar value types must serialize without error")
}

/// Variant-name extractor used by `apply_edit_op` to label
/// `EditOpFailed` errors. Kept in lock-step with the doc-model
/// `EditOp` enum; a missing arm is a compile error so this stays in
/// sync with the wire format.
fn edit_op_kind(op: &EditOp) -> &'static str {
    match op {
        EditOp::InsertText { .. } => "InsertText",
        EditOp::DeleteRange { .. } => "DeleteRange",
        EditOp::FormatRange { .. } => "FormatRange",
        EditOp::InsertBlock { .. } => "InsertBlock",
        EditOp::SplitBlock { .. } => "SplitBlock",
        EditOp::MergeBlocks { .. } => "MergeBlocks",
        EditOp::InsertComment { .. } => "InsertComment",
        EditOp::ReplyToComment { .. } => "ReplyToComment",
        EditOp::SetCommentStatus { .. } => "SetCommentStatus",
        EditOp::Suggest { .. } => "Suggest",
        EditOp::AcceptSuggestion { .. } => "AcceptSuggestion",
        EditOp::InsertCitation { .. } => "InsertCitation",
        EditOp::InsertFootnote { .. } => "InsertFootnote",
    }
}

// ─────────────────────────────────────────────────────────────────
// WASM bridge
// ─────────────────────────────────────────────────────────────────
//
// Thin pass-through wrappers for the JS side. Every behaviour they
// expose is already covered by the host-side Rust tests in
// `tests/bridge.rs`; the wasm exports are smoke-tested in
// `tests/wasm.rs` once a browser runtime is wired in CI.

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    use crate::bridge::{
        apply_edit_op_json, block_at_json, block_count, comments_json, create_doc, find_json,
        restore_from_snapshot, snapshot,
    };
    use crate::{DocId, Error, close_doc, doc_text};

    fn err(e: Error) -> JsValue {
        JsValue::from_str(&e.to_string())
    }

    #[wasm_bindgen(js_name = createDoc)]
    pub fn js_create_doc() -> u64 {
        create_doc().0
    }

    #[wasm_bindgen(js_name = applyEditOp)]
    pub fn js_apply_edit_op(doc_id: u64, op_json: &str) -> Result<(), JsValue> {
        apply_edit_op_json(DocId(doc_id), op_json).map_err(err)
    }

    #[wasm_bindgen(js_name = bridgeSnapshot)]
    pub fn js_snapshot(doc_id: u64) -> Result<Vec<u8>, JsValue> {
        snapshot(DocId(doc_id)).map_err(err)
    }

    #[wasm_bindgen(js_name = restoreFromSnapshot)]
    pub fn js_restore_from_snapshot(snap: &[u8]) -> Result<u64, JsValue> {
        restore_from_snapshot(snap).map(|d| d.0).map_err(err)
    }

    #[wasm_bindgen(js_name = blockCount)]
    pub fn js_block_count(doc_id: u64) -> Result<usize, JsValue> {
        block_count(DocId(doc_id)).map_err(err)
    }

    #[wasm_bindgen(js_name = blockAt)]
    pub fn js_block_at(doc_id: u64, idx: usize) -> Result<Option<String>, JsValue> {
        block_at_json(DocId(doc_id), idx).map_err(err)
    }

    #[wasm_bindgen(js_name = bridgeDocText)]
    pub fn js_doc_text(doc_id: u64) -> Result<String, JsValue> {
        doc_text(DocId(doc_id)).map_err(err)
    }

    #[wasm_bindgen(js_name = bridgeCloseDoc)]
    pub fn js_close_doc(doc_id: u64) -> Result<(), JsValue> {
        close_doc(DocId(doc_id)).map_err(err)
    }

    #[wasm_bindgen(js_name = findInDoc)]
    pub fn js_find_in_doc(doc_id: u64, needle: &str, opts_json: &str) -> Result<String, JsValue> {
        find_json(DocId(doc_id), needle, opts_json).map_err(err)
    }

    #[wasm_bindgen(js_name = commentsInDoc)]
    pub fn js_comments_in_doc(doc_id: u64) -> Result<String, JsValue> {
        comments_json(DocId(doc_id)).map_err(err)
    }
}
