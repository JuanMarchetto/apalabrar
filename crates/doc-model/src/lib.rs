#![deny(unsafe_code)]
#![doc = "Document model: Loro CRDT wrapper. Owns the canonical state of a document — text, formatting marks, snapshot/merge."]

use std::ops::Range;

use loro::{
    Container, ExportMode, LoroDoc, LoroList, LoroListValue, LoroMap, LoroMapValue,
    LoroMovableList, LoroText, LoroValue, TextDelta, ValueOrContainer,
};
use serde::{Deserialize, Serialize};

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

/// Stable container IDs for Phase C/D. Comments / suggestions /
/// citations / footnotes are root `LoroMap`s keyed by id; each
/// value is a sub-`LoroMap` storing the record's fields. `meta`
/// holds monotonically-increasing counters used by the ID
/// generator so generated IDs survive snapshot round-trips.
const COMMENTS_CONTAINER_ID: &str = "comments";
const SUGGESTIONS_CONTAINER_ID: &str = "suggestions";
const CITATIONS_CONTAINER_ID: &str = "citations";
const FOOTNOTES_CONTAINER_ID: &str = "footnotes";
const META_CONTAINER_ID: &str = "meta";

/// Meta keys used for ID generation counters.
const META_KEY_NEXT_COMMENT: &str = "next_comment_id";
const META_KEY_NEXT_SUGGESTION: &str = "next_suggestion_id";
const META_KEY_NEXT_CITATION: &str = "next_citation_id";
const META_KEY_NEXT_FOOTNOTE: &str = "next_footnote_id";
/// Phase 4.6 — counter for reply ids inside comment threads.
const META_KEY_NEXT_REPLY: &str = "next_reply_id";

/// Sub-map field keys (kept private but defined as constants so
/// `mutants` can't silently rename one without breaking tests).
const FIELD_FROM: &str = "from";
const FIELD_TO: &str = "to";
const FIELD_BODY: &str = "body";
const FIELD_REPLACEMENT: &str = "replacement";
const FIELD_STATE: &str = "state";
const FIELD_AT: &str = "at";
const FIELD_KEY: &str = "key";
const FIELD_BLOCKS: &str = "blocks";
const FIELD_KIND: &str = "kind";
const FIELD_TEXT: &str = "text";
/// Phase 4.6 — comment metadata fields stored alongside the existing
/// `from / to / body` keys in each thread's sub-LoroMap.
const FIELD_AUTHOR: &str = "author";
const FIELD_CREATED_AT: &str = "created_at";
/// Comment thread status (`open` | `resolved`). Distinct key from
/// `FIELD_STATE` (which carries suggestion lifecycle) so a future
/// container that mixed both could not collide on key.
const FIELD_STATUS: &str = "status";
/// LoroList of reply sub-maps inside the thread sub-LoroMap.
const FIELD_REPLIES: &str = "replies";
/// Reply id stored alongside the reply's body/author/created_at —
/// stable across reorders even though replies are a LoroList.
const FIELD_REPLY_ID: &str = "id";

/// Suggestion-state string codes (single source of truth).
const STATE_PENDING: &str = "pending";
const STATE_ACCEPTED: &str = "accepted";
const STATE_REJECTED: &str = "rejected";

/// Comment-status string codes (Phase 4.6). Single source of truth.
const STATUS_OPEN: &str = "open";
const STATUS_RESOLVED: &str = "resolved";

/// Phase D marker codepoints. Both live in the Unicode private-use
/// area so they cannot collide with anything a user might type or
/// paste from external sources. Using DISTINCT codepoints (one per
/// marker kind) means the layout engine and OOXML serialiser can
/// dispatch on the codepoint alone, without consulting the parallel
/// citation / footnote maps.
const CITATION_MARKER: char = '\u{E000}';
const FOOTNOTE_MARKER: char = '\u{E001}';

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Transient: the thread_id assigned by the most recent
    /// `apply_edit_op(InsertComment)` on this Doc instance. Reset on
    /// `new` / `from_snapshot`. Not persisted in the CRDT — the
    /// caller is expected to read this immediately after applying
    /// the op.
    last_comment_thread_id: Option<String>,
    /// Transient: the id assigned by the most recent
    /// `apply_edit_op(Suggest)` on this Doc instance. Same lifetime
    /// rules as `last_comment_thread_id`.
    last_suggestion_id: Option<String>,
    /// Transient: id assigned by the most recent
    /// `apply_edit_op(InsertCitation)` (Phase D).
    last_citation_id: Option<String>,
    /// Transient: id assigned by the most recent
    /// `apply_edit_op(InsertFootnote)` (Phase D).
    last_footnote_id: Option<String>,
    /// Transient: reply id assigned by the most recent
    /// `apply_edit_op(ReplyToComment)` (Phase 4.6).
    last_reply_id: Option<String>,
}

impl Doc {
    /// Create a fresh empty document.
    pub fn new() -> Self {
        Self {
            inner: LoroDoc::new(),
            last_comment_thread_id: None,
            last_suggestion_id: None,
            last_citation_id: None,
            last_footnote_id: None,
            last_reply_id: None,
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
        Self {
            inner,
            last_comment_thread_id: None,
            last_suggestion_id: None,
            last_citation_id: None,
            last_footnote_id: None,
            last_reply_id: None,
        }
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

    /// Project every block at once. O(text length + block count) —
    /// equivalent to `(0..block_count()).map(|i| block(i).unwrap())`
    /// but does the body-clone and split exactly once instead of per
    /// call. Use this when you need the full block list (eg. layout,
    /// export to OOXML); use `block(idx)` only when you need exactly
    /// one block by index.
    pub fn blocks(&self) -> Vec<Block> {
        let body_text = self.body().to_string();
        let segments: Vec<&str> = body_text.split('\n').collect();
        let kinds_list = self.blocks_list();
        segments
            .iter()
            .enumerate()
            .map(|(idx, seg)| Block {
                kind: match kinds_list.get(idx) {
                    Some(ValueOrContainer::Value(LoroValue::String(s))) => str_to_kind(s.as_ref()),
                    _ => BlockKind::Paragraph,
                },
                text: (*seg).to_owned(),
            })
            .collect()
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

// ─────────────────────────────────────────────────────────────────
// Comment + suggestion model (Phase C)
// ─────────────────────────────────────────────────────────────────
//
// Storage shape:
//   `comments`    : LoroMap, key = thread_id, value = sub-LoroMap
//   `suggestions` : LoroMap, key = suggestion_id, value = sub-LoroMap
//   `meta`        : LoroMap, holds I64 counters used by the ID gen
//
// Comment sub-map fields: `from`, `to` (I64 codepoint positions),
// `body` (String).
// Suggestion sub-map fields: `from`, `to`, `replacement`, `state`
// (one of "pending" | "accepted" | "rejected").
//
// Anchor caveat: positions are RAW codepoints. They go stale if the
// surrounding text is edited. Phase C v0 ships this; Phase C+ will
// migrate to Loro-cursor-based stable anchors. The reason raw
// positions ship first is the public Loro 1.12 surface: `Cursor` and
// `Side` are NOT re-exported with `pub use`, so persisting a cursor
// would require depending on the `loro_internal` crate. Adding that
// dep was judged out of scope for this phase.

impl Doc {
    fn comments_map(&self) -> LoroMap {
        self.inner.get_map(COMMENTS_CONTAINER_ID)
    }

    fn suggestions_map(&self) -> LoroMap {
        self.inner.get_map(SUGGESTIONS_CONTAINER_ID)
    }

    fn meta_map(&self) -> LoroMap {
        self.inner.get_map(META_CONTAINER_ID)
    }

    /// Read an i64 counter from the meta map, defaulting to 0.
    fn meta_counter(&self, key: &str) -> i64 {
        match self.meta_map().get(key) {
            Some(ValueOrContainer::Value(LoroValue::I64(n))) => n,
            _ => 0,
        }
    }

    /// Generate a new id using the persistent counter under
    /// `counter_key`. Format: `{prefix}-{peer_id_hex}-{counter}` so
    /// IDs from different peers / different counters never collide.
    fn next_id(&self, prefix: &str, counter_key: &str) -> String {
        let n = self.meta_counter(counter_key);
        self.meta_map()
            .insert(counter_key, LoroValue::I64(n + 1))
            .expect("loro meta map insert should not fail");
        let peer = self.inner.peer_id();
        format!("{prefix}-{peer:x}-{n}")
    }

    /// Sorted IDs of every comment thread in the doc.
    pub fn comment_thread_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.comments_map().keys().map(|k| k.to_string()).collect();
        ids.sort();
        ids
    }

    /// Comment by thread_id, or `None` if absent / malformed.
    ///
    /// Old (pre-Phase-4.6) snapshots that lack `author`, `created_at`,
    /// `status`, or `replies` decode with safe defaults: empty author,
    /// `0` timestamp, `Open` status, no replies.
    pub fn comment(&self, thread_id: &str) -> Option<Comment> {
        let entry = match self.comments_map().get(thread_id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return None,
        };
        let from = read_i64_field(&entry, FIELD_FROM)? as usize;
        let to = read_i64_field(&entry, FIELD_TO)? as usize;
        let body = read_string_field(&entry, FIELD_BODY)?;
        let author = read_string_field(&entry, FIELD_AUTHOR).unwrap_or_default();
        let created_at = read_i64_field(&entry, FIELD_CREATED_AT).unwrap_or(0);
        let status = match read_string_field(&entry, FIELD_STATUS).as_deref() {
            Some(s) if s == STATUS_RESOLVED => CommentStatus::Resolved,
            _ => CommentStatus::Open,
        };
        let replies = read_replies(&entry);
        Some(Comment {
            thread_id: thread_id.to_string(),
            from,
            to,
            body,
            author,
            created_at,
            status,
            replies,
        })
    }

    /// Phase 4.6 — every comment thread in the doc, sorted by id.
    /// Built on `comment_thread_ids` + `comment` to share the malformed-
    /// entry handling. Excludes any thread whose record fails to
    /// decode (`comment` returned `None`).
    pub fn comments(&self) -> Vec<Comment> {
        self.comment_thread_ids()
            .into_iter()
            .filter_map(|id| self.comment(&id))
            .collect()
    }

    /// Sorted IDs of every suggestion (any state).
    pub fn suggestion_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .suggestions_map()
            .keys()
            .map(|k| k.to_string())
            .collect();
        ids.sort();
        ids
    }

    /// Suggestion by id, or `None` if absent / malformed.
    pub fn suggestion(&self, id: &str) -> Option<Suggestion> {
        let entry = match self.suggestions_map().get(id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return None,
        };
        let from = read_i64_field(&entry, FIELD_FROM)? as usize;
        let to = read_i64_field(&entry, FIELD_TO)? as usize;
        let replacement = read_string_field(&entry, FIELD_REPLACEMENT)?;
        let state_str = read_string_field(&entry, FIELD_STATE)?;
        Some(Suggestion {
            id: id.to_string(),
            from,
            to,
            replacement,
            state: str_to_state(&state_str),
        })
    }

    /// Sorted IDs of suggestions whose state is `Pending`.
    pub fn pending_suggestion_ids(&self) -> Vec<String> {
        self.suggestion_ids()
            .into_iter()
            .filter(|id| {
                self.suggestion(id)
                    .map(|s| s.state == SuggestionState::Pending)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// The thread_id assigned by the most recent `InsertComment`
    /// applied to this `Doc` instance. Transient — does NOT survive
    /// snapshot / merge. Read this immediately after
    /// `apply_edit_op(InsertComment)` to discover the generated id
    /// when the caller passed `thread_id: None`.
    pub fn last_comment_thread_id(&self) -> Option<String> {
        self.last_comment_thread_id.clone()
    }

    /// Phase 4.6 — reply id assigned by the most recent
    /// `apply_edit_op(ReplyToComment)`. Same transient lifetime as
    /// `last_comment_thread_id` — read immediately after the op.
    pub fn last_reply_id(&self) -> Option<String> {
        self.last_reply_id.clone()
    }

    /// The id assigned by the most recent `Suggest` applied to this
    /// `Doc` instance. Transient. Read after `apply_edit_op(Suggest)`
    /// to feed `EditOp::AcceptSuggestion { suggestion_id }`.
    pub fn last_suggestion_id(&self) -> Option<String> {
        self.last_suggestion_id.clone()
    }

    fn citations_map(&self) -> LoroMap {
        self.inner.get_map(CITATIONS_CONTAINER_ID)
    }

    fn footnotes_map(&self) -> LoroMap {
        self.inner.get_map(FOOTNOTES_CONTAINER_ID)
    }

    /// Sorted IDs of every citation in the doc (Phase D).
    pub fn citation_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.citations_map().keys().map(|k| k.to_string()).collect();
        ids.sort();
        ids
    }

    /// Citation by id, or `None` if absent / malformed (Phase D).
    pub fn citation(&self, id: &str) -> Option<Citation> {
        let entry = match self.citations_map().get(id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return None,
        };
        let at = read_i64_field(&entry, FIELD_AT)? as usize;
        let key = read_string_field(&entry, FIELD_KEY)?;
        Some(Citation {
            id: id.to_string(),
            at,
            key,
        })
    }

    /// The id assigned by the most recent `InsertCitation` applied
    /// to this `Doc` instance (Phase D, transient).
    pub fn last_citation_id(&self) -> Option<String> {
        self.last_citation_id.clone()
    }

    /// Sorted IDs of every footnote in the doc (Phase D).
    pub fn footnote_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.footnotes_map().keys().map(|k| k.to_string()).collect();
        ids.sort();
        ids
    }

    /// Footnote by id, or `None` if absent / malformed (Phase D).
    pub fn footnote(&self, id: &str) -> Option<Footnote> {
        let entry = match self.footnotes_map().get(id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return None,
        };
        let at = read_i64_field(&entry, FIELD_AT)? as usize;
        let blocks = read_block_tree_field(&entry, FIELD_BLOCKS)?;
        Some(Footnote {
            id: id.to_string(),
            at,
            body: BlockTree { blocks },
        })
    }

    /// The id assigned by the most recent `InsertFootnote` applied
    /// to this `Doc` instance (Phase D, transient).
    pub fn last_footnote_id(&self) -> Option<String> {
        self.last_footnote_id.clone()
    }
}

/// Decode a `BlockTree` from a sub-map field. The on-disk shape is
/// `LoroValue::List<LoroValue::Map<{kind: String, text: String}>>`.
/// Malformed entries decode to `Paragraph` with empty text — same
/// forward-compat policy as `str_to_kind`.
fn read_block_tree_field(entry: &LoroMap, key: &str) -> Option<Vec<Block>> {
    let LoroValue::List(list) = entry.get(key).and_then(|v| match v {
        ValueOrContainer::Value(v) => Some(v),
        _ => None,
    })?
    else {
        return None;
    };
    let blocks: Vec<Block> = list
        .iter()
        .map(|v| {
            let LoroValue::Map(m) = v else {
                return Block {
                    kind: BlockKind::Paragraph,
                    text: String::new(),
                };
            };
            let kind = m
                .get(FIELD_KIND)
                .and_then(|v| match v {
                    LoroValue::String(s) => Some(str_to_kind(s.as_ref())),
                    _ => None,
                })
                .unwrap_or(BlockKind::Paragraph);
            let text = m
                .get(FIELD_TEXT)
                .and_then(|v| match v {
                    LoroValue::String(s) => Some(s.to_string()),
                    _ => None,
                })
                .unwrap_or_default();
            Block { kind, text }
        })
        .collect();
    Some(blocks)
}

/// Encode a `BlockTree` as a `LoroValue` for storage in a sub-map
/// `blocks` field. Inverse of `read_block_tree_field`.
fn block_tree_to_value(blocks: &[Block]) -> LoroValue {
    let entries: Vec<LoroValue> = blocks
        .iter()
        .map(|b| {
            LoroValue::Map(LoroMapValue::from(vec![
                (
                    FIELD_KIND.to_string(),
                    LoroValue::String(kind_to_str(&b.kind).into()),
                ),
                (
                    FIELD_TEXT.to_string(),
                    LoroValue::String(b.text.clone().into()),
                ),
            ]))
        })
        .collect();
    LoroValue::List(LoroListValue::from(entries))
}

/// Read an `I64` field from a sub-map, returning `None` for absent /
/// wrong-typed entries (defensive against forward-compat shape drift).
fn read_i64_field(entry: &LoroMap, key: &str) -> Option<i64> {
    match entry.get(key) {
        Some(ValueOrContainer::Value(LoroValue::I64(n))) => Some(n),
        _ => None,
    }
}

/// Read a `String` field from a sub-map.
fn read_string_field(entry: &LoroMap, key: &str) -> Option<String> {
    match entry.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

/// Phase 4.6 — decode the `replies` LoroList stored under a thread
/// sub-LoroMap into a `Vec<CommentReply>`. Missing list (older
/// snapshot) → empty Vec. Malformed entries (not a sub-map) are
/// skipped so a partial corruption doesn't drop the whole thread.
/// List ordering is preserved — `LoroList::push_container` appends
/// at the end and `LoroList::get(i)` is sequential.
fn read_replies(thread_entry: &LoroMap) -> Vec<CommentReply> {
    let list = match thread_entry.get(FIELD_REPLIES) {
        Some(ValueOrContainer::Container(Container::List(l))) => l,
        _ => return Vec::new(),
    };
    let mut out = Vec::with_capacity(list.len());
    for i in 0..list.len() {
        let map = match list.get(i) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => continue,
        };
        let id = read_string_field(&map, FIELD_REPLY_ID).unwrap_or_default();
        let body = read_string_field(&map, FIELD_BODY).unwrap_or_default();
        let author = read_string_field(&map, FIELD_AUTHOR).unwrap_or_default();
        let created_at = read_i64_field(&map, FIELD_CREATED_AT).unwrap_or(0);
        out.push(CommentReply {
            id,
            body,
            author,
            created_at,
        });
    }
    out
}

/// Encode a `SuggestionState` as its canonical string. Phase C v0
/// only ever writes `pending` or `accepted`; `rejected` is here for
/// the future `RejectSuggestion` variant the blueprint will add.
fn state_to_str(state: SuggestionState) -> &'static str {
    match state {
        SuggestionState::Pending => STATE_PENDING,
        SuggestionState::Accepted => STATE_ACCEPTED,
        SuggestionState::Rejected => STATE_REJECTED,
    }
}

/// Decode a state string. Unknown / malformed strings fall back to
/// `Pending` so the suggestion remains actionable.
fn str_to_state(s: &str) -> SuggestionState {
    match s {
        STATE_ACCEPTED => SuggestionState::Accepted,
        STATE_REJECTED => SuggestionState::Rejected,
        _ => SuggestionState::Pending,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
}

/// A flat list of blocks. Used by `EditOp::InsertFootnote`. The
/// general case is recursive (footnotes containing footnotes), but
/// Phase A keeps it flat — Phase D will switch to a recursive
/// representation when the BlockTree gets its own Loro model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockTree {
    pub blocks: Vec<Block>,
}

/// Lifecycle of a comment thread (Phase 4.6). New threads land in
/// `Open`; `EditOp::SetCommentStatus` flips between Open and Resolved.
/// Serialized as lowercase strings ("open" / "resolved") to match
/// the JS-side `CommentStatus` literal union and the stored
/// `FIELD_STATUS` value in Loro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    Open,
    Resolved,
}

/// One reply inside a `Comment` thread (Phase 4.6). Replies are
/// stored as a LoroList of sub-LoroMaps under `FIELD_REPLIES` in the
/// thread's sub-map; this struct is the snapshot type returned by
/// `Doc::comment(id)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentReply {
    pub id: String,
    pub body: String,
    pub author: String,
    /// Epoch milliseconds. `i64` (not `u64`) so JS `Number`-safe and
    /// negative pre-1970 values are representable for fixture / test
    /// determinism.
    pub created_at: i64,
}

/// Snapshot of a comment thread anchored to a codepoint range.
///
/// Anchors are RAW positions — they go stale as the surrounding text
/// is edited. Stable anchors via Loro Cursor land when loro upstream
/// makes the Cursor / Side API public; the v0 UI surfaces a "(stale)"
/// badge when the anchored block was deleted.
///
/// Phase 4.6 added `author`, `created_at`, `status`, `replies`. Old
/// snapshots that lack these fields decode as: `author = ""`,
/// `created_at = 0`, `status = Open`, `replies = []`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub thread_id: String,
    pub from: Position,
    pub to: Position,
    pub body: String,
    pub author: String,
    /// Epoch milliseconds; see [`CommentReply::created_at`].
    pub created_at: i64,
    pub status: CommentStatus,
    pub replies: Vec<CommentReply>,
}

/// Lifecycle of a suggestion (Phase C). New suggestions land in
/// `Pending`; `AcceptSuggestion` flips to `Accepted` and applies the
/// text replacement. `Rejected` is reserved for a future
/// `RejectSuggestion` variant — Phase C v0 only ships accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuggestionState {
    Pending,
    Accepted,
    Rejected,
}

/// Snapshot of a suggestion record (Phase C). Same anchor caveat
/// as `Comment`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub from: Position,
    pub to: Position,
    pub replacement: String,
    pub state: SuggestionState,
}

/// Snapshot of a citation record (Phase D). The citation is
/// represented in the body as a single `\u{E000}` marker codepoint;
/// `at` is the codepoint position of that marker. `key` is the CSL
/// citation key (eg. "Smith2020"). Position anchors are RAW codepoints
/// — same staleness caveat as `Comment` and `Suggestion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub at: Position,
    pub key: String,
}

/// Snapshot of a footnote record (Phase D). The body carries a
/// `\u{E001}` marker codepoint at `at`. `body` is a flat `BlockTree`
/// of the footnote contents. v0 keeps the tree flat (no nested
/// footnotes); the recursive case ships when layout / OOXML round-
/// trip requires it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Footnote {
    pub id: String,
    pub at: Position,
    pub body: BlockTree,
}

/// Cross-boundary edit verb. All 11 variants are implemented as of
/// Phase 2.2 Phase D. The serde shape uses `tag = "kind"` so the JS
/// bridge serialises ops as `{kind: "InsertText", at, text, marks}`
/// matching the blueprint's Section G TypeScript declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
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
    /// Anchor a comment thread to `[from, to)` (Phase C, extended in
    /// Phase 4.6 with `author` and `created_at`).
    ///
    /// `thread_id`: `Some` to assign explicitly (migration / external
    /// linkage); `None` to mint via `next_id` and surface through
    /// `last_comment_thread_id()`.
    InsertComment {
        from: Position,
        to: Position,
        body: String,
        thread_id: Option<String>,
        author: String,
        /// Epoch milliseconds; see [`Comment::created_at`].
        created_at: i64,
    },
    /// Append a reply to an existing comment thread (Phase 4.6).
    /// Errors with [`Error::CommentNotFound`] if `thread_id` does not
    /// match an existing thread. The reply id is minted via
    /// `next_id` and surfaced through `last_reply_id()`.
    ReplyToComment {
        thread_id: String,
        body: String,
        author: String,
        created_at: i64,
    },
    /// Set a comment thread's status (Phase 4.6). Used to resolve a
    /// thread (`Resolved`) or reopen one (`Open`). Errors with
    /// [`Error::CommentNotFound`] for unknown thread ids. Idempotent
    /// for the same status (re-resolving a Resolved thread is a
    /// no-op).
    SetCommentStatus {
        thread_id: String,
        status: CommentStatus,
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
    /// `AcceptSuggestion` was called with an id that does not match
    /// any stored suggestion (or the stored entry was malformed).
    #[error("suggestion '{0}' not found")]
    SuggestionNotFound(String),
    /// `ReplyToComment` / `SetCommentStatus` was called with a
    /// thread_id that does not match any stored comment thread
    /// (Phase 4.6).
    #[error("comment thread '{0}' not found")]
    CommentNotFound(String),
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
            EditOp::InsertComment {
                from,
                to,
                body,
                thread_id,
                author,
                created_at,
            } => self.handle_insert_comment(from, to, body, thread_id, author, created_at),
            EditOp::ReplyToComment {
                thread_id,
                body,
                author,
                created_at,
            } => self.handle_reply_to_comment(thread_id, body, author, created_at),
            EditOp::SetCommentStatus { thread_id, status } => {
                self.handle_set_comment_status(thread_id, status)
            }
            EditOp::Suggest {
                from,
                to,
                replacement,
            } => self.handle_suggest(from, to, replacement),
            EditOp::AcceptSuggestion { suggestion_id } => {
                self.handle_accept_suggestion(suggestion_id)
            }
            EditOp::InsertCitation { at, key } => self.handle_insert_citation(at, key),
            EditOp::InsertFootnote { at, body } => self.handle_insert_footnote(at, body),
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

    /// Anchor a comment thread to `[from, to)`. If `thread_id` is
    /// `Some`, that id is used verbatim — useful for migrations and
    /// for re-linking from external systems. If `None`, a fresh id
    /// is generated via `next_id` and exposed via
    /// `last_comment_thread_id()`. Existing entries with the same id
    /// are overwritten field-by-field (last-write-wins per field).
    fn handle_insert_comment(
        &mut self,
        from: Position,
        to: Position,
        body: String,
        thread_id: Option<String>,
        author: String,
        created_at: i64,
    ) -> Result<(), Error> {
        let id = thread_id.unwrap_or_else(|| self.next_id("c", META_KEY_NEXT_COMMENT));
        let entry = self
            .comments_map()
            .get_or_create_container(&id, LoroMap::new())
            .expect("loro get_or_create_container should not fail");
        entry
            .insert(FIELD_FROM, LoroValue::I64(from as i64))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_TO, LoroValue::I64(to as i64))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_BODY, body)
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_AUTHOR, author)
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_CREATED_AT, LoroValue::I64(created_at))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_STATUS, STATUS_OPEN)
            .expect("loro map insert should not fail");
        self.last_comment_thread_id = Some(id);
        Ok(())
    }

    /// Phase 4.6 — append a reply to an existing thread. Returns
    /// [`Error::CommentNotFound`] if no thread carries `thread_id`.
    fn handle_reply_to_comment(
        &mut self,
        thread_id: String,
        body: String,
        author: String,
        created_at: i64,
    ) -> Result<(), Error> {
        let thread_entry = match self.comments_map().get(&thread_id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return Err(Error::CommentNotFound(thread_id)),
        };
        let replies = thread_entry
            .get_or_create_container(FIELD_REPLIES, LoroList::new())
            .expect("loro get_or_create_container should not fail");
        let reply_id = self.next_id("r", META_KEY_NEXT_REPLY);
        let reply_map = replies
            .push_container(LoroMap::new())
            .expect("loro push_container should not fail");
        reply_map
            .insert(FIELD_REPLY_ID, reply_id.clone())
            .expect("loro map insert should not fail");
        reply_map
            .insert(FIELD_BODY, body)
            .expect("loro map insert should not fail");
        reply_map
            .insert(FIELD_AUTHOR, author)
            .expect("loro map insert should not fail");
        reply_map
            .insert(FIELD_CREATED_AT, LoroValue::I64(created_at))
            .expect("loro map insert should not fail");
        self.last_reply_id = Some(reply_id);
        Ok(())
    }

    /// Phase 4.6 — set a thread's status (open ↔ resolved). Returns
    /// [`Error::CommentNotFound`] for unknown ids. Idempotent: setting
    /// the same status twice is a successful no-op.
    fn handle_set_comment_status(
        &mut self,
        thread_id: String,
        status: CommentStatus,
    ) -> Result<(), Error> {
        let thread_entry = match self.comments_map().get(&thread_id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return Err(Error::CommentNotFound(thread_id)),
        };
        let s = match status {
            CommentStatus::Open => STATUS_OPEN,
            CommentStatus::Resolved => STATUS_RESOLVED,
        };
        thread_entry
            .insert(FIELD_STATUS, s)
            .expect("loro map insert should not fail");
        Ok(())
    }

    /// Record a suggested replacement for `[from, to)`. The doc text
    /// is NOT mutated — that happens on `AcceptSuggestion`. The new
    /// suggestion lands in `Pending` state. The generated id is
    /// surfaced via `last_suggestion_id()`.
    fn handle_suggest(
        &mut self,
        from: Position,
        to: Position,
        replacement: String,
    ) -> Result<(), Error> {
        let id = self.next_id("s", META_KEY_NEXT_SUGGESTION);
        let entry = self
            .suggestions_map()
            .get_or_create_container(&id, LoroMap::new())
            .expect("loro get_or_create_container should not fail");
        entry
            .insert(FIELD_FROM, LoroValue::I64(from as i64))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_TO, LoroValue::I64(to as i64))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_REPLACEMENT, replacement)
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_STATE, state_to_str(SuggestionState::Pending))
            .expect("loro map insert should not fail");
        self.last_suggestion_id = Some(id);
        Ok(())
    }

    /// Apply a previously-recorded suggestion: delete `[from, to)`,
    /// insert `replacement` at `from`, mark the suggestion as
    /// `Accepted`. Already-accepted/rejected suggestions are no-ops
    /// (idempotent). Unknown ids return `Error::SuggestionNotFound`.
    fn handle_accept_suggestion(&mut self, id: String) -> Result<(), Error> {
        let entry = match self.suggestions_map().get(&id) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            _ => return Err(Error::SuggestionNotFound(id)),
        };
        let state = read_string_field(&entry, FIELD_STATE)
            .map(|s| str_to_state(&s))
            .ok_or_else(|| Error::SuggestionNotFound(id.clone()))?;
        if state != SuggestionState::Pending {
            return Ok(());
        }
        let from = read_i64_field(&entry, FIELD_FROM)
            .ok_or_else(|| Error::SuggestionNotFound(id.clone()))? as usize;
        let to = read_i64_field(&entry, FIELD_TO)
            .ok_or_else(|| Error::SuggestionNotFound(id.clone()))? as usize;
        let replacement = read_string_field(&entry, FIELD_REPLACEMENT)
            .ok_or_else(|| Error::SuggestionNotFound(id.clone()))?;
        self.delete(from..to);
        self.insert(from, &replacement);
        entry
            .insert(FIELD_STATE, state_to_str(SuggestionState::Accepted))
            .expect("loro map insert should not fail");
        Ok(())
    }

    /// Insert a citation marker `\u{E000}` at codepoint position
    /// `at` and record `(at, key)` under a generated id. The `at`
    /// stored in the record is the CLIPPED position (matches the
    /// marker's actual location after `min(len)`), not the raw
    /// input. Subsequent body edits do not update this stored
    /// position — the same anchor caveat as Phase C.
    fn handle_insert_citation(&mut self, at: Position, key: String) -> Result<(), Error> {
        let id = self.next_id("cit", META_KEY_NEXT_CITATION);
        let body = self.body();
        let len = body.len_unicode();
        let at = at.min(len);
        body.insert(at, &CITATION_MARKER.to_string())
            .expect("loro insert should not fail");
        let entry = self
            .citations_map()
            .get_or_create_container(&id, LoroMap::new())
            .expect("loro get_or_create_container should not fail");
        entry
            .insert(FIELD_AT, LoroValue::I64(at as i64))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_KEY, key)
            .expect("loro map insert should not fail");
        self.last_citation_id = Some(id);
        Ok(())
    }

    /// Insert a footnote marker `\u{E001}` at codepoint position
    /// `at` and record `(at, body)` under a generated id. Same anchor
    /// caveat as `handle_insert_citation`.
    fn handle_insert_footnote(&mut self, at: Position, body: BlockTree) -> Result<(), Error> {
        let id = self.next_id("fn", META_KEY_NEXT_FOOTNOTE);
        let body_text = self.body();
        let len = body_text.len_unicode();
        let at = at.min(len);
        body_text
            .insert(at, &FOOTNOTE_MARKER.to_string())
            .expect("loro insert should not fail");
        let entry = self
            .footnotes_map()
            .get_or_create_container(&id, LoroMap::new())
            .expect("loro get_or_create_container should not fail");
        entry
            .insert(FIELD_AT, LoroValue::I64(at as i64))
            .expect("loro map insert should not fail");
        entry
            .insert(FIELD_BLOCKS, block_tree_to_value(&body.blocks))
            .expect("loro map insert should not fail");
        self.last_footnote_id = Some(id);
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

    #[test]
    fn str_to_state_decodes_all_three_kinds() {
        // The `STATE_REJECTED` arm is unreachable through the public
        // API in Phase C v0 (no `RejectSuggestion` variant ships
        // yet), but it must still decode correctly for forward-compat
        // when a future phase introduces `Reject`. This unit test
        // also pins the unknown-string fall-through to `Pending`.
        assert_eq!(str_to_state(STATE_PENDING), SuggestionState::Pending);
        assert_eq!(str_to_state(STATE_ACCEPTED), SuggestionState::Accepted);
        assert_eq!(str_to_state(STATE_REJECTED), SuggestionState::Rejected);
        assert_eq!(str_to_state("garbage"), SuggestionState::Pending);
    }

    #[test]
    fn state_to_str_round_trips_all_three_kinds() {
        // Symmetric pin for the encode side. Catches any silent
        // string-rename mutation (`STATE_REJECTED` is otherwise only
        // referenced by `str_to_state`, so a renamed const wouldn't
        // be caught without this).
        assert_eq!(
            str_to_state(state_to_str(SuggestionState::Pending)),
            SuggestionState::Pending
        );
        assert_eq!(
            str_to_state(state_to_str(SuggestionState::Accepted)),
            SuggestionState::Accepted
        );
        assert_eq!(
            str_to_state(state_to_str(SuggestionState::Rejected)),
            SuggestionState::Rejected
        );
    }
}
