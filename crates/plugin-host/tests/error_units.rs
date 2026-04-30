//! Surface tests for [`Error`] display strings.

use apalabrar_plugin_host::{Capability, Error};

#[test]
fn invalid_component_displays_message() {
    let e = Error::InvalidComponent("malformed magic".into());
    assert_eq!(e.to_string(), "invalid wasm component: malformed magic");
}

#[test]
fn capability_denied_displays_capability() {
    let e = Error::CapabilityDenied(Capability::DocRead);
    assert!(e.to_string().contains("DocRead"));
    assert!(e.to_string().starts_with("capability denied:"));
}

#[test]
fn fuel_exhausted_displays() {
    assert_eq!(Error::FuelExhausted.to_string(), "fuel exhausted");
}

#[test]
fn memory_exceeded_displays() {
    assert_eq!(Error::MemoryExceeded.to_string(), "memory limit exceeded");
}

#[test]
fn manifest_displays_message() {
    let e = Error::Manifest("bad".into());
    assert_eq!(e.to_string(), "manifest error: bad");
}

#[test]
fn trap_displays_message() {
    let e = Error::Trap("divide by zero".into());
    assert_eq!(e.to_string(), "plugin trap: divide by zero");
}
