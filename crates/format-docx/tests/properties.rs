//! RED-phase property tests for Validation Gate 4.
//!
//! Two non-trivial invariants:
//!
//! 1. `prop_read_never_panics_on_random_bytes` — the reader must never
//!    panic; any non-OOXML or malformed byte stream produces an `Err`,
//!    not a panic. The cargo-fuzz target asserts the same property over
//!    a much larger random surface.
//! 2. `prop_write_never_panics_after_successful_read` — `write` must
//!    accept any model produced by `read` without panicking. We don't
//!    assert byte-equivalence here because the random inputs that
//!    survive `read` are usually structurally trivial; the corpus
//!    tests cover the equivalence criterion.

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_read_never_panics_on_random_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        let _ = apalabrar_format_docx::read(&bytes);
    }

    #[test]
    fn prop_write_never_panics_after_successful_read(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        if let Ok(model) = apalabrar_format_docx::read(&bytes) {
            let _ = apalabrar_format_docx::write(&model);
        }
    }
}
