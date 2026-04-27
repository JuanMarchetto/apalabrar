#![deny(unsafe_code)]
#![doc = "Plugin host: capability-based sandbox for third-party WASM components."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
