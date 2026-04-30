//! Tests that drive the doc-write and net.http import branches by
//! using the `plugin-full-fixture` reference plugin (which calls
//! all four imports unconditionally).

use apalabrar_plugin_host::{Capability, Error, Grants, Host, Manifest, Quota};

const FULL_FIXTURE_BYTES: &[u8] = include_bytes!("fixtures/full_fixture.component.wasm");

fn full_manifest() -> Manifest {
    Manifest {
        id: "plugin-full-fixture".into(),
        version: "0.0.0".into(),
        capabilities: vec![
            Capability::DocRead,
            Capability::DocWrite,
            Capability::UiPanel,
            Capability::NetHttp,
        ],
    }
}

fn full_grants() -> Grants {
    Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::DocWrite)
        .allow(Capability::UiPanel)
        .allow(Capability::NetHttp)
}

#[test]
fn full_fixture_writes_doc_via_doc_mut() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            FULL_FIXTURE_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_doc("hi");
    plugin.set_http_responder(|_, _| Ok("body".into()));
    plugin.run().unwrap();
    assert_eq!(plugin.doc(), "ECHO[hi]");
}

#[test]
fn full_fixture_calls_installed_responder() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            FULL_FIXTURE_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(|url, method| {
        assert_eq!(url, "https://example.test");
        assert_eq!(method, "GET");
        Ok("the-body".into())
    });
    plugin.run().unwrap();
    assert_eq!(plugin.ui_panels()[0].1, "net ok: the-body");
}

#[test]
fn full_fixture_default_responder_returns_error_string() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            FULL_FIXTURE_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    // No responder installed → default returns Err.
    plugin.run().unwrap();
    assert!(
        plugin.ui_panels()[0]
            .1
            .starts_with("net err: no http responder configured")
    );
}

#[test]
fn full_fixture_responder_error_propagates_to_panel() {
    let host = Host::new().unwrap();
    let mut plugin = host
        .load(
            FULL_FIXTURE_BYTES,
            &full_manifest(),
            full_grants(),
            Quota::large(),
        )
        .unwrap();
    plugin.set_http_responder(|_, _| Err("dns failure".into()));
    plugin.run().unwrap();
    assert_eq!(plugin.ui_panels()[0].1, "net err: dns failure");
}

#[test]
fn full_fixture_denied_when_doc_write_missing() {
    let host = Host::new().unwrap();
    let r = host.load(
        FULL_FIXTURE_BYTES,
        &full_manifest(),
        Grants::empty()
            .allow(Capability::DocRead)
            .allow(Capability::UiPanel)
            .allow(Capability::NetHttp),
        Quota::large(),
    );
    assert!(matches!(
        r,
        Err(Error::CapabilityDenied(Capability::DocWrite))
    ));
}

#[test]
fn full_fixture_denied_when_net_http_missing() {
    let host = Host::new().unwrap();
    let r = host.load(
        FULL_FIXTURE_BYTES,
        &full_manifest(),
        Grants::empty()
            .allow(Capability::DocRead)
            .allow(Capability::DocWrite)
            .allow(Capability::UiPanel),
        Quota::large(),
    );
    assert!(matches!(
        r,
        Err(Error::CapabilityDenied(Capability::NetHttp))
    ));
}
