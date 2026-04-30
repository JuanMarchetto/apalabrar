//! WASM-integration tests for [`Host`] / [`Plugin`].
//!
//! Fixtures are built inline via [`wat::parse_str`] so the test
//! suite stays self-contained (no compiled-binary blobs in the
//! repo, no external toolchain required).

use apalabrar_plugin_host::{Capability, Error, Grants, Host, Manifest, Quota};

fn manifest_with(caps: Vec<Capability>) -> Manifest {
    Manifest {
        id: "test".into(),
        version: "0.0.1".into(),
        capabilities: caps,
    }
}

/// A minimal Component that exports `apalabrar:editor/plugin@0.1.0/run`
/// and imports nothing.
fn empty_plugin_wat() -> &'static str {
    r#"
(component
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component that imports the doc-read interface (without
/// using it) — proves capability gating.
fn plugin_importing_doc_read_wat() -> &'static str {
    r#"
(component
  (import "apalabrar:editor/doc@0.1.0" (instance $doc
    (export "read" (func (result string)))
  ))
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

fn plugin_importing_doc_write_wat() -> &'static str {
    r#"
(component
  (import "apalabrar:editor/doc-mut@0.1.0" (instance $d
    (export "write" (func (param "text" string)))
  ))
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

fn plugin_importing_ui_wat() -> &'static str {
    r#"
(component
  (import "apalabrar:editor/ui@0.1.0" (instance $u
    (export "register-panel" (func (param "title" string) (param "content" string)))
  ))
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

fn plugin_importing_net_wat() -> &'static str {
    r#"
(component
  (import "apalabrar:editor/net@0.1.0" (instance $n
    (export "request" (func (param "url" string) (param "method" string) (result (result string (error string)))))
  ))
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A valid Component that does NOT export the plugin interface.
fn plugin_without_plugin_interface_wat() -> &'static str {
    r#"
(component
  (core module $m (func (export "noop")))
  (core instance $i (instantiate $m))
)
"#
}

/// A Component that exports the plugin interface but no `run` inside.
fn plugin_interface_without_run_wat() -> &'static str {
    r#"
(component
  (core module $m (func (export "other")))
  (core instance $i (instantiate $m))
  (func $other-lifted (canon lift (core func $i "other")))
  (instance $exports (export "other" (func $other-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component whose `run` returns u32 — wrong signature; the
/// host's typed::<(), ()> cast fails.
fn plugin_run_with_wrong_signature_wat() -> &'static str {
    r#"
(component
  (type $sig (func (result u32)))
  (core module $m
    (func (export "run") (result i32) i32.const 0)
  )
  (core instance $i (instantiate $m))
  (func $run-lifted (type $sig) (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component whose `run` reads memory out of bounds — drives
/// the runtime MemoryOutOfBounds trap classifier branch.
fn plugin_oob_memory_read_wat() -> &'static str {
    r#"
(component
  (core module $m
    (memory (export "memory") 1)
    (func (export "run")
      i32.const 1000000
      i32.load drop
    )
  )
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component whose `run` hits `unreachable` — drives the
/// classifier fallthrough (Trap variant that is neither
/// OutOfFuel nor MemoryOutOfBounds).
fn plugin_unreachable_wat() -> &'static str {
    r#"
(component
  (core module $m (func (export "run") unreachable))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component that imports an interface the host never
/// exposes — used to drive the link-error fallthrough path.
fn plugin_importing_unknown_interface_wat() -> &'static str {
    r#"
(component
  (import "apalabrar:editor/fs@0.1.0" (instance $fs
    (export "read-file" (func (param "path" string) (result string)))
  ))
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component that imports the cite interface — used for the
/// CiteRender denial path.
fn plugin_importing_cite_wat() -> &'static str {
    r#"
(component
  (import "apalabrar:editor/cite@0.1.0" (instance $c
    (export "format-bib" (func (param "items-json" string) (param "style" string) (result (result string (error string)))))
    (export "format-inline" (func (param "item-json" string) (param "style" string) (result (result string (error string)))))
  ))
  (core module $m (func (export "run")))
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

/// A Component whose `run` core function spins forever — used
/// to test fuel exhaustion.
fn infinite_loop_plugin_wat() -> &'static str {
    r#"
(component
  (core module $m
    (func (export "run")
      (loop $l
        br $l
      )
    )
  )
  (core instance $i (instantiate $m))
  (func $run-lifted (canon lift (core func $i "run")))
  (instance $exports (export "run" (func $run-lifted)))
  (export "apalabrar:editor/plugin@0.1.0" (instance $exports))
)
"#
}

#[test]
fn host_new_succeeds() {
    let h = Host::new();
    assert!(h.is_ok());
}

#[test]
fn load_invalid_bytes_errors() {
    let host = Host::new().unwrap();
    let r = host.load(
        &[0u8, 1, 2, 3],
        &manifest_with(vec![]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(matches!(r, Err(Error::InvalidComponent(_))));
}

#[test]
fn load_minimal_component_succeeds() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let plugin = host.load(
        &bytes,
        &manifest_with(vec![]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(plugin.is_ok(), "load failed: {:?}", plugin.err());
}

#[test]
fn run_minimal_plugin_returns_ok() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let mut plugin = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    assert!(plugin.run().is_ok());
}

#[test]
fn doc_read_denied_when_not_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_doc_read_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![Capability::DocRead]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(
        matches!(r, Err(Error::CapabilityDenied(Capability::DocRead))),
        "unexpected: {:?}",
        r.err()
    );
}

#[test]
fn doc_write_denied_when_not_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_doc_write_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![Capability::DocWrite]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(
        matches!(r, Err(Error::CapabilityDenied(Capability::DocWrite))),
        "unexpected: {:?}",
        r.err()
    );
}

#[test]
fn ui_panel_denied_when_not_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_ui_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![Capability::UiPanel]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(
        matches!(r, Err(Error::CapabilityDenied(Capability::UiPanel))),
        "unexpected: {:?}",
        r.err()
    );
}

#[test]
fn net_http_denied_when_not_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_net_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![Capability::NetHttp]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(
        matches!(r, Err(Error::CapabilityDenied(Capability::NetHttp))),
        "unexpected: {:?}",
        r.err()
    );
}

#[test]
fn doc_read_succeeds_when_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_doc_read_wat()).unwrap();
    let plugin = host.load(
        &bytes,
        &manifest_with(vec![Capability::DocRead]),
        Grants::empty().allow(Capability::DocRead),
        Quota::small(),
    );
    assert!(
        plugin.is_ok(),
        "expected granted load to succeed: {:?}",
        plugin.err()
    );
}

#[test]
fn ui_panel_succeeds_when_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_ui_wat()).unwrap();
    let plugin = host.load(
        &bytes,
        &manifest_with(vec![Capability::UiPanel]),
        Grants::empty().allow(Capability::UiPanel),
        Quota::small(),
    );
    assert!(
        plugin.is_ok(),
        "expected granted load to succeed: {:?}",
        plugin.err()
    );
}

#[test]
fn host_rejects_grant_not_in_manifest() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![]), // empty
        Grants::empty().allow(Capability::DocRead),
        Quota::small(),
    );
    assert!(
        matches!(r, Err(Error::Manifest(_))),
        "unexpected: {:?}",
        r.err()
    );
}

#[test]
fn run_traps_when_plugin_interface_not_exported() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_without_plugin_interface_wat()).unwrap();
    let mut p = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    let r = p.run();
    let msg = match r {
        Err(Error::Trap(m)) => m,
        other => panic!("expected Trap, got {other:?}"),
    };
    assert!(msg.contains("plugin interface not exported"), "msg: {msg}");
}

#[test]
fn run_traps_when_run_function_not_exported() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_interface_without_run_wat()).unwrap();
    let mut p = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    let r = p.run();
    let msg = match r {
        Err(Error::Trap(m)) => m,
        other => panic!("expected Trap, got {other:?}"),
    };
    assert!(msg.contains("plugin.run not exported"), "msg: {msg}");
}

#[test]
fn run_traps_when_run_has_wrong_signature() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_run_with_wrong_signature_wat()).unwrap();
    let mut p = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    let r = p.run();
    // typed::<(), ()> fails when the actual signature doesn't match.
    assert!(matches!(r, Err(Error::Trap(_))), "got: {:?}", r);
}

#[test]
fn run_traps_oob_memory_read_as_memory_exceeded() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_oob_memory_read_wat()).unwrap();
    let mut p = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    let r = p.run();
    assert!(matches!(r, Err(Error::MemoryExceeded)), "got: {:?}", r);
}

#[test]
fn cite_render_denied_when_not_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_cite_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![Capability::CiteRender]),
        Grants::empty(),
        Quota::small(),
    );
    assert!(
        matches!(r, Err(Error::CapabilityDenied(Capability::CiteRender))),
        "unexpected: {:?}",
        r.err()
    );
}

#[test]
fn cite_render_succeeds_when_granted() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_cite_wat()).unwrap();
    let plugin = host.load(
        &bytes,
        &manifest_with(vec![Capability::CiteRender]),
        Grants::empty().allow(Capability::CiteRender),
        Quota::small(),
    );
    assert!(
        plugin.is_ok(),
        "expected granted load to succeed: {:?}",
        plugin.err()
    );
}

#[test]
fn unreachable_trap_falls_through_to_generic_trap() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_unreachable_wat()).unwrap();
    let mut p = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    let r = p.run();
    assert!(matches!(r, Err(Error::Trap(_))), "got: {:?}", r);
}

#[test]
fn unknown_interface_falls_through_to_trap() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(plugin_importing_unknown_interface_wat()).unwrap();
    let r = host.load(
        &bytes,
        &manifest_with(vec![]),
        Grants::empty(),
        Quota::small(),
    );
    // None of the 4 known interfaces match → classifier falls through.
    assert!(matches!(r, Err(Error::Trap(_))), "got: {:?}", r.err());
}

#[test]
fn fuel_exhausted_traps_on_infinite_loop() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(infinite_loop_plugin_wat()).unwrap();
    let mut plugin = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::new(10_000, 16),
        )
        .unwrap();
    let r = plugin.run();
    assert!(matches!(r, Err(Error::FuelExhausted)), "got: {:?}", r);
}

#[test]
fn doc_state_is_settable_and_readable() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let mut plugin = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    plugin.set_doc("hello world");
    assert_eq!(plugin.doc(), "hello world");
}

#[test]
fn ui_panels_starts_empty() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let plugin = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    assert!(plugin.ui_panels().is_empty());
}

#[test]
fn http_responder_installs_without_running() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let mut plugin = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    plugin.set_http_responder(|_, _| Ok("ok".into()));
    assert!(plugin.run().is_ok());
}

#[test]
fn two_plugins_have_isolated_doc_state() {
    let host = Host::new().unwrap();
    let bytes = wat::parse_str(empty_plugin_wat()).unwrap();
    let mut a = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    let mut b = host
        .load(
            &bytes,
            &manifest_with(vec![]),
            Grants::empty(),
            Quota::small(),
        )
        .unwrap();
    a.set_doc("aaa");
    b.set_doc("bbb");
    assert_eq!(a.doc(), "aaa");
    assert_eq!(b.doc(), "bbb");
}
