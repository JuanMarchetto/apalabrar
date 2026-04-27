//! RED-phase tests for apalabrar-format-docx. The public surface is a
//! thin pair: `parse_text` (DOCX bytes -> plain text with `\n` paragraph
//! separators) and `serialize_text` (plain text -> fresh DOCX bytes).
//! These integration tests assert the round-trip contract and the
//! error variants. Lossless OOXML preservation is Phase 1.
//!
//! Inline audit (no separate audit commit; surface is small):
//! - 0 tautologies; every assert checks observable behaviour.
//! - 0 mocks of self.
//! - Edge cases covered: empty bytes, garbage bytes, empty paragraph,
//!   multi-paragraph, multibyte LATAM, lossless round-trip via prop.
//! - 1 property test on serialize<->parse round-trip; non-trivial.
//! - No coverage gap blocking GREEN.

use apalabrar_format_docx::{Error, parse_text, serialize_text};
use proptest::prelude::*;

// ---------- Happy path ----------

#[test]
fn serialize_then_parse_round_trips_a_single_line() {
    let bytes = serialize_text("hello world").expect("serialize");
    assert_eq!(parse_text(&bytes).unwrap(), "hello world");
}

#[test]
fn serialize_then_parse_round_trips_empty_text() {
    let bytes = serialize_text("").expect("serialize");
    assert_eq!(parse_text(&bytes).unwrap(), "");
}

#[test]
fn serialize_then_parse_preserves_paragraph_boundaries() {
    let bytes = serialize_text("alpha\nbeta\ngamma").expect("serialize");
    assert_eq!(parse_text(&bytes).unwrap(), "alpha\nbeta\ngamma");
}

#[test]
fn serialize_then_parse_preserves_empty_paragraph_in_middle() {
    // Empty middle paragraph: alpha, "", beta. Preserved as alpha\n\nbeta.
    let bytes = serialize_text("alpha\n\nbeta").expect("serialize");
    assert_eq!(parse_text(&bytes).unwrap(), "alpha\n\nbeta");
}

// ---------- Multibyte / LATAM ----------

#[test]
fn serialize_then_parse_preserves_latam_accents() {
    let original = "Año mañana es el día 1°";
    let bytes = serialize_text(original).expect("serialize");
    assert_eq!(parse_text(&bytes).unwrap(), original);
}

// ---------- Error cases ----------

#[test]
fn parse_text_rejects_empty_bytes_with_empty_input_error() {
    let result = parse_text(&[]);
    assert!(
        matches!(result, Err(Error::EmptyInput)),
        "expected EmptyInput, got {:?}",
        result.err()
    );
}

#[test]
fn parse_text_rejects_garbage_bytes_with_invalid_ooxml_error() {
    let garbage = b"this is definitely not a docx file";
    let result = parse_text(garbage);
    assert!(
        matches!(result, Err(Error::InvalidOoxml(_))),
        "expected InvalidOoxml, got {:?}",
        result.err()
    );
}

#[test]
fn parse_text_rejects_zip_without_docx_parts_with_invalid_ooxml_error() {
    // A minimal but invalid zip header (PK\x03\x04...) with no docx
    // parts inside should fail OOXML parsing.
    let mut zip = vec![0x50u8, 0x4B, 0x03, 0x04];
    zip.extend(std::iter::repeat_n(0u8, 20));
    let result = parse_text(&zip);
    assert!(
        matches!(result, Err(Error::InvalidOoxml(_))),
        "expected InvalidOoxml, got {:?}",
        result.err()
    );
}

// ---------- Property tests ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// `parse_text(serialize_text(x)) == x` for any reasonable UTF-8
    /// string. Regex restricts to printable ASCII + LATAM accents +
    /// `\n` paragraph separator so the property exercises the
    /// paragraph-splitting branch under multibyte input.
    #[test]
    fn prop_serialize_then_parse_round_trip_is_identity(
        text in "[a-zA-Z0-9áéíóúüñçÁÉÍÓÚÜÑÇ \n]{0,30}",
    ) {
        let bytes = serialize_text(&text).unwrap();
        prop_assert_eq!(parse_text(&bytes).unwrap(), text);
    }
}
