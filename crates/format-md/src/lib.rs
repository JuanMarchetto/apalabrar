#![deny(unsafe_code)]
#![doc = "Markdown format I/O — CommonMark + GFM tables/footnotes/tasklists."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
