//! Capability + Grants — the security boundary.
//!
//! A `Capability` is a single named permission. A `Grants` set is
//! the *exact* set of capabilities the host hands a plugin at
//! instantiation. Capabilities not in the set are absent from the
//! wasmtime `Linker` — calling them traps at link time.
//!
//! Grants are constructed via `empty().allow(...)` chaining; this
//! is the only way to add a capability, which keeps the surface
//! audit-friendly (no `from_iter`, no `extend`, no `From<&[...]>`).

use std::collections::BTreeSet;

/// One named capability the host can grant to a plugin.
///
/// Names are kebab-case in serialized form (manifest JSON).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Read the document text.
    DocRead,
    /// Modify the document text.
    DocWrite,
    /// Register a UI panel with the editor shell.
    UiPanel,
    /// Issue HTTP requests through the host adapter.
    NetHttp,
    /// Render CSL citations / bibliographies via the host's
    /// citation engine. Phase 5.4 addition.
    CiteRender,
}

/// Set of capabilities granted to one plugin instance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants {
    set: BTreeSet<Capability>,
}

impl Grants {
    /// An empty grant set — the plugin gets nothing.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add `cap` to the set and return the updated builder.
    pub fn allow(mut self, cap: Capability) -> Self {
        self.set.insert(cap);
        self
    }

    /// Whether `cap` is in the set.
    pub fn contains(&self, cap: Capability) -> bool {
        self.set.contains(&cap)
    }

    /// Iterator over granted capabilities, in stable order.
    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        self.set.iter().copied()
    }
}
