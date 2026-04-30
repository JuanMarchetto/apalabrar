//! [`Manifest`] parser tests.

use apalabrar_plugin_host::{Capability, Manifest};

#[test]
fn parse_minimum_valid_no_capabilities() {
    let json = r#"{"id":"x","version":"0.1.0","capabilities":[]}"#;
    let m = Manifest::from_json(json).unwrap();
    assert_eq!(m.id, "x");
    assert_eq!(m.version, "0.1.0");
    assert!(m.capabilities.is_empty());
}

#[test]
fn parse_with_one_capability() {
    let json = r#"{"id":"x","version":"0.1.0","capabilities":["doc-read"]}"#;
    let m = Manifest::from_json(json).unwrap();
    assert_eq!(m.capabilities, vec![Capability::DocRead]);
}

#[test]
fn parse_with_all_capabilities() {
    let json = r#"{"id":"x","version":"0.1.0","capabilities":["doc-read","doc-write","ui-panel","net-http"]}"#;
    let m = Manifest::from_json(json).unwrap();
    assert_eq!(m.capabilities.len(), 4);
    assert!(m.capabilities.contains(&Capability::DocRead));
    assert!(m.capabilities.contains(&Capability::DocWrite));
    assert!(m.capabilities.contains(&Capability::UiPanel));
    assert!(m.capabilities.contains(&Capability::NetHttp));
}

#[test]
fn parse_invalid_json_errors() {
    let r = Manifest::from_json("not json");
    assert!(r.is_err());
}

#[test]
fn parse_missing_field_errors() {
    let r = Manifest::from_json(r#"{"id":"x","version":"0.1.0"}"#);
    assert!(r.is_err());
}

#[test]
fn parse_unknown_capability_errors() {
    let r = Manifest::from_json(r#"{"id":"x","version":"0.1.0","capabilities":["fs-read"]}"#);
    assert!(r.is_err());
}

#[test]
fn round_trip_serialize_deserialize() {
    let m = Manifest {
        id: "word-counter".into(),
        version: "0.1.0".into(),
        capabilities: vec![Capability::DocRead, Capability::UiPanel],
    };
    let json = serde_json::to_string(&m).unwrap();
    let m2 = Manifest::from_json(&json).unwrap();
    assert_eq!(m, m2);
}
