//! Apalabrar plugin host — Phase 5.3.
//!
//! Loads WASM Components, sandboxes them with capability grants
//! and resource quotas, and surfaces a small Rust-native API so
//! the editor shell can talk to plugins without ever calling
//! `wasmtime` directly.
//!
//! ## Capability model
//! Four named capabilities — see [`Capability`]. The host's
//! [`Grants`] is the *exact* set of capabilities approved at load
//! time. Capabilities not in the set are absent from the linker;
//! a plugin importing them fails at instantiation with
//! [`Error::CapabilityDenied`].
//!
//! ## Resource quota
//! Each plugin runs under a [`Quota`] that caps fuel (instructions)
//! and memory pages. Exhaustion surfaces as
//! [`Error::FuelExhausted`] / [`Error::MemoryExceeded`].
//!
//! ## Manifest
//! Plugins ship a [`Manifest`] declaring requested capabilities;
//! the host refuses to grant anything not declared.

#![deny(unsafe_code)]

mod capability;
mod error;
mod host;
mod manifest;
mod plugin;
mod quota;

pub use capability::{Capability, Grants};
pub use error::{Error, Result};
pub use host::Host;
pub use manifest::Manifest;
pub use plugin::Plugin;
pub use quota::Quota;

/// Crate version string (filled by Cargo).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
