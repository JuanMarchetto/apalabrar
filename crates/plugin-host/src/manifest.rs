//! Plugin manifest — declares id, version, and requested
//! capabilities. JSON-serialized.
//!
//! The host validates the manifest *before* loading the WASM
//! bytes; capabilities listed here gate which imports the linker
//! exposes. Unknown capability strings cause [`crate::Error::Manifest`].

use crate::capability::Capability;
use crate::error::{Error, Result};

/// A plugin's self-description.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    /// Stable id (kebab-case). Used as the plugin's identity in
    /// logs, marketplace, telemetry.
    pub id: String,
    /// Semver string. Not parsed here — the marketplace layer is
    /// responsible for compatibility checks.
    pub version: String,
    /// Capabilities the plugin requests. The host must approve a
    /// subset (or all) of these via `Grants` before instantiating.
    pub capabilities: Vec<Capability>,
}

impl Manifest {
    /// Parse a JSON manifest. Returns [`Error::Manifest`] on any
    /// malformed input or unknown capability name.
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(|e| Error::Manifest(e.to_string()))
    }
}
