//! Fuzz target for `apalabrar_format_docx::read`.
//!
//! Asserts the same property the proptest version asserts at 64 cases:
//! the reader must NEVER panic on arbitrary bytes. The cargo-fuzz harness
//! drives this with libFuzzer's coverage-guided generator and explores a
//! much larger surface than a property test ever could in CI time.
//!
//! Run locally:
//!
//!   cargo +nightly fuzz run docx_read -- -max_total_time=600
//!
//! Crashes / OOMs / hangs are persisted under
//! `crates/format-docx/fuzz/artifacts/docx_read/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

use apalabrar_format_docx::read;

fuzz_target!(|data: &[u8]| {
    let _ = read(data);
});
