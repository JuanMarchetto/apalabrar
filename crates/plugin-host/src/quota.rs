//! Resource quota — fuel + memory pages.
//!
//! Plugins are sandboxed not just by capability set but by hard
//! resource caps:
//! * **fuel** — wasmtime instruction-counter units consumed before
//!   trapping with [`crate::Error::FuelExhausted`].
//! * **memory_pages** — maximum WASM memory pages (64 KiB each)
//!   the plugin can grow to before [`crate::Error::MemoryExceeded`].

/// Per-plugin resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quota {
    /// Fuel units. 1 unit ≈ 1 wasm instruction.
    pub fuel: u64,
    /// Memory pages of 64 KiB each.
    pub memory_pages: u32,
}

impl Quota {
    /// Construct a quota explicitly.
    pub const fn new(fuel: u64, memory_pages: u32) -> Self {
        Self { fuel, memory_pages }
    }

    /// "Small" quota — appropriate for a quickly-loaded UI panel
    /// plugin. 1M fuel ≈ a few ms of compute, 16 pages = 1 MiB.
    pub const fn small() -> Self {
        Self::new(1_000_000, 16)
    }

    /// "Large" quota — for plugins that crunch through documents
    /// (citation render, format conversion). 100M fuel, 4 MiB.
    pub const fn large() -> Self {
        Self::new(100_000_000, 64)
    }
}

impl Default for Quota {
    fn default() -> Self {
        Self::small()
    }
}
