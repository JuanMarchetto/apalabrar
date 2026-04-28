#![deny(unsafe_code)]
#![doc = "DOCX format I/O. Two surfaces: a flat-text projection (parse_text / serialize_text, used by editor-core for v0 plumbing) and a structural read / write returning DocModel (the OOXML round-trip moat)."]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::ops::Range;

use docx_rs::{DocumentChild, Docx, Paragraph, ParagraphChild, Run, RunChild, read_docx};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use thiserror::Error;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum Error {
    #[error("input bytes are empty")]
    EmptyInput,
    #[error("input bytes are not a valid OOXML/.docx file: {0}")]
    InvalidOoxml(String),
    #[error("OOXML serialization failed: {0}")]
    SerializeFailed(String),
    #[error("zip read/write failed: {0}")]
    InvalidZip(String),
    #[error("XML parsing failed: {0}")]
    XmlError(String),
    #[error("paragraph index {index} out of bounds (len {len})")]
    InvalidParagraphIndex { index: usize, len: usize },
}

// ---------------------------------------------------------------------------
// Flat-text surface (Gate 2)
// ---------------------------------------------------------------------------

/// Parse a `.docx` file into plain UTF-8 text. Each top-level paragraph
/// becomes one logical line; multiple paragraphs are joined by `'\n'`.
/// Empty paragraphs round-trip as empty lines.
pub fn parse_text(bytes: &[u8]) -> Result<String, Error> {
    if bytes.is_empty() {
        return Err(Error::EmptyInput);
    }
    let docx = read_docx(bytes).map_err(|e| Error::InvalidOoxml(e.to_string()))?;
    let mut paragraphs: Vec<String> = Vec::new();
    for child in &docx.document.children {
        if let DocumentChild::Paragraph(p) = child {
            let mut line = String::new();
            for pchild in &p.children {
                if let ParagraphChild::Run(r) = pchild {
                    for rchild in &r.children {
                        if let RunChild::Text(t) = rchild {
                            line.push_str(&t.text);
                        }
                    }
                }
            }
            paragraphs.push(line);
        }
    }
    Ok(paragraphs.join("\n"))
}

/// Serialize a plain UTF-8 string into a fresh `.docx`. The string is
/// split on `'\n'`; each piece becomes one paragraph (empty pieces
/// become empty paragraphs). Round-trip with `parse_text` is a fixed
/// point.
pub fn serialize_text(text: &str) -> Result<Vec<u8>, Error> {
    let mut docx = Docx::new();
    for piece in text.split('\n') {
        let p = if piece.is_empty() {
            Paragraph::new()
        } else {
            Paragraph::new().add_run(Run::new().add_text(piece))
        };
        docx = docx.add_paragraph(p);
    }
    let mut buf: Vec<u8> = Vec::new();
    docx.build()
        .pack(Cursor::new(&mut buf))
        .map_err(|e| Error::SerializeFailed(e.to_string()))?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Structural surface (Gate 4 — OOXML round-trip moat)
// ---------------------------------------------------------------------------

/// Stable name of the main document part inside a `.docx` zip.
const DOCUMENT_PART: &str = "word/document.xml";

/// Opaque structural model of a `.docx` file. See module docs above.
pub struct DocModel {
    /// Every zip entry, indexed by its archive name. Stored verbatim so
    /// untouched parts round-trip byte-for-byte (the lossless contract).
    parts: BTreeMap<String, Vec<u8>>,
    /// Indexed parse of `word/document.xml` for paragraph-level edits.
    document: DocumentTree,
}

struct DocumentTree {
    /// Original raw bytes of `word/document.xml`. The splice source.
    raw: Vec<u8>,
    /// One entry per top-level `<w:p>` in document order. Each carries
    /// its byte range in `raw` plus the plain-text projection. When
    /// `set_paragraph_text` is called we set `new_text` and flip
    /// `dirty_count`; `write` then splices fresh XML for those entries
    /// and re-emits everything else verbatim.
    paragraphs: Vec<ParagraphInfo>,
    /// Set to `true` as soon as any paragraph is mutated.
    dirty: bool,
}

struct ParagraphInfo {
    /// Byte range in `DocumentTree::raw` covering `<w:p>...</w:p>` (or
    /// a self-closed `<w:p/>`) inclusive.
    range: Range<usize>,
    /// Concatenated `<w:t>` text content from every `<w:r>` child of
    /// this paragraph.
    text: String,
    /// `Some` after `set_paragraph_text`; `None` for unmodified paragraphs.
    new_text: Option<String>,
}

impl DocModel {
    pub fn paragraph_count(&self) -> usize {
        self.document.paragraphs.len()
    }

    pub fn paragraph_text(&self, index: usize) -> Option<&str> {
        let p = self.document.paragraphs.get(index)?;
        Some(p.new_text.as_deref().unwrap_or(&p.text))
    }

    pub fn set_paragraph_text(&mut self, index: usize, text: &str) -> Result<(), Error> {
        let len = self.document.paragraphs.len();
        let Some(p) = self.document.paragraphs.get_mut(index) else {
            return Err(Error::InvalidParagraphIndex { index, len });
        };
        p.new_text = Some(text.to_string());
        self.document.dirty = true;
        Ok(())
    }
}

/// Parse `.docx` bytes into a structural `DocModel`. Errors on empty
/// input, malformed zips, missing `word/document.xml`, or malformed XML
/// inside the main document part.
pub fn read(bytes: &[u8]) -> Result<DocModel, Error> {
    let (model, _shadow) = read_preserve(bytes)?;
    Ok(model)
}

// ---------------------------------------------------------------------------
// Shadow-preserve surface
// ---------------------------------------------------------------------------
//
// `read_preserve` reads a `.docx` into a `DocModel` AND a `ShadowXml`
// snapshot of every zip part. The structural surface only interprets
// `word/document.xml` for paragraph-level edits; every other part is
// unrecognized and survives byte-for-byte through the shadow.

/// Verbatim shadow of every zip part in the original `.docx`, captured
/// at read time. Holding a `ShadowXml` alongside a (possibly mutated)
/// `DocModel` is the lossless escape hatch: callers always have access
/// to the pristine original bytes.
pub struct ShadowXml {
    parts: BTreeMap<String, Vec<u8>>,
}

impl ShadowXml {
    /// Archive names of every zip part captured at read time, in
    /// `BTreeMap` order (lexicographic by name).
    pub fn part_names(&self) -> Vec<&str> {
        self.parts.keys().map(String::as_str).collect()
    }

    /// Verbatim bytes of a single zip part by archive name. `None` if
    /// the original `.docx` did not contain a part with that name.
    pub fn part(&self, name: &str) -> Option<&[u8]> {
        self.parts.get(name).map(Vec::as_slice)
    }

    /// Verbatim bytes of `word/document.xml`. Always present after a
    /// successful `read_preserve`.
    pub fn document_xml(&self) -> &[u8] {
        self.parts
            .get(DOCUMENT_PART)
            .map(Vec::as_slice)
            .expect("read_preserve guarantees word/document.xml")
    }

    /// Total number of zip parts captured. A successful `read_preserve`
    /// always produces a non-empty shadow (we error out earlier if
    /// `word/document.xml` is missing), so the conventional
    /// `is_empty()` companion would be a fallback for an impossible
    /// state and a permanent mutation-survivor.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.parts.len()
    }
}

/// Parse `.docx` bytes into a structural `DocModel` AND a verbatim
/// `ShadowXml` snapshot of the input. Errors on empty input, malformed
/// zips, missing `word/document.xml`, or malformed XML inside the main
/// document part — same error variants as `read`.
pub fn read_preserve(bytes: &[u8]) -> Result<(DocModel, ShadowXml), Error> {
    if bytes.is_empty() {
        return Err(Error::EmptyInput);
    }
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::InvalidZip(e.to_string()))?;
    let mut parts = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| Error::InvalidZip(e.to_string()))?;
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| Error::InvalidZip(e.to_string()))?;
        parts.insert(name, buf);
    }
    let raw = parts
        .get(DOCUMENT_PART)
        .cloned()
        .ok_or_else(|| Error::InvalidOoxml(format!("missing {DOCUMENT_PART}")))?;
    let paragraphs = index_paragraphs(&raw)?;
    let shadow = ShadowXml {
        parts: parts.clone(),
    };
    let model = DocModel {
        parts,
        document: DocumentTree {
            raw,
            paragraphs,
            dirty: false,
        },
    };
    Ok((model, shadow))
}

/// Serialize a `DocModel` back to `.docx` bytes. Unmodified parts are
/// re-emitted verbatim; dirty paragraphs are spliced into a fresh
/// `word/document.xml` while the unchanged byte ranges around them stay
/// exactly as they were on disk.
pub fn write(doc: &DocModel) -> Result<Vec<u8>, Error> {
    let document_xml = if doc.document.dirty {
        rebuild_document_xml(&doc.document)
    } else {
        doc.document.raw.clone()
    };
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in &doc.parts {
            zip.start_file(name, options)
                .map_err(|e| Error::InvalidZip(e.to_string()))?;
            let bytes: &[u8] = if name == DOCUMENT_PART {
                &document_xml
            } else {
                content
            };
            zip.write_all(bytes)
                .map_err(|e| Error::InvalidZip(e.to_string()))?;
        }
        zip.finish().map_err(|e| Error::InvalidZip(e.to_string()))?;
    }
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Internals: paragraph indexing + splice rebuild
// ---------------------------------------------------------------------------

/// Walk `word/document.xml` once and record every top-level `<w:p>`'s
/// byte range and concatenated `<w:t>` text. Stays single-pass so the
/// cost is linear in the document size.
fn index_paragraphs(xml: &[u8]) -> Result<Vec<ParagraphInfo>, Error> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut paragraphs = Vec::new();

    let mut p_start: Option<usize> = None;
    let mut text_acc = String::new();
    let mut in_t = false;

    loop {
        // quick-xml 0.36 returns positions as u64; we cast to usize because
        // the underlying byte slice is indexed with usize. The cast is sound
        // for any input that fits in memory on the host.
        let pos_before = reader.buffer_position() as usize;
        let evt = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error::XmlError(e.to_string()))?;
        match evt {
            Event::Start(e) => {
                let name = e.name();
                let bytes = name.as_ref();
                if bytes == b"w:p" && p_start.is_none() {
                    p_start = Some(pos_before);
                } else if bytes == b"w:t" && p_start.is_some() {
                    in_t = true;
                }
            }
            Event::End(e) => {
                let name = e.name();
                let bytes = name.as_ref();
                if bytes == b"w:t" {
                    in_t = false;
                } else if bytes == b"w:p" {
                    if let Some(start) = p_start.take() {
                        let end = reader.buffer_position() as usize;
                        paragraphs.push(ParagraphInfo {
                            range: start..end,
                            text: std::mem::take(&mut text_acc),
                            new_text: None,
                        });
                    }
                }
            }
            Event::Empty(e) => {
                if e.name().as_ref() == b"w:p" && p_start.is_none() {
                    let end = reader.buffer_position() as usize;
                    paragraphs.push(ParagraphInfo {
                        range: pos_before..end,
                        text: String::new(),
                        new_text: None,
                    });
                }
            }
            Event::Text(t) => {
                if in_t {
                    let s = t.unescape().map_err(|e| Error::XmlError(e.to_string()))?;
                    text_acc.push_str(&s);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(paragraphs)
}

/// Splice dirty paragraphs into a fresh document.xml, copying every
/// other byte range from the original raw stream.
fn rebuild_document_xml(doc: &DocumentTree) -> Vec<u8> {
    let mut out = Vec::with_capacity(doc.raw.len() + 256);
    let mut cursor = 0;
    for p in &doc.paragraphs {
        out.extend_from_slice(&doc.raw[cursor..p.range.start]);
        if let Some(new_text) = &p.new_text {
            out.extend_from_slice(format_paragraph(new_text).as_bytes());
        } else {
            out.extend_from_slice(&doc.raw[p.range.clone()]);
        }
        cursor = p.range.end;
    }
    out.extend_from_slice(&doc.raw[cursor..]);
    out
}

/// Produce a minimal v0 `<w:p>` XML for a freshly-set paragraph. This is
/// intentionally property-free (no rPr / pPr) — Phase 1 will preserve
/// inherited formatting by parsing the original paragraph's properties
/// and reusing them. For Gate 4 the test only checks that OTHER parts
/// are byte-equivalent, so this minimal form is sufficient.
fn format_paragraph(text: &str) -> String {
    format!(
        "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_escape(text)
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
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
