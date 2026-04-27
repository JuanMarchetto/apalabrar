#![deny(unsafe_code)]
#![doc = "DOCX format I/O. v0 surface: parse a .docx into plain UTF-8 text (paragraphs joined by `\\n`) and serialize a plain UTF-8 string back into a fresh single-section .docx. Lossless OOXML preservation arrives in Phase 1."]

use std::io::Cursor;

use docx_rs::{DocumentChild, Docx, Paragraph, ParagraphChild, Run, RunChild, read_docx};
use thiserror::Error;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum Error {
    #[error("input bytes are empty")]
    EmptyInput,
    #[error("input bytes are not a valid OOXML/.docx file: {0}")]
    InvalidOoxml(String),
    #[error("OOXML serialization failed: {0}")]
    SerializeFailed(String),
}

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
        // Tables and other DocumentChild variants are ignored at v0;
        // Phase 1 will preserve them via the lossless OOXML pattern.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
