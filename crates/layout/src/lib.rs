#![deny(unsafe_code)]
#![doc = "Layout engine — paged document layout from `apalabrar-doc-model::Doc` to `RenderPlan`."]

//! # Apalabrar layout engine (Phase 4.1)
//!
//! Single public entry point: [`layout`]. It consumes the canonical
//! [`apalabrar_doc_model::Doc`] (the Loro CRDT-backed document) and a
//! [`Viewport`] and produces a [`RenderPlan`] the JS shell can paint.
//!
//! ## Surface
//!
//! - [`layout`] — pure free function; no engine handle to manage.
//! - [`RenderPlan`] — the full output: pages + dirty rects (Phase 4.1
//!   scope; `glyph_runs`/`selections`/`carets` from blueprint Section
//!   G land in 4.2 on top of `#[non_exhaustive]`).
//! - [`Viewport`], [`LETTER_AT_96DPI`] — page geometry.
//! - [`BlockKind`], [`Page`], [`BlockBox`], [`Line`], [`Rect`] — the
//!   pieces of [`RenderPlan`].
//! - [`Error`] — single failure mode: a viewport whose margins
//!   collapsed the content area to zero or negative.
//!
//! ## Determinism
//!
//! Shaping uses a single bundled DejaVu Sans face; no machine-local
//! font fallback runs. Two calls with the same `Doc` and `Viewport`
//! produce byte-identical [`RenderPlan`]s.

use std::cell::RefCell;
use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};

pub mod shaping;

pub use shaping::{GlyphRun, PositionedGlyph};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bundled DejaVu Sans regular face. Single-font corpus keeps shaping
/// deterministic across machines so snapshots and benches reproduce.
const BUNDLED_FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

/// 96 DPI Letter page (8.5" × 11") with 1" margins.
pub const LETTER_AT_96DPI: Viewport = Viewport {
    page_width_px: 816.0,
    page_height_px: 1056.0,
    margin_px: 96.0,
};

/// Layout failure modes.
///
/// Marked `#[non_exhaustive]` so 4.2/4.3 can add variants (eg. font
/// loading once we accept user-supplied fonts) without a SemVer break.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The viewport's content area collapsed to a non-positive value
    /// (margins ≥ half the page). Layout has nowhere to flow blocks.
    #[error("viewport content area is non-positive: {content_width_px}×{content_height_px} px")]
    EmptyContentArea {
        content_width_px: f32,
        content_height_px: f32,
    },
}

/// Page geometry (logical pixels). Origin is the top-left of the
/// printable area (inside the margin).
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

/// Axis-aligned rectangle in viewport pixels. Origin is top-left of
/// the page's printable area.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x_px: f32,
    pub y_px: f32,
    pub width_px: f32,
    pub height_px: f32,
}

/// Kind of a laid-out block, with structural metadata preserved for
/// the renderer (heading level, list indent). Mirrors the doc-model's
/// `BlockKind` shape; we keep it as a layout-local `Copy` enum so
/// downstream `.iter().map(|b| b.kind)` patterns work without clones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading { level: u8 },
    ListItem { indent: u8 },
}

/// One rendered line inside a block. Coordinates are relative to the
/// block's origin.
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

/// Output of [`layout`]. Phase 4.1 filled `pages` and `dirty_rects`;
/// Phase 4.2 adds `glyph_runs` (one entry per shaped line). Future
/// phases add `selections` and `carets` on top of `#[non_exhaustive]`.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderPlan {
    pub pages: Vec<Page>,
    pub dirty_rects: Vec<Rect>,
    pub glyph_runs: Vec<GlyphRun>,
}

impl RenderPlan {
    /// Number of laid-out pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Total `BlockBox` count across pages. Equals `doc.block_count()`
    /// for any successful layout pass.
    pub fn block_box_count(&self) -> usize {
        self.pages.iter().map(|p| p.blocks.len()).sum()
    }
}

/// Lay out `doc` against `viewport` and return a [`RenderPlan`].
pub fn layout(doc: &apalabrar_doc_model::Doc, viewport: &Viewport) -> Result<RenderPlan, Error> {
    let cw = viewport.content_width();
    let ch = viewport.content_height();
    if cw <= 0.0 || ch <= 0.0 {
        return Err(Error::EmptyContentArea {
            content_width_px: cw,
            content_height_px: ch,
        });
    }

    // `Doc::blocks()` projects all blocks in O(text + block_count); calling
    // `Doc::block(idx)` in a loop instead would re-clone and re-split the
    // body per block (O(N²) on a 6600-block doc).
    let blocks = doc.blocks();
    let cw_round = cw.round() as i32;

    // Thread-local cache reuses the cosmic-text FontSystem and the per-block
    // shaped lines across calls. The free-fn API stays stateless from the
    // caller's perspective; the cache is an internal optimisation. Cache
    // keys are content-addressable on (viewport content width, kind, text)
    // so re-laying out the same document hits the cache 100 % and only the
    // page-pack runs (≈ms instead of ≈hundreds of ms cold).
    let (pages, glyph_runs) = SHAPING_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.shapes.len() > SHAPING_CACHE_CAPACITY {
            cache.shapes.clear();
        }
        let mut shaped: Vec<ShapedBlock> = Vec::with_capacity(blocks.len());
        let mut glyph_runs: Vec<GlyphRun> = Vec::new();
        for (idx, block) in blocks.iter().enumerate() {
            let key = CacheKey::new(cw_round, &block.kind, &block.text);
            let cached = match cache.shapes.get(&key) {
                Some(c) => c.clone(),
                None => {
                    let fs = cache.font_system.get_or_insert_with(build_font_system);
                    let s = shape_uncached(&block.kind, &block.text, viewport, fs);
                    cache.shapes.insert(key, s.clone());
                    s
                }
            };
            // Emit one GlyphRun per shaped non-empty line. Empty
            // line_glyphs are the synthetic placeholder for empty
            // paragraphs; the renderer has nothing to paint there.
            for (line_idx, line_glyphs) in cached.line_glyphs.iter().enumerate() {
                if line_glyphs.is_empty() {
                    continue;
                }
                glyph_runs.push(GlyphRun {
                    block_index: idx,
                    line_index: line_idx,
                    font_size_px: cached.font_size_px,
                    baseline_y_px: cached.lines[line_idx].baseline_y_px,
                    glyphs: line_glyphs.clone(),
                });
            }
            shaped.push(ShapedBlock {
                block_index: idx,
                kind: cached.kind,
                height_px: cached.height_px,
                left_inset_px: cached.left_inset_px,
                lines: cached.lines,
            });
        }
        (pack_pages(&shaped, viewport), glyph_runs)
    });

    let dirty_rects: Vec<Rect> = pages
        .iter()
        .map(|_| Rect {
            x_px: 0.0,
            y_px: 0.0,
            width_px: cw,
            height_px: ch,
        })
        .collect();

    Ok(RenderPlan {
        pages,
        dirty_rects,
        glyph_runs,
    })
}

// ─────────────────────────────────────────────────────────────────
// Internals: thread-local cache, font system, shaping, page packing.
// ─────────────────────────────────────────────────────────────────

/// Hard cap on cached entries. On overflow the cache clears wholesale —
/// LRU would be more accurate but the bound exists for memory safety,
/// not hot-path correctness.
const SHAPING_CACHE_CAPACITY: usize = 100_000;

/// Per-thread cache of shaped block geometry plus the `FontSystem`
/// itself. Both are expensive to recreate; both are safe to reuse for
/// any number of layout passes on the current thread.
struct ShapingCache {
    font_system: Option<FontSystem>,
    shapes: HashMap<CacheKey, CachedShape>,
}

thread_local! {
    static SHAPING_CACHE: RefCell<ShapingCache> = RefCell::new(ShapingCache {
        font_system: None,
        shapes: HashMap::new(),
    });
}

/// Hash key for a shaped block. Captures everything that influences the
/// shaped output: viewport content width (rounded to integer pixels —
/// sub-pixel viewport drift never changes the wrap), kind (with its
/// level / indent payload), and the block's text bytes.
#[derive(Hash, Eq, PartialEq)]
struct CacheKey {
    content_width_round: i32,
    kind_tag: u8,
    kind_param: u8,
    text: String,
}

impl CacheKey {
    fn new(content_width_round: i32, kind: &apalabrar_doc_model::BlockKind, text: &str) -> Self {
        use apalabrar_doc_model::BlockKind as DK;
        let (kind_tag, kind_param) = match kind {
            DK::Paragraph => (0, 0),
            DK::Heading { level } => (1, (*level).clamp(1, 6)),
            DK::ListItem { indent } => (2, *indent),
        };
        Self {
            content_width_round,
            kind_tag,
            kind_param,
            text: text.to_owned(),
        }
    }
}

/// Cached shaped geometry for one block. Block index is NOT part of the
/// cached value because a block's index in the doc changes as siblings
/// are inserted / deleted; the shaped lines do not.
#[derive(Clone)]
struct CachedShape {
    kind: BlockKind,
    height_px: f32,
    left_inset_px: f32,
    /// Font size used for every shaped line in this block. Phase 4.2
    /// emits one shape per block, so a single value covers all lines.
    font_size_px: f32,
    lines: Vec<Line>,
    /// One entry per shaped (cosmic-text-emitted) line, parallel to
    /// `lines`. Empty when the block has no shaped output (the
    /// synthetic placeholder line for empty paragraphs).
    line_glyphs: Vec<Vec<PositionedGlyph>>,
}

fn build_font_system() -> FontSystem {
    let mut fs = FontSystem::new_with_locale_and_db(
        "en-US".to_owned(),
        cosmic_text::fontdb::Database::new(),
    );
    fs.db_mut().load_font_data(BUNDLED_FONT.to_vec());
    fs
}

/// Per-kind layout metrics. `font_size_px` and `line_height_px` go to
/// cosmic-text; `space_before_px` and `space_after_px` are added to
/// the block's outer height. `left_inset_px` shifts the block's
/// origin x and shrinks its wrap width.
struct KindMetrics {
    font_size_px: f32,
    line_height_px: f32,
    space_before_px: f32,
    space_after_px: f32,
    left_inset_px: f32,
}

fn metrics_for_kind(kind: &apalabrar_doc_model::BlockKind) -> (BlockKind, KindMetrics) {
    use apalabrar_doc_model::BlockKind as DK;
    match kind {
        DK::Paragraph => (
            BlockKind::Paragraph,
            KindMetrics {
                font_size_px: 14.0,
                line_height_px: 18.0,
                space_before_px: 0.0,
                space_after_px: 8.0,
                left_inset_px: 0.0,
            },
        ),
        DK::Heading { level } => {
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
        DK::ListItem { indent } => {
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

/// Per-block shaped geometry plus its index in the doc. The pack
/// step consumes a `Vec` of these top-to-bottom.
struct ShapedBlock {
    block_index: usize,
    kind: BlockKind,
    height_px: f32,
    left_inset_px: f32,
    lines: Vec<Line>,
}

/// Shape one block without touching the cache. Returns a [`CachedShape`]
/// — the part of the geometry that's stable across `block_index`
/// renumberings, so the result is cache-safe.
fn shape_uncached(
    kind: &apalabrar_doc_model::BlockKind,
    text: &str,
    viewport: &Viewport,
    font_system: &mut FontSystem,
) -> CachedShape {
    let (kind, m) = metrics_for_kind(kind);
    let metrics = Metrics::new(m.font_size_px, m.line_height_px);
    let mut buffer = Buffer::new(font_system, metrics);
    let wrap_width = (viewport.content_width() - m.left_inset_px).max(1.0);
    buffer.set_size(font_system, Some(wrap_width), None);
    let attrs = Attrs::new().family(Family::Name("DejaVu Sans"));
    buffer.set_text(font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, false);

    let mut lines: Vec<Line> = Vec::new();
    let mut line_glyphs: Vec<Vec<PositionedGlyph>> = Vec::new();
    let mut sum_line_heights = 0.0;
    for run in buffer.layout_runs() {
        lines.push(Line {
            width_px: run.line_w,
            height_px: run.line_height,
            baseline_y_px: run.line_y,
        });
        let mut glyphs: Vec<PositionedGlyph> = Vec::with_capacity(run.glyphs.len());
        for g in run.glyphs.iter() {
            glyphs.push(PositionedGlyph {
                glyph_id: g.glyph_id,
                cluster_start: byte_to_codepoint(text, g.start),
                cluster_end: byte_to_codepoint(text, g.end),
                x_px: g.x,
                y_px: g.y,
                width_px: g.w,
            });
        }
        line_glyphs.push(glyphs);
        sum_line_heights += run.line_height;
    }
    let height_px = sum_line_heights + m.space_before_px + m.space_after_px;
    CachedShape {
        kind,
        height_px,
        left_inset_px: m.left_inset_px,
        font_size_px: m.font_size_px,
        lines,
        line_glyphs,
    }
}

/// Convert a byte offset within `text` to the corresponding codepoint
/// (char) offset. Byte offsets that fall in the middle of a multi-byte
/// character or past the end clamp to the nearest valid boundary.
fn byte_to_codepoint(text: &str, byte_offset: usize) -> u32 {
    let clamped = byte_offset.min(text.len());
    text[..clamped].chars().count() as u32
}

/// Greedy top-to-bottom page packing. A block taller than a single
/// page lives alone on its own page so the result is never empty
/// when blocks exist.
fn pack_pages(shaped: &[ShapedBlock], viewport: &Viewport) -> Vec<Page> {
    let content_height = viewport.content_height();
    let mut pages: Vec<Page> = Vec::new();
    let mut current = Page::default();
    let mut y_cursor = 0.0_f32;

    for sb in shaped {
        let needs_new_page =
            !current.blocks.is_empty() && y_cursor + sb.height_px > content_height + f32::EPSILON;
        if needs_new_page {
            pages.push(std::mem::take(&mut current));
            y_cursor = 0.0;
        }
        current.blocks.push(BlockBox {
            block_index: sb.block_index,
            kind: sb.kind,
            origin_x_px: sb.left_inset_px,
            origin_y_px: y_cursor,
            width_px: viewport.content_width() - sb.left_inset_px,
            height_px: sb.height_px,
            lines: sb.lines.clone(),
        });
        y_cursor += sb.height_px;
    }
    if !current.blocks.is_empty() {
        pages.push(current);
    }
    pages
}

#[cfg(test)]
mod loadable {
    use super::*;

    #[test]
    fn version_pinned() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn bundled_font_is_present_and_nonempty() {
        assert!(
            BUNDLED_FONT.len() > 100_000,
            "DejaVuSans.ttf bundled at compile time should be ~750 KB, got {} bytes",
            BUNDLED_FONT.len(),
        );
    }
}
