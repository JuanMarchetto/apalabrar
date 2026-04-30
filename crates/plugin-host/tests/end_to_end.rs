//! End-to-end test: the real reference plugin (`plugin-word-counter`)
//! is compiled out-of-band and committed as a binary fixture; this
//! suite loads it, runs it, and asserts on the host-side state
//! the plugin produced via the granted capabilities.

use apalabrar_plugin_host::{Capability, Error, Grants, Host, Manifest, Quota};

const WORD_COUNTER_BYTES: &[u8] = include_bytes!("fixtures/word_counter.component.wasm");

fn full_manifest() -> Manifest {
    Manifest {
        id: "plugin-word-counter".into(),
        version: "0.0.0".into(),
        capabilities: vec![Capability::DocRead, Capability::UiPanel],
    }
}

fn full_grants() -> Grants {
    Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::UiPanel)
}

#[test]
fn word_counter_counts_three_words() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            WORD_COUNTER_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_doc("hola que tal");
    plugin.run().unwrap();
    let panels = plugin.ui_panels();
    assert_eq!(panels.len(), 1);
    assert_eq!(panels[0].0, "Word Counter");
    assert_eq!(panels[0].1, "3 words");
}

#[test]
fn word_counter_counts_zero_on_empty_doc() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            WORD_COUNTER_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.run().unwrap();
    assert_eq!(plugin.ui_panels()[0].1, "0 words");
}

#[test]
fn word_counter_counts_long_doc() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            WORD_COUNTER_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    let text = "lorem ipsum dolor sit amet ".repeat(100); // 500 words
    plugin.set_doc(text);
    plugin.run().unwrap();
    assert_eq!(plugin.ui_panels()[0].1, "500 words");
}

#[test]
fn word_counter_isolation_two_plugin_instances() {
    let host = Host::new().unwrap();
    let mut a = host
        .load(
            WORD_COUNTER_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    let mut b = host
        .load(
            WORD_COUNTER_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    a.set_doc("uno dos");
    b.set_doc("uno dos tres cuatro cinco");
    a.run().unwrap();
    b.run().unwrap();
    assert_eq!(a.ui_panels()[0].1, "2 words");
    assert_eq!(b.ui_panels()[0].1, "5 words");
}

#[test]
fn word_counter_denied_doc_read_when_only_ui_granted() {
    let host = Host::new().unwrap();
    let r = host.load(
        WORD_COUNTER_BYTES,
        &full_manifest(),
        Grants::empty().allow(Capability::UiPanel),
        Quota::large(),
    );
    assert!(matches!(
        r,
        Err(Error::CapabilityDenied(Capability::DocRead))
    ));
}

#[test]
fn word_counter_denied_ui_when_only_doc_read_granted() {
    let host = Host::new().unwrap();
    let r = host.load(
        WORD_COUNTER_BYTES,
        &full_manifest(),
        Grants::empty().allow(Capability::DocRead),
        Quota::large(),
    );
    assert!(matches!(
        r,
        Err(Error::CapabilityDenied(Capability::UiPanel))
    ));
}

#[test]
fn word_counter_runs_under_low_fuel() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            WORD_COUNTER_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::new(1_000, 64),
        )
        .unwrap();
    plugin.set_doc("a");
    let r = plugin.run();
    // 1000 fuel is insufficient for the wit-bindgen prologue + a
    // realloc + register-panel call. We assert this exhausts fuel.
    assert!(matches!(r, Err(Error::FuelExhausted)), "got: {:?}", r);
}

#[test]
fn word_counter_memory_below_minimum_returns_memory_exceeded() {
    let host = Host::new().unwrap();
    let r = host.load(
        WORD_COUNTER_BYTES,
        &full_manifest(),
        full_grants(),
        Quota::new(1_000_000, 1), // plugin needs 17+ pages
    );
    assert!(
        matches!(r, Err(Error::MemoryExceeded)),
        "got: {:?}",
        r.err()
    );
}
