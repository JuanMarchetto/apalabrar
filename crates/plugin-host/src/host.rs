//! [`Host`] — owns the wasmtime [`Engine`](wasmtime::Engine) and
//! is the entry point for loading plugins.
//!
//! One [`Host`] is shared across many plugin loads; each load
//! produces a fresh [`Plugin`](crate::plugin::Plugin) with its
//! own `Store` (so plugins are isolated from each other).

use crate::capability::Grants;
use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::plugin::Plugin;
use crate::quota::Quota;

/// The plugin host. Holds the wasmtime engine.
pub struct Host {
    engine: wasmtime::Engine,
}

impl Host {
    /// Build a new host with Component Model + fuel metering enabled.
    pub fn new() -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        let engine =
            wasmtime::Engine::new(&config).map_err(|e| Error::InvalidComponent(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Load `bytes` (a WASM Component) under `manifest`, with the
    /// host approving exactly the capabilities in `grants` (which
    /// must be a subset of `manifest.capabilities`) and the
    /// resource limits in `quota`.
    pub fn load(
        &self,
        bytes: &[u8],
        manifest: &Manifest,
        grants: Grants,
        quota: Quota,
    ) -> Result<Plugin> {
        for cap in grants.iter() {
            if !manifest.capabilities.contains(&cap) {
                return Err(Error::Manifest(format!(
                    "grant {cap:?} not declared in manifest"
                )));
            }
        }
        Plugin::instantiate(&self.engine, bytes, grants, quota)
    }
}
