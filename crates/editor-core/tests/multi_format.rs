//! Phase 5.6.2 — multi-format I/O.
//!
//! Validates `open_doc(bytes, format)` and `to_format(doc_id, format)`
//! across the five supported v0 formats. DOCX/MD/HTML are functional
//! today; RTF/ODT are stubbed in their respective crates and the
//! editor surface returns `Error::FormatNotSupported` so the UI can
//! surface a clear "v1" message. The MD/HTML paths use UTF-8
//! passthrough — the source string IS the editable text in v0 (rich
//! semantic round-trip arrives with phase 5.9 + the structural readers
//! already living in `format-md` / `format-html`).

use apalabrar_editor_core::{DocFormat, Error, doc_text, open_doc, parse_doc_format, to_format};

mod common;
use common::build_minimal_docx;

// ───────────────────────── parse_doc_format ─────────────────────────

#[test]
fn parse_doc_format_accepts_canonical_short_forms() {
    assert_eq!(parse_doc_format("docx").unwrap(), DocFormat::Docx);
    assert_eq!(parse_doc_format("md").unwrap(), DocFormat::Markdown);
    assert_eq!(parse_doc_format("html").unwrap(), DocFormat::Html);
    assert_eq!(parse_doc_format("rtf").unwrap(), DocFormat::Rtf);
    assert_eq!(parse_doc_format("odt").unwrap(), DocFormat::Odt);
}

#[test]
fn parse_doc_format_accepts_markdown_as_synonym_for_md() {
    assert_eq!(parse_doc_format("markdown").unwrap(), DocFormat::Markdown);
}

#[test]
fn parse_doc_format_rejects_unknown_with_unknownformat_carrying_the_input() {
    match parse_doc_format("pdf") {
        Err(Error::UnknownFormat { name }) => assert_eq!(name, "pdf"),
        other => panic!("expected UnknownFormat, got: {other:?}"),
    }
}

#[test]
fn parse_doc_format_is_case_sensitive() {
    // We deliberately don't lowercase — JS dispatch passes the canonical
    // short form, and accepting `"DOCX"` would only mask bugs in the
    // caller. Document the choice with a test.
    assert!(matches!(
        parse_doc_format("DOCX"),
        Err(Error::UnknownFormat { .. }),
    ));
}

// ───────────────────────── DOCX ─────────────────────────

#[test]
fn open_doc_docx_returns_text_from_a_minimal_docx() {
    let bytes = build_minimal_docx("hello world");
    let id = open_doc(&bytes, DocFormat::Docx).expect("minimal docx should parse");
    assert_eq!(doc_text(id).unwrap(), "hello world");
}

#[test]
fn to_format_docx_round_trips_text() {
    let bytes = build_minimal_docx("hello world");
    let id = open_doc(&bytes, DocFormat::Docx).unwrap();
    let serialized = to_format(id, DocFormat::Docx).expect("docx serialize");
    let id2 = open_doc(&serialized, DocFormat::Docx).unwrap();
    assert_eq!(doc_text(id2).unwrap(), "hello world");
}

#[test]
fn open_doc_docx_rejects_garbage_bytes() {
    let r = open_doc(b"not a real docx", DocFormat::Docx);
    assert!(matches!(r, Err(Error::InvalidInput { .. })));
}

// ───────────────────────── Markdown ─────────────────────────

#[test]
fn open_doc_markdown_decodes_utf8_source_as_text() {
    let id = open_doc(b"# Title\n\nbody", DocFormat::Markdown).unwrap();
    assert_eq!(doc_text(id).unwrap(), "# Title\n\nbody");
}

#[test]
fn open_doc_markdown_preserves_latam_accents() {
    let id = open_doc("¡Hola, mundó!".as_bytes(), DocFormat::Markdown).unwrap();
    assert_eq!(doc_text(id).unwrap(), "¡Hola, mundó!");
}

#[test]
fn open_doc_markdown_rejects_invalid_utf8() {
    let bytes: &[u8] = &[0xff, 0xfe, 0xfd];
    let r = open_doc(bytes, DocFormat::Markdown);
    assert!(
        matches!(r, Err(Error::InvalidInput { .. })),
        "got: {:?}",
        r.err(),
    );
}

#[test]
fn to_format_markdown_returns_utf8_bytes_of_doc_text() {
    let id = open_doc(b"# Hello", DocFormat::Markdown).unwrap();
    let out = to_format(id, DocFormat::Markdown).unwrap();
    assert_eq!(out, b"# Hello".to_vec());
}

// ───────────────────────── HTML ─────────────────────────

#[test]
fn open_doc_html_decodes_utf8_source_as_text() {
    let id = open_doc(b"<p>Hello</p>", DocFormat::Html).unwrap();
    assert_eq!(doc_text(id).unwrap(), "<p>Hello</p>");
}

#[test]
fn open_doc_html_rejects_invalid_utf8() {
    let bytes: &[u8] = &[0xc3, 0x28]; // invalid UTF-8 continuation
    let r = open_doc(bytes, DocFormat::Html);
    assert!(matches!(r, Err(Error::InvalidInput { .. })));
}

#[test]
fn to_format_html_returns_utf8_bytes_of_doc_text() {
    let id = open_doc(b"<h1>x</h1>", DocFormat::Html).unwrap();
    let out = to_format(id, DocFormat::Html).unwrap();
    assert_eq!(out, b"<h1>x</h1>".to_vec());
}

// ───────────────────────── RTF / ODT ─────────────────────────

#[test]
fn open_doc_rtf_returns_format_not_supported() {
    let r = open_doc(b"{\\rtf1}", DocFormat::Rtf);
    assert!(matches!(r, Err(Error::FormatNotSupported { .. })));
}

#[test]
fn open_doc_odt_returns_format_not_supported() {
    let r = open_doc(b"PK\x03\x04", DocFormat::Odt);
    assert!(matches!(r, Err(Error::FormatNotSupported { .. })));
}

#[test]
fn to_format_rtf_returns_format_not_supported() {
    let id = open_doc(b"hi", DocFormat::Markdown).unwrap();
    let r = to_format(id, DocFormat::Rtf);
    assert!(matches!(r, Err(Error::FormatNotSupported { .. })));
}

#[test]
fn to_format_odt_returns_format_not_supported() {
    let id = open_doc(b"hi", DocFormat::Markdown).unwrap();
    let r = to_format(id, DocFormat::Odt);
    assert!(matches!(r, Err(Error::FormatNotSupported { .. })));
}

#[test]
fn format_not_supported_error_carries_the_offending_format() {
    let r = open_doc(b"x", DocFormat::Rtf);
    match r {
        Err(Error::FormatNotSupported { format }) => assert_eq!(format, DocFormat::Rtf),
        other => panic!("expected FormatNotSupported, got: {other:?}"),
    }
}

// ───────────────────────── Empty input ─────────────────────────

#[test]
fn open_doc_empty_bytes_errors_for_every_format() {
    for fmt in [
        DocFormat::Docx,
        DocFormat::Markdown,
        DocFormat::Html,
        DocFormat::Rtf,
        DocFormat::Odt,
    ] {
        let r = open_doc(b"", fmt);
        assert!(
            matches!(r, Err(Error::EmptyInput)),
            "fmt={fmt:?}, got: {:?}",
            r.err(),
        );
    }
}

// ───────────────────────── Cross-format conversion ─────────────────────────

#[test]
fn cross_format_save_md_then_open_as_md_preserves_text() {
    let id = open_doc(b"# Heading\n\ntext", DocFormat::Markdown).unwrap();
    let bytes = to_format(id, DocFormat::Markdown).unwrap();
    let id2 = open_doc(&bytes, DocFormat::Markdown).unwrap();
    assert_eq!(doc_text(id).unwrap(), doc_text(id2).unwrap());
}

#[test]
fn opening_a_docx_then_saving_as_markdown_preserves_text() {
    let docx = build_minimal_docx("plain text");
    let id = open_doc(&docx, DocFormat::Docx).unwrap();
    let md_bytes = to_format(id, DocFormat::Markdown).unwrap();
    assert_eq!(md_bytes, b"plain text".to_vec());
}
