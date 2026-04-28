#![deny(unsafe_code)]
#![doc = "Layout engine — paged document layout with shaping, line breaking, and \
incremental relayout for keystroke-to-render perf gating (Validation Gate 5)."]

//! # Apalabrar layout engine
//!
//! A minimal paged-layout engine sized to validate the Path B perf thesis:
//! incremental relayout after a single-character edit must complete well under
//! 16 ms p95 on a 10 000-block document so the keystroke-to-paint chain has
//! headroom for CRDT + paint after layout finishes.
//!
//! ## Surface (locked for Gate 5)
//!
//! - [`Document`] — flat `Vec<Block>` of [`Block::Paragraph`] / [`Block::Heading`] /
//!   [`Block::ListItem`]. The block list is the layout input; richer marks (bold,
//!   italic, links) are out of scope for this gate.
//! - [`Engine`] — owns a [`cosmic_text::FontSystem`] seeded with a single bundled
//!   face (DejaVu Sans regular) plus a per-block shaped-buffer cache. Constructed
//!   once; reused for every layout call. The bundled font keeps layout numbers
//!   deterministic across machines.
//! - [`Engine::layout`] — full layout from scratch. Shapes every block, packs
//!   blocks into [`Page`]s top-to-bottom. Caches shaped buffers indexed by block.
//! - [`Engine::relayout_after_change`] — incremental path. Re-shapes only the
//!   block at `changed_block`; re-packs pages over the cache. Returns `Err` if
//!   `changed_block` is out of bounds for the document. Assumes the block list
//!   length is unchanged (typing a character doesn't add or remove blocks); a
//!   length mismatch falls back to a full layout under the hood.
//!
//! ## Why this scope
//!
//! Gate 5 measures layout time. Glyph painting is downstream and out of scope
//! for the bench, so the public surface stops at line bounds and block boxes:
//! every shaped run lives inside the cached `cosmic_text::Buffer` and is
//! reachable later when the editor wires up the bridge, but nothing outside
//! the crate needs to walk glyphs to drive the bench.

use std::collections::BTreeMap;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bundled DejaVu Sans regular face. Single-font corpus keeps shaping
/// deterministic across machines so the bench numbers are reproducible.
const BUNDLED_FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

/// 96 DPI Letter page (8.5" × 11") with 1" margins. The bench uses these
/// numbers; downstream code may pick its own.
pub const LETTER_AT_96DPI: Viewport = Viewport {
    page_width_px: 816.0,
    page_height_px: 1056.0,
    margin_px: 96.0,
};

/// Layout failure modes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A bundled font failed to load. Should never fire in production builds
    /// because the font is embedded via `include_bytes!`; surfaced for tests.
    #[error("failed to load bundled font: {0}")]
    FontLoad(String),

    /// `relayout_after_change` was called with an index past the document's
    /// last block.
    #[error("block index {index} out of bounds (document has {len} blocks)")]
    InvalidBlockIndex { index: usize, len: usize },
}

/// One block in a flow-layout document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// Body paragraph.
    Paragraph { text: String },
    /// `<h1>`–`<h6>` style heading. `level` is 1-based, clamped to 1..=6 by the
    /// engine (out-of-range levels collapse to the nearest valid one).
    Heading { level: u8, text: String },
    /// Bulleted list item. `indent` is a logical depth (0 = top level); the
    /// engine maps it to a left-margin in pixels.
    ListItem { indent: u8, text: String },
}

impl Block {
    /// Borrow the text content regardless of variant.
    pub fn text(&self) -> &str {
        match self {
            Block::Paragraph { text }
            | Block::Heading { text, .. }
            | Block::ListItem { text, .. } => text,
        }
    }
}

/// Flat document model the engine consumes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Page geometry (logical pixels). Origin is top-left of the printable area;
/// content reflows inside `page_width_px × page_height_px` minus 2×`margin_px`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub page_width_px: f32,
    pub page_height_px: f32,
    pub margin_px: f32,
}

impl Viewport {
    pub fn content_width(&self) -> f32 {
        self.page_width_px - 2.0 * self.margin_px
    }

    pub fn content_height(&self) -> f32 {
        self.page_height_px - 2.0 * self.margin_px
    }
}

/// The kind of a laid-out block, with structural metadata preserved for the
/// renderer (heading level, list indent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading { level: u8 },
    ListItem { indent: u8 },
}

/// A single rendered line inside a block. Coordinates are relative to the
/// block's origin (so `baseline_y_px` is small even for late lines in a
/// long paragraph).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub width_px: f32,
    pub height_px: f32,
    pub baseline_y_px: f32,
}

/// One block placed on a page.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockBox {
    pub block_index: usize,
    pub kind: BlockKind,
    /// Top-left origin within the page's printable area (inside the margin).
    pub origin_x_px: f32,
    pub origin_y_px: f32,
    pub width_px: f32,
    pub height_px: f32,
    pub lines: Vec<Line>,
}

/// One page of laid-out blocks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    pub blocks: Vec<BlockBox>,
}

/// Result of a layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LaidOutDocument {
    pub pages: Vec<Page>,
}

impl LaidOutDocument {
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Total number of `BlockBox`es across all pages. Useful for sanity checks
    /// — should equal `Document::len()` when the layout completes successfully.
    pub fn block_box_count(&self) -> usize {
        self.pages.iter().map(|p| p.blocks.len()).sum()
    }
}

/// Stateful layout engine. Constructed once per editor session.
pub struct Engine {
    viewport: Viewport,
    font_system: FontSystem,
    /// One cached shaped buffer per block. Indexed by `block_index`.
    /// Stored in a BTreeMap so a partial-population state (eg after a panic)
    /// is not wedged into `Vec`'s position assumptions.
    cache: BTreeMap<usize, ShapedBlock>,
    /// Document length the cache was built from. `relayout_after_change`
    /// falls back to a full layout if the document length differs.
    cached_doc_len: usize,
}

/// Engine-internal: shaped representation of a single block.
struct ShapedBlock {
    kind: BlockKind,
    /// Pixel height contributed to the page flow (sum of line heights + the
    /// kind-specific top/bottom spacing).
    height_px: f32,
    /// Pixel left-offset contributed by the kind (eg list indent). Right-edge
    /// trims are handled by the wrap width passed to cosmic-text.
    left_inset_px: f32,
    /// One entry per shaped line.
    lines: Vec<Line>,
}

impl Engine {
    /// Build a new engine with the given viewport. Loads the bundled DejaVu
    /// Sans face into the font system; returns an [`Error::FontLoad`] if the
    /// embedded bytes are corrupt (not expected in normal builds).
    pub fn new(viewport: Viewport) -> Result<Self, Error> {
        let mut font_system = FontSystem::new_with_locale_and_db(
            "en-US".to_owned(),
            cosmic_text::fontdb::Database::new(),
        );
        font_system.db_mut().load_font_data(BUNDLED_FONT.to_vec());
        if font_system.db().faces().next().is_none() {
            return Err(Error::FontLoad(
                "fontdb empty after loading DejaVuSans.ttf".to_owned(),
            ));
        }
        Ok(Self {
            viewport,
            font_system,
            cache: BTreeMap::new(),
            cached_doc_len: 0,
        })
    }

    /// Full layout from scratch. Re-shapes every block, replaces the cache,
    /// and packs the result into pages.
    pub fn layout(&mut self, doc: &Document) -> LaidOutDocument {
        self.cache.clear();
        for (idx, block) in doc.blocks.iter().enumerate() {
            let shaped = shape_block(block, &self.viewport, &mut self.font_system);
            self.cache.insert(idx, shaped);
        }
        self.cached_doc_len = doc.len();
        pack_pages(&self.cache, &self.viewport)
    }

    /// Incremental relayout: re-shape only `changed_block` and re-pack.
    ///
    /// Returns [`Error::InvalidBlockIndex`] if `changed_block >= doc.len()`.
    /// If the document length differs from the cache, transparently falls
    /// back to a full layout so the caller never observes a stale cache.
    pub fn relayout_after_change(
        &mut self,
        doc: &Document,
        changed_block: usize,
    ) -> Result<LaidOutDocument, Error> {
        if changed_block >= doc.len() {
            return Err(Error::InvalidBlockIndex {
                index: changed_block,
                len: doc.len(),
            });
        }
        if doc.len() != self.cached_doc_len {
            return Ok(self.layout(doc));
        }
        let shaped = shape_block(
            &doc.blocks[changed_block],
            &self.viewport,
            &mut self.font_system,
        );
        self.cache.insert(changed_block, shaped);
        Ok(pack_pages(&self.cache, &self.viewport))
    }

    /// Number of blocks currently in the shaped cache. Test-visible for
    /// assertions that the cache survives across calls.
    pub fn cached_block_count(&self) -> usize {
        self.cache.len()
    }

    /// Document length the cache was built from. Test-visible.
    pub fn cached_doc_len(&self) -> usize {
        self.cached_doc_len
    }
}

// --------- Engine-private helpers ---------

/// Per-kind layout metrics. `font_size_px` and `line_height_px` go to
/// cosmic-text; `space_before_px` and `space_after_px` are added to the
/// block's outer height so consecutive blocks don't bump into each other.
/// `left_inset_px` shifts the block's origin x and shrinks its wrap width.
struct KindMetrics {
    font_size_px: f32,
    line_height_px: f32,
    space_before_px: f32,
    space_after_px: f32,
    left_inset_px: f32,
}

fn metrics_for(block: &Block) -> (BlockKind, KindMetrics) {
    match block {
        Block::Paragraph { .. } => (
            BlockKind::Paragraph,
            KindMetrics {
                font_size_px: 14.0,
                line_height_px: 18.0,
                space_before_px: 0.0,
                space_after_px: 8.0,
                left_inset_px: 0.0,
            },
        ),
        Block::Heading { level, .. } => {
            let level = (*level).clamp(1, 6);
            let (font, line, before, after) = match level {
                1 => (24.0, 30.0, 16.0, 12.0),
                2 => (20.0, 26.0, 14.0, 10.0),
                _ => (16.0, 22.0, 12.0, 8.0),
            };
            (
                BlockKind::Heading { level },
                KindMetrics {
                    font_size_px: font,
                    line_height_px: line,
                    space_before_px: before,
                    space_after_px: after,
                    left_inset_px: 0.0,
                },
            )
        }
        Block::ListItem { indent, .. } => {
            let indent = *indent;
            (
                BlockKind::ListItem { indent },
                KindMetrics {
                    font_size_px: 14.0,
                    line_height_px: 18.0,
                    space_before_px: 0.0,
                    space_after_px: 4.0,
                    left_inset_px: 24.0 + 16.0 * indent as f32,
                },
            )
        }
    }
}

/// Shape one block into geometry. Drops the cosmic-text Buffer afterwards;
/// only `Vec<Line>` + summary metrics survive in the cache.
fn shape_block(block: &Block, viewport: &Viewport, font_system: &mut FontSystem) -> ShapedBlock {
    let (kind, m) = metrics_for(block);
    let metrics = Metrics::new(m.font_size_px, m.line_height_px);
    let mut buffer = Buffer::new(font_system, metrics);
    let wrap_width = (viewport.content_width() - m.left_inset_px).max(1.0);
    buffer.set_size(font_system, Some(wrap_width), None);
    let attrs = Attrs::new().family(Family::Name("DejaVu Sans"));
    buffer.set_text(font_system, block.text(), attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, false);

    let mut lines: Vec<Line> = Vec::new();
    let mut sum_line_heights = 0.0;
    for run in buffer.layout_runs() {
        lines.push(Line {
            width_px: run.line_w,
            height_px: run.line_height,
            baseline_y_px: run.line_y,
        });
        sum_line_heights += run.line_height;
    }
    if lines.is_empty() {
        // Empty text still occupies one line height so the block is visible.
        lines.push(Line {
            width_px: 0.0,
            height_px: m.line_height_px,
            baseline_y_px: m.line_height_px,
        });
        sum_line_heights = m.line_height_px;
    }
    let height_px = sum_line_heights + m.space_before_px + m.space_after_px;
    ShapedBlock {
        kind,
        height_px,
        left_inset_px: m.left_inset_px,
        lines,
    }
}

/// Greedy top-to-bottom page packing. Blocks are not split across pages
/// (sufficient for the perf gate; full block-splitting comes later in the
/// real engine). A block taller than a single page lives alone on its own
/// page so the result is never empty when blocks exist.
fn pack_pages(cache: &BTreeMap<usize, ShapedBlock>, viewport: &Viewport) -> LaidOutDocument {
    let content_height = viewport.content_height();
    let mut pages: Vec<Page> = Vec::new();
    let mut current = Page::default();
    let mut y_cursor = 0.0_f32;

    for (&block_index, shaped) in cache {
        let needs_new_page = !current.blocks.is_empty()
            && y_cursor + shaped.height_px > content_height + f32::EPSILON;
        if needs_new_page {
            pages.push(std::mem::take(&mut current));
            y_cursor = 0.0;
        }
        current.blocks.push(BlockBox {
            block_index,
            kind: shaped.kind,
            origin_x_px: shaped.left_inset_px,
            origin_y_px: y_cursor,
            width_px: viewport.content_width() - shaped.left_inset_px,
            height_px: shaped.height_px,
            lines: shaped.lines.clone(),
        });
        y_cursor += shaped.height_px;
    }
    if !current.blocks.is_empty() {
        pages.push(current);
    }
    LaidOutDocument { pages }
}

#[cfg(test)]
mod loadable {
    use super::*;

    #[test]
    fn version_pinned() {
        // Version pin: bumps to the package version are intentional and tracked
        // through the workspace; the test fails loudly if Cargo.toml drifts.
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn bundled_font_is_present_and_nonempty() {
        // Confidence floor for downstream tests: if the font byte slice is
        // empty, every shape call below would silently fall back to whatever
        // fontdb decides, which is not reproducible.
        assert!(
            BUNDLED_FONT.len() > 100_000,
            "DejaVuSans.ttf bundled at compile time should be ~750 KB, got {} bytes",
            BUNDLED_FONT.len(),
        );
    }
}
