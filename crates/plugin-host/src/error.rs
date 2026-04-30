//! Error types for the plugin host.
//!
//! Every fallible code path in this crate funnels into [`Error`].
//! Variants are deliberately *narrow* (one per failure mode) so
//! callers can match precisely — security-boundary code must not
//! collapse "denied" into "trap" into "out-of-fuel".

use crate::capability::Capability;

/// Errors raised by the plugin host. See module-level docs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes did not parse as a valid WASM Component.
    #[error("invalid wasm component: {0}")]
    InvalidComponent(String),

    /// A capability was used by the plugin but not present in the
    /// `Grants` set the host instantiated it with.
    #[error("capability denied: {0:?}")]
    CapabilityDenied(Capability),

    /// The plugin executed more instructions than its fuel budget.
    #[error("fuel exhausted")]
    FuelExhausted,

    /// The plugin tried to grow memory beyond its page budget.
    #[error("memory limit exceeded")]
    MemoryExceeded,

    /// The manifest JSON failed to parse or referenced an unknown
    /// capability.
    #[error("manifest error: {0}")]
    Manifest(String),

    /// The plugin trapped during execution for any reason not
    /// covered by the more specific variants above.
    #[error("plugin trap: {0}")]
    Trap(String),
}

/// Crate-wide convenience alias.
pub type Result<T> = std::result::Result<T, Error>;
