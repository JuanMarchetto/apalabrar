//! Phase 4.2 — text-shaping output exposed in [`crate::RenderPlan`].
//!
//! cosmic-text 0.12 (which itself uses rustybuzz, the pure-Rust
//! HarfBuzz port, for shape-word) does the actual shaping work.
//! This module's job is to project the per-line glyph data
//! cosmic-text produces into a stable [`GlyphRun`] format the JS
//! shell can paint without re-shaping.
//!
//! Why a sub-module: keeps `lib.rs` focused on the public layout
//! surface and the page-pack engine; everything related to glyph
//! extraction lives here. The cache (`CachedShape`) carries glyph
//! data alongside line geometry so a cache hit returns both
//! without re-shaping.
//!
//! Per-glyph fields are minimal for Phase 4.2: glyph id (font-
//! specific), cluster range (for hit-testing back to source
//! codepoints), 2D position, and width. Color, BiDi level, and
//! per-glyph font fallback land in later phases — multi-font
//! fallback is not needed yet (single bundled DejaVu Sans face).

/// One shaped glyph in line-local coordinates. Coordinates are
/// pixels relative to the [`GlyphRun`]'s origin (the line's
/// top-left); add the parent block's `origin_x_px` / `origin_y_px`
/// to reach page-local coordinates, then add the page origin to
/// reach viewport-local.
///
/// `cluster_start` / `cluster_end` are codepoint offsets into the
/// source block text — the same offsets `Doc::block(idx).text`
/// indexes. A 1-glyph-per-codepoint shape has
/// `cluster_end == cluster_start + len_utf8(char)`. A ligature
/// (eg. `fi` → single glyph) keeps a cluster range that spans the
/// composed codepoints; a decomposition (one combining mark
/// becoming two glyphs) emits two glyphs that share a cluster.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionedGlyph {
    /// Font-specific glyph index. Always non-zero for shaped text;
    /// zero is the `.notdef` slot, which cosmic-text only emits
    /// when the font has no coverage for the codepoint.
    pub glyph_id: u16,
    /// Start codepoint offset of the source cluster in the block's
    /// text.
    pub cluster_start: u32,
    /// End codepoint offset of the source cluster in the block's
    /// text (exclusive).
    pub cluster_end: u32,
    /// X offset in line-local coordinates (left-to-right pen).
    pub x_px: f32,
    /// Y offset in line-local coordinates.
    pub y_px: f32,
    /// Width contribution toward the next glyph.
    pub width_px: f32,
}

/// One run of shaped glyphs sharing a single block + line + font
/// size. Phase 4.2 emits exactly one [`GlyphRun`] per shaped line:
/// rich-text mark boundaries (which would split a line into
/// multiple runs by font / weight / color) land in a later phase
/// once the doc-model carries per-character marks for layout.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphRun {
    /// Block index in the doc — matches
    /// [`crate::BlockBox::block_index`].
    pub block_index: usize,
    /// 0-based line index within the block. Matches the index into
    /// [`crate::BlockBox::lines`].
    pub line_index: usize,
    /// Font size used for this run (pixels).
    pub font_size_px: f32,
    /// Baseline offset in line-local coordinates — where the
    /// renderer should place the glyph origins.
    pub baseline_y_px: f32,
    /// Glyphs in left-to-right paint order.
    pub glyphs: Vec<PositionedGlyph>,
}
