#![deny(unsafe_code)]
#![doc = "Citation engine: citeproc-rs wrapper for CSL styles, bibliography rendering, and Zotero RPC."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
