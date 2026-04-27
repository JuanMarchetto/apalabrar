//! Generate a small sample `.docx` for the Demo page.
//!
//! Usage:
//!   cargo run --release -p apalabrar-editor-core --example generate_fixture -- <output-path>
//!
//! This is a one-off helper; the produced file is committed to
//! `tests-corpus/demo/sample.docx` and consumed by the Vite-served Demo
//! page in `packages/app/src/pages/Demo.tsx`.

use std::env;
use std::fs;

use apalabrar_format_docx::serialize_text;

fn main() {
    let out = env::args()
        .nth(1)
        .expect("usage: cargo run --example generate_fixture -- <output-path>");
    let text = "Apalabrar — demo de Validation Gate 2.\n\n\
                Este documento se genera con docx-rs sobre WASM y se \
                proyecta a texto plano vía Loro CRDT.\n\n\
                Ñoño, año, mañana, día, sí: los acentos LATAM se preservan \
                a través de open_docx → apply_op → to_docx.";
    let bytes = serialize_text(text).expect("format-docx serialize");
    fs::write(&out, &bytes).expect("write fixture");
    println!("wrote {} ({} bytes)", out, bytes.len());
}
