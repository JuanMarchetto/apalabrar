#![deny(unsafe_code)]
#![doc = "Document model: Loro CRDT wrapper. Owns the canonical state of a document."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
