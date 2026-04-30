//! Zotero plugin — Phase 5.4 v0 hero feature.
//!
//! Reads a document looking for `[[zot:KEY]]` markers, fetches
//! each unique key from a BetterBibTeX CSL-JSON RPC, replaces the
//! markers with rendered inline citations, and (re)renders a
//! bibliography section delimited by `<!-- bib-start -->` /
//! `<!-- bib-end -->`. The active CSL style is read from a
//! `<!-- style: ID -->` marker at the top of the document; if
//! absent, defaults to `"apa"`.
//!
//! Re-running the plugin is idempotent: each rendered inline
//! citation is wrapped in `<!-- zot:KEY -->RENDERED<!-- /zot -->`,
//! so a subsequent run (e.g. after the user changed the style
//! marker) finds the keys again and re-renders in the new style.
//!
//! ## Architecture
//!
//! All transformation logic lives as pure functions on this
//! module; only `run()` (gated by `#[cfg(target_arch = "wasm32")]`)
//! does host I/O. That separation lets `cargo test` run on the
//! host target without depending on wasmtime, the wit-bindgen
//! guest extern "C" symbols, or a network.

#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeSet;

/// CSL style id used when no `<!-- style: ID -->` marker is found.
pub const DEFAULT_STYLE: &str = "apa";

/// Marker prefix that identifies a yet-to-be-rendered citation.
pub const RAW_MARKER_OPEN: &str = "[[zot:";
const RAW_MARKER_CLOSE: &str = "]]";

/// Marker pair that wraps a rendered inline citation. Survives
/// re-runs so that a style change can re-render in place.
pub const RENDERED_OPEN_PREFIX: &str = "<!-- zot:";
const RENDERED_OPEN_SUFFIX: &str = " -->";
const RENDERED_CLOSE: &str = "<!-- /zot -->";

/// Markers around the auto-generated bibliography block.
pub const BIB_START: &str = "<!-- bib-start -->";
pub const BIB_END: &str = "<!-- bib-end -->";

/// Style marker prefix `<!-- style: ID -->`.
const STYLE_OPEN: &str = "<!-- style:";

/// Extract every unique citation key referenced in `doc`, in the
/// order they first appear. Picks up both `[[zot:KEY]]` and the
/// rendered form `<!-- zot:KEY -->...<!-- /zot -->` so
/// already-rendered docs still re-render on a style change.
pub fn extract_keys(doc: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut keys = Vec::new();
    for key in iter_raw_keys(doc).chain(iter_rendered_keys(doc)) {
        if seen.insert(key.clone()) {
            keys.push(key);
        }
    }
    keys
}

fn iter_raw_keys(doc: &str) -> impl Iterator<Item = String> + '_ {
    iter_keys_between(doc, RAW_MARKER_OPEN, RAW_MARKER_CLOSE)
}

fn iter_rendered_keys(doc: &str) -> impl Iterator<Item = String> + '_ {
    iter_keys_between(doc, RENDERED_OPEN_PREFIX, RENDERED_OPEN_SUFFIX)
}

fn iter_keys_between<'a>(
    doc: &'a str,
    open: &'static str,
    close: &'static str,
) -> impl Iterator<Item = String> + 'a {
    let mut rest = doc;
    std::iter::from_fn(move || {
        let start = rest.find(open)?;
        let after_open = &rest[start + open.len()..];
        let end = after_open.find(close)?;
        let key = after_open[..end].trim().to_string();
        rest = &after_open[end + close.len()..];
        if key.is_empty() { None } else { Some(key) }
    })
}

/// Read the style marker `<!-- style: ID -->` (whitespace-trimmed
/// inside the marker) or fall back to [`DEFAULT_STYLE`].
pub fn parse_style(doc: &str) -> String {
    let Some(start) = doc.find(STYLE_OPEN) else {
        return DEFAULT_STYLE.into();
    };
    let after = &doc[start + STYLE_OPEN.len()..];
    let Some(end) = after.find("-->") else {
        return DEFAULT_STYLE.into();
    };
    let id = after[..end].trim();
    if id.is_empty() {
        DEFAULT_STYLE.into()
    } else {
        id.to_string()
    }
}

/// Replace every occurrence of the raw marker `[[zot:KEY]]` and
/// every existing rendered block `<!-- zot:KEY -->...<!-- /zot -->`
/// with a fresh rendered block for `key` carrying `rendered`.
pub fn apply_inline(doc: &str, key: &str, rendered: &str) -> String {
    let raw = format!("{RAW_MARKER_OPEN}{key}{RAW_MARKER_CLOSE}");
    let opener = format!("{RENDERED_OPEN_PREFIX}{key}{RENDERED_OPEN_SUFFIX}");
    let block = format!("{opener}{rendered}{RENDERED_CLOSE}");

    let mut out = doc.replace(&raw, &block);
    // Replace any existing rendered block for this key with the new
    // one. Advance past each replacement so we don't re-process the
    // freshly-inserted block (which itself begins with `opener`).
    let mut cursor = 0;
    while let Some(rel) = out[cursor..].find(&opener) {
        let start = cursor + rel;
        let after = start + opener.len();
        let Some(close_rel) = out[after..].find(RENDERED_CLOSE) else {
            break;
        };
        let close = after + close_rel + RENDERED_CLOSE.len();
        out.replace_range(start..close, &block);
        cursor = start + block.len();
    }
    out
}

/// Replace the bibliography block delimited by [`BIB_START`] /
/// [`BIB_END`] with `bib`. If the block is absent, append it
/// (preceded by a blank line) at the end of the doc.
pub fn apply_bibliography(doc: &str, bib: &str) -> String {
    if let Some(start) = doc.find(BIB_START)
        && let Some(end_rel) = doc[start..].find(BIB_END)
    {
        let end = start + end_rel + BIB_END.len();
        let mut out = String::with_capacity(doc.len() + bib.len());
        out.push_str(&doc[..start]);
        out.push_str(BIB_START);
        out.push('\n');
        out.push_str(bib);
        out.push('\n');
        out.push_str(BIB_END);
        out.push_str(&doc[end..]);
        return out;
    }
    let trimmed = doc.trim_end();
    let mut out = String::with_capacity(doc.len() + bib.len() + 64);
    out.push_str(trimmed);
    if !trimmed.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(BIB_START);
    out.push('\n');
    out.push_str(bib);
    out.push('\n');
    out.push_str(BIB_END);
    out.push('\n');
    out
}

/// Concatenate item JSON strings into a single CSL-JSON array.
/// Each `items[i].1` MUST be a valid JSON object string.
pub fn build_items_json(items: &[(String, String)]) -> String {
    let mut out = String::from("[");
    for (i, (_, json)) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(json);
    }
    out.push(']');
    out
}

/// Pure orchestrator: render every cited item in `items` and
/// splice the bibliography. The two callbacks are the host's
/// `cite.format-inline` and `cite.format-bib`; tests pass mocks.
pub fn process_doc<F1, F2>(
    doc: &str,
    style: &str,
    items: &[(String, String)],
    mut render_inline: F1,
    mut render_bib: F2,
) -> String
where
    F1: FnMut(&str, &str) -> std::result::Result<String, String>,
    F2: FnMut(&str, &str) -> std::result::Result<String, String>,
{
    let mut out = doc.to_string();
    for (key, item_json) in items {
        let rendered = match render_inline(item_json, style) {
            Ok(s) => s,
            Err(e) => format!("[zot-err:{e}]"),
        };
        out = apply_inline(&out, key, &rendered);
    }
    let items_json = build_items_json(items);
    let bib = match render_bib(&items_json, style) {
        Ok(s) => s,
        Err(e) => format!("[zot-bib-err:{e}]"),
    };
    apply_bibliography(&out, &bib)
}

/// Build the BetterBibTeX RPC URL for a single citekey. The
/// `csljson` endpoint is what BBT exposes for CSL-JSON export.
pub fn rpc_url_for(key: &str) -> String {
    format!("http://localhost:23119/better-bibtex/csljson?key={key}")
}

// ─────────────────────────────────────────────────────────────────
// WASM guest entry point — only compiled for wasm32 target.
// ─────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod guest {
    wit_bindgen::generate!({
        world: "zotero-plugin",
        path: "wit/",
        generate_all,
    });

    use super::*;
    use exports::apalabrar::editor::plugin::Guest;

    struct Component;

    impl Guest for Component {
        fn run() {
            let doc = apalabrar::editor::doc::read();
            let style = parse_style(&doc);
            let keys = extract_keys(&doc);

            let mut items: Vec<(String, String)> = Vec::new();
            for key in &keys {
                let url = rpc_url_for(key);
                if let Ok(json) = apalabrar::editor::net::request(&url, "GET") {
                    items.push((key.clone(), json));
                }
            }

            let new_doc = process_doc(
                &doc,
                &style,
                &items,
                |item_json, style_id| apalabrar::editor::cite::format_inline(item_json, style_id),
                |items_json, style_id| apalabrar::editor::cite::format_bib(items_json, style_id),
            );

            apalabrar::editor::doc_mut::write(&new_doc);
            apalabrar::editor::ui::register_panel(
                "Zotero",
                &format!("Imported {} items in {style} style", items.len()),
            );
        }
    }

    export!(Component);
}

// ─────────────────────────────────────────────────────────────────
// Unit tests — host target.
// ─────────────────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn extract_keys_finds_raw_markers() {
        let doc = "Hello [[zot:Smith2020]] world [[zot:Jones2019]] end.";
        assert_eq!(
            extract_keys(doc),
            vec!["Smith2020".to_string(), "Jones2019".to_string()]
        );
    }

    #[test]
    fn extract_keys_dedupes() {
        let doc = "[[zot:K]] [[zot:K]] [[zot:K]]";
        assert_eq!(extract_keys(doc), vec!["K".to_string()]);
    }

    #[test]
    fn extract_keys_finds_rendered_markers() {
        let doc = "<!-- zot:Smith2020 -->(Smith, 2020)<!-- /zot --> tail";
        assert_eq!(extract_keys(doc), vec!["Smith2020".to_string()]);
    }

    #[test]
    fn extract_keys_combines_raw_and_rendered() {
        let doc = "<!-- zot:A -->X<!-- /zot --> [[zot:B]]";
        let keys = extract_keys(doc);
        assert!(keys.contains(&"A".into()));
        assert!(keys.contains(&"B".into()));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn extract_keys_returns_empty_for_no_markers() {
        assert!(extract_keys("plain prose").is_empty());
    }

    #[test]
    fn extract_keys_skips_empty_keys() {
        // [[zot:]] with empty key should not register.
        assert!(extract_keys("[[zot:]] body").is_empty());
    }

    #[test]
    fn parse_style_returns_default_when_marker_absent() {
        assert_eq!(parse_style("hello"), "apa");
    }

    #[test]
    fn parse_style_extracts_value_from_marker() {
        assert_eq!(parse_style("<!-- style: harvard -->\nbody"), "harvard");
    }

    #[test]
    fn parse_style_trims_whitespace_inside_marker() {
        assert_eq!(parse_style("<!-- style:    nature   -->"), "nature");
    }

    #[test]
    fn parse_style_falls_back_when_marker_unclosed() {
        // Missing `-->` → fall back.
        assert_eq!(parse_style("<!-- style: harvard "), "apa");
    }

    #[test]
    fn parse_style_falls_back_when_marker_empty() {
        assert_eq!(parse_style("<!-- style:  -->"), "apa");
    }

    #[test]
    fn apply_inline_replaces_raw_marker() {
        let out = apply_inline("a [[zot:K]] b", "K", "(Smith, 2020)");
        assert_eq!(out, "a <!-- zot:K -->(Smith, 2020)<!-- /zot --> b");
    }

    #[test]
    fn apply_inline_replaces_existing_rendered_block() {
        let doc = "head <!-- zot:K -->OLD<!-- /zot --> tail";
        let out = apply_inline(doc, "K", "NEW");
        assert_eq!(out, "head <!-- zot:K -->NEW<!-- /zot --> tail");
    }

    #[test]
    fn apply_inline_replaces_multiple_occurrences() {
        let doc = "[[zot:K]] mid [[zot:K]]";
        let out = apply_inline(doc, "K", "X");
        assert_eq!(
            out,
            "<!-- zot:K -->X<!-- /zot --> mid <!-- zot:K -->X<!-- /zot -->"
        );
    }

    #[test]
    fn apply_inline_handles_doc_starting_with_rendered_block() {
        // Doc starts with the rendered marker (start = 0). Catches
        // mutations like `cursor = start * block.len()` that fail to
        // advance when start is 0.
        let doc = "<!-- zot:K -->OLD<!-- /zot --> tail";
        let out = apply_inline(doc, "K", "NEW");
        assert_eq!(out, "<!-- zot:K -->NEW<!-- /zot --> tail");
    }

    #[test]
    fn apply_inline_does_not_touch_other_keys() {
        let doc = "[[zot:A]] [[zot:B]]";
        let out = apply_inline(doc, "A", "AAA");
        assert_eq!(out, "<!-- zot:A -->AAA<!-- /zot --> [[zot:B]]");
    }

    #[test]
    fn apply_bibliography_appends_when_block_missing() {
        let out = apply_bibliography("body text", "BIB");
        assert!(out.contains(BIB_START));
        assert!(out.contains("BIB"));
        assert!(out.contains(BIB_END));
        assert!(out.starts_with("body text"));
    }

    #[test]
    fn apply_bibliography_replaces_existing_block() {
        let doc = "pre\n\n<!-- bib-start -->\nOLD\n<!-- bib-end -->\npost";
        let out = apply_bibliography(doc, "NEW");
        assert!(out.contains("NEW"));
        assert!(!out.contains("OLD"));
        assert!(out.contains("pre"));
        assert!(out.contains("post"));
    }

    #[test]
    fn apply_bibliography_appends_to_empty_doc_without_double_blank() {
        let out = apply_bibliography("", "BIB");
        // No leading blank line when source was empty.
        assert!(out.starts_with(BIB_START));
    }

    #[test]
    fn build_items_json_empty_yields_empty_array() {
        assert_eq!(build_items_json(&[]), "[]");
    }

    #[test]
    fn build_items_json_concatenates_with_commas() {
        let items = vec![
            ("a".into(), r#"{"id":"a"}"#.into()),
            ("b".into(), r#"{"id":"b"}"#.into()),
        ];
        assert_eq!(build_items_json(&items), r#"[{"id":"a"},{"id":"b"}]"#);
    }

    #[test]
    fn rpc_url_for_uses_better_bibtex_csljson_endpoint() {
        assert_eq!(
            rpc_url_for("Smith2020"),
            "http://localhost:23119/better-bibtex/csljson?key=Smith2020"
        );
    }

    #[test]
    fn process_doc_renders_inline_then_appends_bibliography() {
        let doc = "Quote: [[zot:K]].";
        let items = vec![("K".into(), r#"{"id":"K","type":"book"}"#.into())];
        let out = process_doc(
            doc,
            "apa",
            &items,
            |_, _| Ok("(Smith 2020)".into()),
            |_, _| Ok("Smith J. 2020. Title.".into()),
        );
        assert!(out.contains("(Smith 2020)"));
        assert!(out.contains(BIB_START));
        assert!(out.contains("Smith J. 2020. Title."));
        assert!(out.contains(BIB_END));
    }

    #[test]
    fn process_doc_surfaces_inline_render_error_inline() {
        let doc = "[[zot:K]]";
        let items = vec![("K".into(), "{}".into())];
        let out = process_doc(
            doc,
            "apa",
            &items,
            |_, _| Err("bad item".into()),
            |_, _| Ok("BIB".into()),
        );
        assert!(out.contains("[zot-err:bad item]"));
    }

    #[test]
    fn process_doc_surfaces_bib_render_error_in_block() {
        let doc = "[[zot:K]]";
        let items = vec![("K".into(), "{}".into())];
        let out = process_doc(
            doc,
            "apa",
            &items,
            |_, _| Ok("INL".into()),
            |_, _| Err("unknown style".into()),
        );
        assert!(out.contains("[zot-bib-err:unknown style]"));
    }

    #[test]
    fn process_doc_re_renders_existing_inline_blocks_on_style_change() {
        // Doc already contains a rendered block in APA; re-running
        // with Harvard render fn should swap the inner text.
        let doc = "<!-- zot:K -->(APA)<!-- /zot -->";
        let items = vec![("K".into(), "{}".into())];
        let out = process_doc(
            doc,
            "harvard",
            &items,
            |_, _| Ok("(HARVARD)".into()),
            |_, _| Ok("BIB".into()),
        );
        assert!(out.contains("(HARVARD)"));
        assert!(!out.contains("(APA)"));
    }

    #[test]
    fn process_doc_replaces_existing_bib_block() {
        let doc = "body\n<!-- bib-start -->\nOLD\n<!-- bib-end -->\n";
        let out = process_doc(
            doc,
            "apa",
            &[],
            |_, _| Ok("X".into()),
            |_, _| Ok("FRESH".into()),
        );
        assert!(out.contains("FRESH"));
        assert!(!out.contains("OLD"));
    }
}
