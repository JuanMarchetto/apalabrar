#![deny(unsafe_code)]
#![doc = "HTML format I/O — html5ever for parse, controlled emit on save."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
