#![deny(unsafe_code)]
#![doc = "ODT format I/O. Stub for v0. Implementation lands in v1."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
