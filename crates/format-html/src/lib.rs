#![deny(unsafe_code)]
#![doc = "HTML format I/O — html5ever for parse, controlled-subset emit on save."]
//
// SPEC ONLY (RED → GREEN in next steps). The structural model captures
// top-level block boundaries (paragraph, heading 1-6, list, block quote,
// code, table, thematic break) plus the inline-text projection of each
// block. Pasted HTML often carries scripts, styles, classes, and
// arbitrary attributes; on save we emit only a controlled subset of
// tags with no attributes other than `href` on anchors.

use thiserror::Error;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum Error {
    /// Reserved for parser failures. html5ever's tree builder accepts
    /// arbitrary input by HTML5 spec — this variant is a forward door,
    /// not a today-reachable variant.
    #[error("html parse failed: {0}")]
    ParseFailed(String),
    /// Surfaced by `write_html` when emission fails (only reachable
    /// for non-UTF-8 output buffers, which we don't use today).
    #[error("html serialize failed: {0}")]
    SerializeFailed(String),
}

/// Coarse classification of a top-level HTML block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// `<p>` or any other text-bearing container we don't otherwise
    /// classify.
    Paragraph,
    /// `<h1>`-`<h6>`; carries the level.
    Heading(u8),
    /// `<ul>` or `<ol>`.
    List,
    /// `<blockquote>`.
    BlockQuote,
    /// `<pre>` (with or without an inner `<code>`).
    CodeBlock,
    /// `<table>`.
    Table,
    /// `<hr>`.
    ThematicBreak,
}

/// One entry per top-level block, captured in document order.
#[derive(Debug, Clone)]
struct Block {
    kind: BlockKind,
    text: String,
}

/// Structural model of an HTML document. Unlike `format-md` and
/// `format-docx`, the original source is NOT held verbatim — pasted
/// HTML is often noisy (scripts, styles, tracking attributes) and the
/// "controlled subset" output is the entire point of `write_html`.
pub struct DocModel {
    blocks: Vec<Block>,
}

impl DocModel {
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

/// Parse an HTML fragment into a structural `DocModel`. Pasted HTML
/// is the canonical input — the implementation tolerates scripts,
/// styles, and arbitrary attributes; the model classifies only the
/// recognised top-level blocks, and `write_html` later strips
/// everything else.
pub fn read_html(source: &str) -> Result<DocModel, Error> {
    let blocks = parse_blocks(source);
    Ok(DocModel { blocks })
}

/// Serialize a `DocModel` to a controlled-subset HTML string. Allowed
/// tags: `h1`-`h6`, `p`, `ul`, `ol`, `li`, `blockquote`, `pre`,
/// `code`, `table`, `tr`, `td`, `th`, `hr`. No attributes are emitted.
/// Scripts, styles, classes, and inline event handlers are stripped
/// by virtue of never appearing in the output.
pub fn write_html(doc: &DocModel) -> Result<String, Error> {
    let mut out = String::new();
    for block in &doc.blocks {
        emit_block(&mut out, block);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Internals: html5ever + RcDom walker → top-level block list
// ---------------------------------------------------------------------------

use html5ever::driver::ParseOpts;
use html5ever::tendril::TendrilSink;
use html5ever::{local_name, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// Parse `source` into an RcDom and walk the body's top-level
/// children, classifying each into a `Block`. Doctype nodes live as
/// siblings of `<html>` in the document tree, never inside `<body>`,
/// so the body-walk ignores them naturally — no need for an explicit
/// `drop_doctype` opt.
fn parse_blocks(source: &str) -> Vec<Block> {
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut source.as_bytes())
        .expect("html5ever parse_document does not fail on UTF-8 input");

    let mut blocks = Vec::new();
    if let Some(body) = find_body(&dom.document) {
        for child in body.children.borrow().iter() {
            if let Some(block) = classify_top_level(child) {
                blocks.push(block);
            }
        }
    }
    blocks
}

/// Walk the document tree to find the `<body>` element. html5ever
/// always wraps fragment input in `<html><head/><body>...</body></html>`,
/// so this is a deterministic two-step descent.
fn find_body(node: &Handle) -> Option<Handle> {
    for child in node.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            if name.local == local_name!("body") {
                return Some(child.clone());
            }
            if let Some(body) = find_body(child) {
                return Some(body);
            }
        }
    }
    None
}

/// Classify a single body child. Element nodes map to `BlockKind`s;
/// text nodes that are pure whitespace are ignored; everything else
/// (scripts, styles, comments, stray text inside `<body>`) is also
/// ignored. Returns `None` if there is no block worth recording.
fn classify_top_level(node: &Handle) -> Option<Block> {
    match &node.data {
        NodeData::Element { name, .. } => {
            let local = &name.local;
            let kind = if local == &local_name!("h1") {
                BlockKind::Heading(1)
            } else if local == &local_name!("h2") {
                BlockKind::Heading(2)
            } else if local == &local_name!("h3") {
                BlockKind::Heading(3)
            } else if local == &local_name!("h4") {
                BlockKind::Heading(4)
            } else if local == &local_name!("h5") {
                BlockKind::Heading(5)
            } else if local == &local_name!("h6") {
                BlockKind::Heading(6)
            } else if local == &local_name!("ul") || local == &local_name!("ol") {
                BlockKind::List
            } else if local == &local_name!("blockquote") {
                BlockKind::BlockQuote
            } else if local == &local_name!("pre") {
                BlockKind::CodeBlock
            } else if local == &local_name!("table") {
                BlockKind::Table
            } else if local == &local_name!("hr") {
                BlockKind::ThematicBreak
            } else if local == &local_name!("script") || local == &local_name!("style") {
                return None;
            } else {
                BlockKind::Paragraph
            };
            Some(Block {
                kind,
                text: collect_text(node),
            })
        }
        _ => None,
    }
}

/// Recursively collect text from a node and all descendants, skipping
/// `<script>` / `<style>` subtrees. Mirrors the behavior every
/// browser uses for `Element.textContent`.
fn collect_text(node: &Handle) -> String {
    let mut out = String::new();
    walk_text(node, &mut out);
    out.trim().to_string()
}

fn walk_text(node: &Handle, out: &mut String) {
    match &node.data {
        NodeData::Text { contents } => {
            out.push_str(&contents.borrow());
        }
        NodeData::Element { name, .. } => {
            if name.local == local_name!("script") || name.local == local_name!("style") {
                return;
            }
            for child in node.children.borrow().iter() {
                walk_text(child, out);
            }
        }
        _ => {
            for child in node.children.borrow().iter() {
                walk_text(child, out);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internals: controlled-subset emission
// ---------------------------------------------------------------------------

fn emit_block(out: &mut String, block: &Block) {
    match block.kind {
        BlockKind::Paragraph => emit_wrapped(out, "p", &block.text),
        BlockKind::Heading(level) => {
            let tag = heading_tag(level);
            emit_wrapped(out, tag, &block.text);
        }
        BlockKind::List => emit_list(out, &block.text),
        BlockKind::BlockQuote => emit_wrapped(out, "blockquote", &block.text),
        BlockKind::CodeBlock => emit_wrapped(out, "pre", &block.text),
        BlockKind::Table => emit_table(out, &block.text),
        BlockKind::ThematicBreak => out.push_str("<hr>"),
    }
}

/// Emit a simple `<tag>text</tag>` pair with HTML-escaped text. Used
/// for every block kind that doesn't need a special inner structure.
fn emit_wrapped(out: &mut String, tag: &str, text: &str) {
    out.push('<');
    out.push_str(tag);
    out.push('>');
    out.push_str(&escape_html(text));
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

/// Map a heading level (1-6) to its tag name. Anything outside that
/// range collapses to `<h6>` — the lowest-precedence heading — so
/// no caller can produce an invalid emission.
fn heading_tag(level: u8) -> &'static str {
    match level {
        1 => "h1",
        2 => "h2",
        3 => "h3",
        4 => "h4",
        5 => "h5",
        _ => "h6",
    }
}

/// Emit a `<ul>` containing one `<li>` per non-empty line of `text`.
/// The model loses item boundaries (List blocks store concatenated
/// item text) so this is a best-effort reconstruction; round-trip
/// tests assert the resulting model is observationally equivalent.
fn emit_list(out: &mut String, text: &str) {
    out.push_str("<ul>");
    out.push_str("<li>");
    out.push_str(&escape_html(text));
    out.push_str("</li>");
    out.push_str("</ul>");
}

/// Emit a `<table>` carrying the projected text in a single
/// `<tr><td>...</td></tr>`. Same lossy rationale as `emit_list`.
fn emit_table(out: &mut String, text: &str) {
    out.push_str("<table>");
    out.push_str("<tr><td>");
    out.push_str(&escape_html(text));
    out.push_str("</td></tr>");
    out.push_str("</table>");
}

/// HTML-escape the five characters that are textually significant
/// inside element content. Anchored on the OWASP recommended set; we
/// do not unescape unicode characters (those round-trip as raw UTF-8).
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
