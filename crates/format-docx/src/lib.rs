#![deny(unsafe_code)]
#![doc = "DOCX format I/O. v0 surface: parse a .docx into plain UTF-8 text (paragraphs joined by `\\n`) and serialize a plain UTF-8 string back into a fresh single-section .docx. Lossless OOXML preservation arrives in Phase 1."]

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
/// is preserved as one logical line; multiple paragraphs are joined by
/// `'\n'`. Empty paragraphs round-trip as empty lines.
///
/// Errors:
/// - `EmptyInput` for `&[]`.
/// - `InvalidOoxml` if the bytes are not a parseable `.docx` zip.
pub fn parse_text(_bytes: &[u8]) -> Result<String, Error> {
    todo!("Step 4 GREEN: read_docx + walk Document/Paragraph/Run/Text")
}

/// Serialize a plain UTF-8 string into a fresh `.docx`. The string is
/// split on `'\n'`; each piece becomes one paragraph (empty pieces
/// become empty paragraphs). Round-trip with `parse_text` must be a
/// fixed point: `parse_text(serialize_text(x)) == x` for any UTF-8
/// string `x`.
pub fn serialize_text(_text: &str) -> Result<Vec<u8>, Error> {
    todo!(
        "Step 4 GREEN: build Docx with one Paragraph per `\\n`-separated piece, .pack into Vec<u8>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
