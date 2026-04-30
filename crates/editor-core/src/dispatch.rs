//! Phase 2.4 — EditOp dispatcher producing `RenderDelta`.
//!
//! `dispatch` is the single entry point: takes a `&mut Doc` + `EditOp`,
//! applies the op, returns a `RenderDelta` describing what the layout
//! engine and UI shell need to repaint. Errors mirror the doc-model's
//! `apply_edit_op` failures, wrapped as `Error::EditOpFailed` so the
//! variant name and human-readable reason both reach JS.
//!
//! ## Why a dispatcher (in addition to `bridge::apply_edit_op`)?
//!
//! The bridge is the wasm/JSON entry point. The dispatcher is the
//! Rust-internal seam between "apply this op" and "tell the renderer
//! what changed." The bridge will eventually delegate to `dispatch`
//! and surface the `RenderDelta` to JS as a small JSON struct (or a
//! shared-buffer message); for Phase 2.4 the dispatcher stands on its
//! own and the bridge keeps its existing `Result<(), Error>` shape.
//! This keeps Phase 2.4 a strictly additive change.
//!
//! ## What's in a `RenderDelta`
//!
//! - `dirty_blocks` — half-open block index range to reflow.
//!   `start == end` means nothing dirty (no-op text insert, etc.).
//! - `structural` — `true` if the total block count changed. Layout
//!   uses this to invalidate pagination / scroll height.
//! - `caret_hint` — codepoint position where the caret should land
//!   after the op, on the post-op body. `None` for ops that don't
//!   move the caret (`FormatRange`, `InsertComment`).
//! - `minted_id` — annotation id created by this op
//!   (`InsertComment`, `Suggest`, `InsertCitation`, `InsertFootnote`).
//!   Surfaced so the UI can attach the new annotation without
//!   re-querying the doc.
//!
//! ## Clip-defensive semantics
//!
//! The dispatcher mirrors `Doc::insert/delete/format`'s clip-defensive
//! contract: out-of-bounds positions are clipped, never panicked. For
//! `DeleteRange` and `FormatRange`, the doc-model treats `from > to`
//! as a no-op (it does NOT swap the indices); the dispatcher matches
//! that behaviour by computing `end = to.min(len).max(start)` rather
//! than `min/max(from, to)`.

use apalabrar_doc_model::{Doc, EditOp, Position};

use crate::Error;

/// Half-open codepoint-block range `[start, end)`. `end == start`
/// means no dirty blocks (the op produced no observable change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRange {
    pub start: usize,
    pub end: usize,
}

impl BlockRange {
    /// Empty range starting at 0; sentinel for "nothing changed".
    pub const EMPTY: Self = Self { start: 0, end: 0 };

    /// `true` if the range is empty (`end == start`).
    pub fn is_empty(self) -> bool {
        self.end == self.start
    }
}

/// New annotation id minted by an op. Variant identifies which
/// container received the new record so the UI can route it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MintedId {
    Comment(String),
    Suggestion(String),
    Citation(String),
    Footnote(String),
    /// Phase 4.6 — id of a reply minted by `EditOp::ReplyToComment`.
    Reply(String),
}

/// What the dispatcher tells the renderer after applying an op.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDelta {
    pub dirty_blocks: BlockRange,
    pub structural: bool,
    pub caret_hint: Option<Position>,
    pub minted_id: Option<MintedId>,
}

impl RenderDelta {
    /// "Nothing happened" delta: empty range, no structural change,
    /// no caret hint, no minted id. Used for empty-text inserts and
    /// other clip-defensive no-ops.
    pub const NOOP: Self = Self {
        dirty_blocks: BlockRange::EMPTY,
        structural: false,
        caret_hint: None,
        minted_id: None,
    };
}

/// Apply `op` to `doc` and return a `RenderDelta` describing what
/// the renderer must repaint.
///
/// Errors mirror `Doc::apply_edit_op`:
/// - `Error::EditOpFailed { kind, reason }` for any doc-model failure
///   (carries the variant name + the doc-model error's `Display`).
///
/// Successful application is total: every valid `EditOp` produces
/// a `RenderDelta` (possibly the `NOOP` sentinel for empty inserts).
pub fn dispatch(doc: &mut Doc, op: EditOp) -> Result<RenderDelta, Error> {
    let kind = edit_op_kind(&op);
    match op {
        EditOp::InsertText { at, text, marks } => {
            if text.is_empty() {
                return Ok(RenderDelta::NOOP);
            }
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let actual_at = at.min(pre_len);
            let block_start = block_idx_at(&pre_text, actual_at);
            let newlines = text.chars().filter(|c| *c == '\n').count();
            let inserted_chars = text.chars().count();

            apply(doc, EditOp::InsertText { at, text, marks }, kind)?;

            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_start,
                    end: block_start + 1 + newlines,
                },
                structural: newlines > 0,
                caret_hint: Some(actual_at + inserted_chars),
                minted_id: None,
            })
        }
        EditOp::DeleteRange { from, to } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let pre_blocks = doc.block_count();
            let start = from.min(pre_len);
            let end = to.min(pre_len).max(start);
            let was_noop = start == end;
            let block_lo = block_idx_at(&pre_text, start);
            let block_hi = block_idx_at(&pre_text, end);

            apply(doc, EditOp::DeleteRange { from, to }, kind)?;

            if was_noop {
                return Ok(RenderDelta::NOOP);
            }
            let post_blocks = doc.block_count();
            let structural = post_blocks != pre_blocks;
            let dirty_end = if structural {
                block_lo + 1
            } else {
                block_hi + 1
            };
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: dirty_end,
                },
                structural,
                caret_hint: Some(start),
                minted_id: None,
            })
        }
        EditOp::FormatRange { from, to, mark } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let start = from.min(pre_len);
            let end = to.min(pre_len).max(start);
            let was_noop = start == end;
            let block_lo = block_idx_at(&pre_text, start);
            let block_hi = block_idx_at(&pre_text, end);

            apply(doc, EditOp::FormatRange { from, to, mark }, kind)?;

            if was_noop {
                return Ok(RenderDelta::NOOP);
            }
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: block_hi + 1,
                },
                structural: false,
                caret_hint: None,
                minted_id: None,
            })
        }
        EditOp::InsertBlock { at, block } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let actual_at = at.min(pre_len);
            let block_idx = block_idx_at(&pre_text, actual_at);
            let block_text_len = block.text.chars().count();

            apply(doc, EditOp::InsertBlock { at, block }, kind)?;

            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_idx,
                    end: block_idx + 2,
                },
                structural: true,
                caret_hint: Some(actual_at + block_text_len + 1),
                minted_id: None,
            })
        }
        EditOp::SplitBlock { at } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let actual_at = at.min(pre_len);
            let block_idx = block_idx_at(&pre_text, actual_at);

            apply(doc, EditOp::SplitBlock { at }, kind)?;

            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_idx,
                    end: block_idx + 2,
                },
                structural: true,
                caret_hint: Some(actual_at + 1),
                minted_id: None,
            })
        }
        EditOp::MergeBlocks { first, second } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let pre_blocks = doc.block_count();
            let f = first.min(pre_len);
            let block_first = block_idx_at(&pre_text, f);

            apply(doc, EditOp::MergeBlocks { first, second }, kind)?;

            let post_blocks = doc.block_count();
            if post_blocks == pre_blocks {
                // Non-adjacent (or otherwise out-of-contract) merge:
                // the doc-model returns Ok and does not mutate.
                return Ok(RenderDelta::NOOP);
            }
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_first,
                    end: block_first + 1,
                },
                structural: true,
                caret_hint: None,
                minted_id: None,
            })
        }
        EditOp::InsertComment {
            from,
            to,
            body,
            thread_id,
            author,
            created_at,
        } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let start = from.min(pre_len);
            let end = to.min(pre_len).max(start);
            let block_lo = block_idx_at(&pre_text, start);
            let block_hi = block_idx_at(&pre_text, end);

            apply(
                doc,
                EditOp::InsertComment {
                    from,
                    to,
                    body,
                    thread_id,
                    author,
                    created_at,
                },
                kind,
            )?;

            let id = doc
                .last_comment_thread_id()
                .expect("doc-model contract: InsertComment Ok ⇒ last_comment_thread_id Some");
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: block_hi + 1,
                },
                structural: false,
                caret_hint: None,
                minted_id: Some(MintedId::Comment(id)),
            })
        }
        EditOp::ReplyToComment {
            thread_id,
            body,
            author,
            created_at,
        } => {
            // Phase 4.6 — RenderDelta for a reply: no text mutation, no
            // structural change, no caret movement. The dirty_blocks
            // range matches the parent thread's anchor so the renderer
            // can re-paint any badge / count overlay on the thread.
            let parent_range = doc
                .comment(&thread_id)
                .map(|c| (c.from, c.to))
                .unwrap_or((0, 0));
            apply(
                doc,
                EditOp::ReplyToComment {
                    thread_id,
                    body,
                    author,
                    created_at,
                },
                kind,
            )?;
            let pre_text = doc.text();
            let block_lo = block_idx_at(&pre_text, parent_range.0);
            let block_hi = block_idx_at(&pre_text, parent_range.1);
            let id = doc
                .last_reply_id()
                .expect("doc-model contract: ReplyToComment Ok ⇒ last_reply_id Some");
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: block_hi + 1,
                },
                structural: false,
                caret_hint: None,
                minted_id: Some(MintedId::Reply(id)),
            })
        }
        EditOp::SetCommentStatus { thread_id, status } => {
            // Phase 4.6 — same dirty-block range as the parent thread's
            // anchor so the badge re-paints.
            let parent_range = doc
                .comment(&thread_id)
                .map(|c| (c.from, c.to))
                .unwrap_or((0, 0));
            apply(doc, EditOp::SetCommentStatus { thread_id, status }, kind)?;
            let pre_text = doc.text();
            let block_lo = block_idx_at(&pre_text, parent_range.0);
            let block_hi = block_idx_at(&pre_text, parent_range.1);
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: block_hi + 1,
                },
                structural: false,
                caret_hint: None,
                minted_id: None,
            })
        }
        EditOp::Suggest {
            from,
            to,
            replacement,
        } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let start = from.min(pre_len);
            let end = to.min(pre_len).max(start);
            let block_lo = block_idx_at(&pre_text, start);
            let block_hi = block_idx_at(&pre_text, end);

            apply(
                doc,
                EditOp::Suggest {
                    from,
                    to,
                    replacement,
                },
                kind,
            )?;

            let id = doc
                .last_suggestion_id()
                .expect("doc-model contract: Suggest Ok ⇒ last_suggestion_id Some");
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: block_hi + 1,
                },
                structural: false,
                caret_hint: None,
                minted_id: Some(MintedId::Suggestion(id)),
            })
        }
        EditOp::AcceptSuggestion { suggestion_id } => {
            let pre_text = doc.text();
            let pre_blocks = doc.block_count();
            let (range_lo, range_hi) = match doc.suggestion(&suggestion_id) {
                Some(s) => {
                    let lo = s.from.min(s.to);
                    let hi = s.from.max(s.to);
                    (lo, hi)
                }
                None => (0, 0),
            };
            let block_lo = block_idx_at(&pre_text, range_lo);
            let block_hi = block_idx_at(&pre_text, range_hi);

            apply(doc, EditOp::AcceptSuggestion { suggestion_id }, kind)?;

            let post_blocks = doc.block_count();
            let structural = post_blocks != pre_blocks;
            let dirty_end = if structural {
                block_lo + 1
            } else {
                block_hi + 1
            };
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_lo,
                    end: dirty_end,
                },
                structural,
                caret_hint: None,
                minted_id: None,
            })
        }
        EditOp::InsertCitation { at, key } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let actual_at = at.min(pre_len);
            let block_idx = block_idx_at(&pre_text, actual_at);

            apply(doc, EditOp::InsertCitation { at, key }, kind)?;

            let id = doc
                .last_citation_id()
                .expect("doc-model contract: InsertCitation Ok ⇒ last_citation_id Some");
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_idx,
                    end: block_idx + 1,
                },
                structural: false,
                caret_hint: Some(actual_at + 1),
                minted_id: Some(MintedId::Citation(id)),
            })
        }
        EditOp::InsertFootnote { at, body } => {
            let pre_text = doc.text();
            let pre_len = pre_text.chars().count();
            let actual_at = at.min(pre_len);
            let block_idx = block_idx_at(&pre_text, actual_at);

            apply(doc, EditOp::InsertFootnote { at, body }, kind)?;

            let id = doc
                .last_footnote_id()
                .expect("doc-model contract: InsertFootnote Ok ⇒ last_footnote_id Some");
            Ok(RenderDelta {
                dirty_blocks: BlockRange {
                    start: block_idx,
                    end: block_idx + 1,
                },
                structural: false,
                caret_hint: Some(actual_at + 1),
                minted_id: Some(MintedId::Footnote(id)),
            })
        }
    }
}

/// Forward to `Doc::apply_edit_op`, wrapping any error as
/// `Error::EditOpFailed { kind, reason }`. Centralised so every
/// match arm produces identically-shaped errors.
fn apply(doc: &mut Doc, op: EditOp, kind: &'static str) -> Result<(), Error> {
    doc.apply_edit_op(op).map_err(|e| Error::EditOpFailed {
        kind: kind.to_string(),
        reason: e.to_string(),
    })
}

/// Block index containing codepoint position `p` in `text`.
/// Convention matches `Doc::block_idx_at`: positions ON a `\n`
/// codepoint belong to the PRECEDING block.
fn block_idx_at(text: &str, p: Position) -> usize {
    let mut block = 0;
    for (i, ch) in text.chars().enumerate() {
        if i == p {
            return block;
        }
        if ch == '\n' {
            block += 1;
        }
    }
    block
}

/// Variant-name extractor used by `dispatch` to label
/// `EditOpFailed` errors.
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
