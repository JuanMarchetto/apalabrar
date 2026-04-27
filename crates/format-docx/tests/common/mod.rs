//! Shared test helpers for the OOXML round-trip test suite.
//!
//! Two pillars:
//!
//! 1. `unzip_parts(bytes)` — read a `.docx` zip into a `BTreeMap<part_name,
//!    Vec<u8>>` so tests can compare per-part rather than dealing with
//!    zip framing differences (compression level, file ordering,
//!    timestamps).
//!
//! 2. `normalize_xml(bytes)` — re-emit OOXML in a canonical form: sorted
//!    attributes, no whitespace-only text between elements, trimmed
//!    surrounding whitespace inside text. Comparison after normalization
//!    is the gate-4 standard for "byte-equivalent XML".
//!
//! These helpers are real implementation; the SPEC stubs in `lib.rs` stay
//! `todo!()` so the round-trip tests fail at the right place during RED.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use zip::ZipArchive;

/// Read every entry of a `.docx` zip into a `BTreeMap<name, bytes>`.
/// Panics on malformed zip — callers pass valid `.docx` bytes only.
pub fn unzip_parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("zip parse");
    let mut parts = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("zip entry");
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).expect("zip entry read");
        parts.insert(name, buf);
    }
    parts
}

/// Canonicalise OOXML/XML bytes:
/// - whitespace-only text events are dropped (they are presentational);
/// - text content keeps its internal whitespace (xml:space="preserve" is
///   how OOXML signals significant trailing space, so we never trim);
/// - attributes inside Start / Empty events are sorted by key.
///
/// The canonical form is re-emitted via `quick_xml::writer::Writer` with
/// no extra indentation so two normalised buffers can be compared with
/// plain `assert_eq!`.
pub fn normalize_xml(input: &[u8]) -> Vec<u8> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => writer
                .write_event(Event::Start(canonical_start(&e)))
                .expect("xml writer"),
            Ok(Event::Empty(e)) => writer
                .write_event(Event::Empty(canonical_start(&e)))
                .expect("xml writer"),
            Ok(Event::Text(e)) => {
                let raw = e.as_ref();
                let only_ws = !raw.is_empty() && raw.iter().all(|b| b.is_ascii_whitespace());
                if !only_ws {
                    writer.write_event(Event::Text(e)).expect("xml writer");
                }
            }
            Ok(Event::Eof) => break,
            Ok(other) => writer.write_event(other).expect("xml writer"),
            Err(err) => panic!("normalize_xml: parse error: {err}"),
        }
        buf.clear();
    }
    writer.into_inner().into_inner()
}

fn canonical_start(start: &BytesStart<'_>) -> BytesStart<'static> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut attrs: Vec<(Vec<u8>, Vec<u8>)> = start
        .attributes()
        .filter_map(|a| a.ok())
        .map(|a| (a.key.as_ref().to_vec(), a.value.into_owned()))
        .collect();
    attrs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = BytesStart::new(name);
    for (k, v) in attrs {
        out.push_attribute((k.as_slice(), v.as_slice()));
    }
    out
}

/// Assert two `.docx` byte buffers contain the same set of zip parts and
/// every XML / .rels part is byte-equivalent after `normalize_xml`. Binary
/// parts (images, etc.) must match exactly.
pub fn assert_zip_xml_equivalent(label: &str, input: &[u8], output: &[u8]) {
    let a = unzip_parts(input);
    let b = unzip_parts(output);
    let a_names: Vec<_> = a.keys().cloned().collect();
    let b_names: Vec<_> = b.keys().cloned().collect();
    assert_eq!(a_names, b_names, "{label}: zip part names differ");

    for (name, ab) in &a {
        let bb = b.get(name).unwrap();
        if is_xml_part(name) {
            let na = normalize_xml(ab);
            let nb = normalize_xml(bb);
            assert_eq!(
                String::from_utf8_lossy(&na),
                String::from_utf8_lossy(&nb),
                "{label}: part {name} differs after normalization",
            );
        } else {
            assert_eq!(ab, bb, "{label}: binary part {name} differs");
        }
    }
}

/// Like `assert_zip_xml_equivalent` but every part name listed in
/// `expected_modified` is allowed to differ. Use for the modify-then-write
/// test: only the modified XML parts (e.g. `word/document.xml`) should
/// change; everything else must be preserved verbatim.
pub fn assert_unchanged_parts_equivalent(
    label: &str,
    input: &[u8],
    output: &[u8],
    expected_modified: &[&str],
) {
    let a = unzip_parts(input);
    let b = unzip_parts(output);
    let a_names: Vec<_> = a.keys().cloned().collect();
    let b_names: Vec<_> = b.keys().cloned().collect();
    assert_eq!(a_names, b_names, "{label}: zip part names differ");

    for (name, ab) in &a {
        if expected_modified.contains(&name.as_str()) {
            continue;
        }
        let bb = b.get(name).unwrap();
        if is_xml_part(name) {
            assert_eq!(
                String::from_utf8_lossy(&normalize_xml(ab)),
                String::from_utf8_lossy(&normalize_xml(bb)),
                "{label}: untouched part {name} differs after normalization",
            );
        } else {
            assert_eq!(ab, bb, "{label}: untouched binary part {name} differs");
        }
    }
}

fn is_xml_part(name: &str) -> bool {
    name.ends_with(".xml") || name.ends_with(".rels")
}
