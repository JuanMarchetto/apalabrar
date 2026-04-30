//! Test-fixture plugin that exercises every capability the host
//! supports. Used by `crates/plugin-host` to drive line coverage
//! of the doc-write and net.http import branches.

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
        let original = apalabrar::editor::doc::read();
        apalabrar::editor::doc_mut::write(&format!("ECHO[{original}]"));
        let resp = apalabrar::editor::net::request("https://example.test", "GET");
        let panel_body = match resp {
            Ok(body) => format!("net ok: {body}"),
            Err(e) => format!("net err: {e}"),
        };
        apalabrar::editor::ui::register_panel("Full Fixture", &panel_body);
    }
}

export!(Component);
