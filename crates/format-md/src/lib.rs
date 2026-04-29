#![deny(unsafe_code)]
#![doc = "Markdown format I/O — CommonMark + GFM tables/footnotes/tasklists/strikethrough."]
//
// SPEC ONLY (RED → GREEN in next steps). The structural model captures
// top-level block boundaries (paragraph, heading 1-6, list, block quote,
// fenced code, table, thematic break, footnote definition) plus the
// inline-text projection of each block. The original source is held
// verbatim so `write_md` is lossless by default — caller-side mutation
// hooks can be layered on later.

use thiserror::Error;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum Error {
    /// Reserved for parser failures. pulldown-cmark currently never
    /// errors on UTF-8 input so this is a forward-compatibility door,
    /// not a today-reachable variant.
    #[error("markdown parse failed: {0}")]
    ParseFailed(String),
}

/// Coarse classification of a top-level Markdown block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Plain paragraph or content block we don't otherwise classify.
    Paragraph,
    /// ATX or Setext heading; carries the level (1-6).
    Heading(u8),
    /// Ordered or unordered list (one entry per top-level list, not per item).
    List,
    /// `> quoted` block.
    BlockQuote,
    /// Fenced or indented code block.
    CodeBlock,
    /// GFM pipe table.
    Table,
    /// `---` / `***` thematic break.
    ThematicBreak,
    /// `[^id]: …` footnote definition (GFM).
    FootnoteDefinition,
}

/// One entry per top-level block, captured in document order.
#[derive(Debug, Clone)]
struct Block {
    kind: BlockKind,
    text: String,
}

/// Structural model of a Markdown document. Holds the original source
/// verbatim so `write_md` can round-trip byte-for-byte without a
/// re-emit pass; the indexed block list is the editable surface.
pub struct DocModel {
    /// Original source captured at `read_md` time. The lossless
    /// guarantee for unmodified docs.
    source: String,
    /// Top-level blocks in document order.
    blocks: Vec<Block>,
}

impl DocModel {
    /// Original Markdown source captured at `read_md` time.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Number of top-level blocks.
    pub fn paragraph_count(&self) -> usize {
        self.blocks.len()
    }

    /// Inline-text projection of block `index`. `None` if `index >=
    /// paragraph_count`.
    pub fn paragraph_text(&self, index: usize) -> Option<&str> {
        self.blocks.get(index).map(|b| b.text.as_str())
    }

    /// Classification of block `index`. `None` if `index >=
    /// paragraph_count`.
    pub fn block_kind(&self, index: usize) -> Option<BlockKind> {
        self.blocks.get(index).map(|b| b.kind)
    }
}

/// Parse a Markdown string into a structural `DocModel`. CommonMark
/// + GFM (tables, footnotes, strikethrough, task lists) are enabled.
pub fn read_md(source: &str) -> Result<DocModel, Error> {
    let blocks = parse_blocks(source);
    Ok(DocModel {
        source: source.to_string(),
        blocks,
    })
}

/// Serialize a `DocModel` back to Markdown. For an unmodified
/// `DocModel` the output is byte-equivalent to the input that
/// produced it (the lossless contract via the verbatim source).
pub fn write_md(doc: &DocModel) -> Result<String, Error> {
    Ok(doc.source.clone())
}

// ---------------------------------------------------------------------------
// Internals: pulldown-cmark event walker → top-level block list
// ---------------------------------------------------------------------------

use pulldown_cmark::{Event, Options, Parser, Tag};

/// Walk the event stream once, emitting one `Block` per top-level
/// container. Inline-text events are concatenated verbatim because
/// pulldown-cmark already emits whitespace inside `Text` runs; only
/// `SoftBreak` / `HardBreak` need an explicit space injection so
/// multi-line paragraphs don't fuse word boundaries. The `depth`
/// counter is balanced by pulldown-cmark's own Start/End invariant,
/// so neither saturating arithmetic nor `if depth > 0` text-event
/// guards are needed — those would be defensive code for states
/// pulldown-cmark cannot produce.
fn parse_blocks(source: &str) -> Vec<Block> {
    let parser = Parser::new_ext(source, gfm_options());
    let mut blocks = Vec::new();
    let mut depth: usize = 0;
    let mut current_kind: Option<BlockKind> = None;
    let mut current_text = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    current_kind = Some(top_level_block_kind(&tag));
                    current_text.clear();
                }
                depth += 1;
            }
            Event::End(_end) => {
                depth -= 1;
                if depth == 0 {
                    if let Some(kind) = current_kind.take() {
                        blocks.push(Block {
                            kind,
                            text: std::mem::take(&mut current_text),
                        });
                    }
                }
            }
            Event::Text(s) | Event::Code(s) | Event::Html(s) | Event::InlineHtml(s) => {
                current_text.push_str(&s);
            }
            Event::SoftBreak | Event::HardBreak => current_text.push(' '),
            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
            Event::Rule => {
                // Thematic break is a one-shot top-level block; it has
                // no Start/End wrapping events.
                blocks.push(Block {
                    kind: BlockKind::ThematicBreak,
                    text: String::new(),
                });
            }
        }
    }
    blocks
}

/// Map a pulldown-cmark `Tag` (only inspected when it opens at top
/// level) to our coarse `BlockKind`. Anything not explicitly matched
/// falls back to `Paragraph` — a deliberate choice so a parser
/// extension that introduces a new top-level tag doesn't crash the
/// model, just lands in the catch-all bucket.
fn top_level_block_kind(tag: &Tag<'_>) -> BlockKind {
    match tag {
        Tag::Heading { level, .. } => BlockKind::Heading(heading_level_to_u8(*level)),
        Tag::List(_) => BlockKind::List,
        Tag::BlockQuote(_) => BlockKind::BlockQuote,
        Tag::CodeBlock(_) => BlockKind::CodeBlock,
        Tag::Table(_) => BlockKind::Table,
        Tag::FootnoteDefinition(_) => BlockKind::FootnoteDefinition,
        _ => BlockKind::Paragraph,
    }
}

/// pulldown-cmark exposes `HeadingLevel` as an enum H1..H6; flatten to
/// 1..=6 so callers don't need to depend on pulldown-cmark.
fn heading_level_to_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    use pulldown_cmark::HeadingLevel::*;
    match level {
        H1 => 1,
        H2 => 2,
        H3 => 3,
        H4 => 4,
        H5 => 5,
        H6 => 6,
    }
}

/// Enable every GFM extension this layer claims to support: tables,
/// footnotes, strikethrough, task lists. The other pulldown-cmark
/// extensions (math, smart punctuation, definition lists, sub/super
/// script, etc.) stay off so the parser stays a pure CommonMark+GFM
/// reader for now.
fn gfm_options() -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
