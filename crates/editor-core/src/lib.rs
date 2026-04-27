#![deny(unsafe_code)]
#![doc = "Apalabrar editor core — umbrella public API exposing the WASM-callable surface."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_pinned_to_workspace_zero_zero_zero() {
        assert_eq!(VERSION, "0.0.0");
    }
}
