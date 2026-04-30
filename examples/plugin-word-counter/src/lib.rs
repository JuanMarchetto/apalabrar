//! Reference plugin: word counter.
//!
//! Reads the document via `doc.read`, counts whitespace-separated
//! tokens, and surfaces the result via `ui.register-panel`.
//! Used by `crates/plugin-host`'s integration suite as the
//! end-to-end fixture.

// wit-bindgen 0.34 generates code that triggers Rust 2024's
// `unsafe_op_in_unsafe_fn` lint. Allow it inside the macro
// expansion so workspace clippy stays clean.
#![allow(unsafe_op_in_unsafe_fn)]

wit_bindgen::generate!({
    world: "apalabrar-plugin",
    path: "wit/",
    generate_all,
});

use exports::apalabrar::editor::plugin::Guest;

struct Component;

impl Guest for Component {
    fn run() {
        let text = apalabrar::editor::doc::read();
        let count = text.split_whitespace().count();
        apalabrar::editor::ui::register_panel("Word Counter", &format!("{count} words"));
    }
}

export!(Component);
