#![deny(unsafe_code)]
#![doc = "AI bridge: on-device (Phi-3 WebGPU, Whisper.cpp) and cloud BYO-key clients."]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable() {
        assert_eq!(VERSION, "0.0.0");
    }
}
