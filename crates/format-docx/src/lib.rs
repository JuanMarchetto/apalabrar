#![deny(unsafe_code)]
#![doc = "DOCX format I/O with lossless OOXML preservation pattern."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
