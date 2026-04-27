//! Shared test fixtures for the editor-core integration tests.
//!
//! `build_minimal_docx` produces a valid `.docx` zip in-memory containing a
//! single paragraph with the given text. The empty-text case yields a doc
//! with one empty paragraph (the smallest legal OOXML body).

use docx_rs::{Docx, Paragraph, Run};
use std::io::Cursor;

pub fn build_minimal_docx(text: &str) -> Vec<u8> {
    let docx = if text.is_empty() {
        Docx::new().add_paragraph(Paragraph::new())
    } else {
        Docx::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)))
    };
    let mut buf: Vec<u8> = Vec::new();
    docx.build()
        .pack(Cursor::new(&mut buf))
        .expect("docx-rs should pack a minimal doc to a Vec<u8>");
    buf
}
