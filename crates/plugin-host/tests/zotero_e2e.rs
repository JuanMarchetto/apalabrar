//! End-to-end test of the Zotero plugin (`examples/plugin-zotero`).
//!
//! The plugin is compiled out-of-band to a Component and committed
//! at `tests/fixtures/zotero.component.wasm`. These tests:
//! * load it through `plugin-host` with all five capabilities,
//! * install a deterministic mock Zotero responder via
//!   [`Plugin::set_http_responder`],
//! * seed a doc with `[[zot:KEY]]` markers,
//! * run the plugin,
//! * assert that the doc contains rendered inline citations and a
//!   bibliography block in the requested style.

use apalabrar_plugin_host::{Capability, Grants, Host, Manifest, Quota};

const ZOTERO_BYTES: &[u8] = include_bytes!("fixtures/zotero.component.wasm");

fn full_manifest() -> Manifest {
    Manifest {
        id: "plugin-zotero".into(),
        version: "0.0.0".into(),
        capabilities: vec![
            Capability::DocRead,
            Capability::DocWrite,
            Capability::UiPanel,
            Capability::NetHttp,
            Capability::CiteRender,
        ],
    }
}

fn full_grants() -> Grants {
    Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::DocWrite)
        .allow(Capability::UiPanel)
        .allow(Capability::NetHttp)
        .allow(Capability::CiteRender)
}

/// Canned Zotero CSL-JSON for two citekeys. The plugin fetches
/// these via `net.request` (mocked); the host then renders them
/// with the bundled APA / Harvard CSL styles.
fn mock_responder() -> impl Fn(&str, &str) -> Result<String, String> + Send + Sync + 'static {
    |url: &str, _method: &str| -> Result<String, String> {
        if url.contains("key=Smith2020") {
            Ok(r#"{"id":"Smith2020","type":"article-journal","title":"On the nature of typesetting","author":[{"family":"Smith","given":"J"}],"issued":{"date-parts":[[2020]]},"container-title":"Journal of Typography","volume":"12","issue":"3","page":"123-145"}"#.into())
        } else if url.contains("key=Jones2019") {
            Ok(r#"{"id":"Jones2019","type":"article-journal","title":"Counterpoint","author":[{"family":"Jones","given":"A"}],"issued":{"date-parts":[[2019]]},"container-title":"Other Journal","volume":"5","page":"7-22"}"#.into())
        } else {
            Err("not found".into())
        }
    }
}

#[test]
fn zotero_renders_inline_and_bibliography_in_apa() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(mock_responder());
    plugin.set_doc("Some prose [[zot:Smith2020]] and more [[zot:Jones2019]].");
    plugin.run().unwrap();
    let out = plugin.doc();
    // Inline markers replaced with rendered blocks.
    assert!(
        out.contains("<!-- zot:Smith2020 -->") && out.contains("<!-- /zot -->"),
        "missing rendered Smith block: {out}"
    );
    assert!(
        out.contains("<!-- zot:Jones2019 -->"),
        "missing rendered Jones block: {out}"
    );
    // Bibliography appended.
    assert!(
        out.contains("<!-- bib-start -->") && out.contains("<!-- bib-end -->"),
        "missing bib block: {out}"
    );
    // APA-style author render in the bib (last name + initial).
    assert!(out.contains("Smith"), "expected Smith in bib: {out}");
    assert!(out.contains("Jones"), "expected Jones in bib: {out}");
}

#[test]
fn zotero_re_renders_bibliography_when_style_changes() {
    let host = Host::new().unwrap();

    // First run: APA. Capture the resulting doc.
    let mut p1 = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    p1.set_http_responder(mock_responder());
    p1.set_doc("<!-- style: apa -->\n[[zot:Smith2020]]");
    p1.run().unwrap();
    let after_apa = p1.doc().to_string();

    // Second run: same input doc but Harvard style — should
    // produce a *different* bibliography string than APA, even
    // though the same single item is cited.
    let mut p2 = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    p2.set_http_responder(mock_responder());
    p2.set_doc("<!-- style: harvard -->\n[[zot:Smith2020]]");
    p2.run().unwrap();
    let after_harvard = p2.doc().to_string();

    assert_ne!(
        after_apa, after_harvard,
        "APA and Harvard renders should differ"
    );
    // Both should still contain a bibliography block.
    assert!(after_apa.contains("<!-- bib-start -->"));
    assert!(after_harvard.contains("<!-- bib-start -->"));
}

#[test]
fn zotero_handles_unknown_key_gracefully() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(mock_responder());
    plugin.set_doc("Unknown [[zot:NotInZotero]] key.");
    // Should not trap — just skip the key, produce empty bib.
    plugin.run().unwrap();
    let out = plugin.doc();
    // Marker stays as-is (no fetched item to render with).
    assert!(out.contains("[[zot:NotInZotero]]"), "marker dropped: {out}");
    assert!(out.contains("<!-- bib-start -->"), "missing bib: {out}");
}

#[test]
fn zotero_empty_doc_produces_empty_bibliography_block() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(mock_responder());
    plugin.run().unwrap();
    let out = plugin.doc();
    // No items → bib block is just the empty bibliography (might
    // be the empty string between the markers).
    assert!(
        out.contains("<!-- bib-start -->") && out.contains("<!-- bib-end -->"),
        "expected bib markers even on empty doc: {out}"
    );
}

#[test]
fn zotero_dedupes_duplicate_keys_in_bibliography() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(mock_responder());
    plugin.set_doc("[[zot:Smith2020]] and again [[zot:Smith2020]] and [[zot:Smith2020]].");
    plugin.run().unwrap();
    let out = plugin.doc();
    // Each marker → one rendered block; bib still has Smith only once.
    let bib_start = out.find("<!-- bib-start -->").unwrap();
    let bib_end = out.find("<!-- bib-end -->").unwrap();
    let bib = &out[bib_start..bib_end];
    let occurrences = bib.matches("Smith").count();
    assert_eq!(
        occurrences, 1,
        "Smith should appear once in bib, got: {bib}"
    );
}

#[test]
fn zotero_panel_reports_imported_count_and_style() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            ZOTERO_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(mock_responder());
    plugin.set_doc("<!-- style: harvard -->\n[[zot:Smith2020]] [[zot:Jones2019]]");
    plugin.run().unwrap();
    let panels = plugin.ui_panels();
    assert_eq!(panels.len(), 1);
    assert_eq!(panels[0].0, "Zotero");
    assert!(panels[0].1.contains("2"), "panel body: {}", panels[0].1);
    assert!(
        panels[0].1.contains("harvard"),
        "panel body: {}",
        panels[0].1
    );
}
