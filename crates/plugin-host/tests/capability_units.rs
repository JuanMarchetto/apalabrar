//! Pure-Rust unit tests for [`Grants`] / [`Capability`] —
//! no WASM, no wasmtime. Covers the security boundary's
//! pure-data layer.

use apalabrar_plugin_host::{Capability, Grants};

#[test]
fn empty_grants_contain_nothing() {
    let g = Grants::empty();
    assert!(!g.contains(Capability::DocRead));
    assert!(!g.contains(Capability::DocWrite));
    assert!(!g.contains(Capability::UiPanel));
    assert!(!g.contains(Capability::NetHttp));
}

#[test]
fn allow_doc_read_only_contains_doc_read() {
    let g = Grants::empty().allow(Capability::DocRead);
    assert!(g.contains(Capability::DocRead));
    assert!(!g.contains(Capability::DocWrite));
    assert!(!g.contains(Capability::UiPanel));
    assert!(!g.contains(Capability::NetHttp));
}

#[test]
fn allow_chains_accumulate() {
    let g = Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::UiPanel);
    assert!(g.contains(Capability::DocRead));
    assert!(g.contains(Capability::UiPanel));
    assert!(!g.contains(Capability::DocWrite));
    assert!(!g.contains(Capability::NetHttp));
}

#[test]
fn allow_is_idempotent() {
    let g = Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::DocRead);
    assert_eq!(g.iter().count(), 1);
}

#[test]
fn iter_is_sorted_and_complete() {
    let g = Grants::empty()
        .allow(Capability::NetHttp)
        .allow(Capability::DocRead)
        .allow(Capability::UiPanel)
        .allow(Capability::DocWrite);
    let collected: Vec<_> = g.iter().collect();
    // BTreeSet ordering: enum discriminants in declaration order.
    assert_eq!(
        collected,
        vec![
            Capability::DocRead,
            Capability::DocWrite,
            Capability::UiPanel,
            Capability::NetHttp,
        ]
    );
}

#[test]
fn clone_does_not_share_state() {
    let a = Grants::empty().allow(Capability::DocRead);
    let b = a.clone();
    let a2 = a.allow(Capability::UiPanel);
    // b must still have only DocRead, since clone was independent.
    assert!(b.contains(Capability::DocRead));
    assert!(!b.contains(Capability::UiPanel));
    assert!(a2.contains(Capability::UiPanel));
}

#[test]
fn capability_serializes_kebab_case() {
    let cap = Capability::DocRead;
    let json = serde_json::to_string(&cap).unwrap();
    assert_eq!(json, "\"doc-read\"");
}

#[test]
fn capability_deserializes_kebab_case() {
    let cap: Capability = serde_json::from_str("\"net-http\"").unwrap();
    assert_eq!(cap, Capability::NetHttp);
}

#[test]
fn capability_unknown_kebab_errors() {
    let r: serde_json::Result<Capability> = serde_json::from_str("\"fs-read\"");
    assert!(r.is_err());
}

#[test]
fn grants_default_is_empty() {
    let g = Grants::default();
    assert_eq!(g.iter().count(), 0);
}

#[test]
fn grants_eq_compares_set_contents() {
    let a = Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::UiPanel);
    let b = Grants::empty()
        .allow(Capability::UiPanel)
        .allow(Capability::DocRead);
    assert_eq!(a, b, "order of allow() calls must not matter");
}
