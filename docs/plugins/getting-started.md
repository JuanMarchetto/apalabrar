# Your first Apalabrar plugin in 30 minutes

This walkthrough builds a working **word counter** plugin from scratch — the same one that ships in
[`examples/plugin-word-counter/`](../../examples/plugin-word-counter). It compiles to a WASM
Component, runs sandboxed inside the editor, and only sees the document text + a way to register a
UI panel.

By the end, you will have:

- A Cargo crate that compiles to `wasm32-unknown-unknown`.
- A WIT contract that declares the **smallest** capability surface the plugin needs.
- A Component (single `.wasm` blob) that the editor's plugin host can load.
- An understanding of how capability gating, manifests, and resource quotas hang together.

This is not throw-away code. The same machinery powers
[`examples/plugin-zotero/`](../../examples/plugin-zotero), which imports CSL items from BetterBibTeX
and renders bibliographies end-to-end. When you finish this guide you will know the SDK.

## What you need (5 minutes)

```bash
# Rust 1.85+ with the wasm32-unknown-unknown target.
rustup target add wasm32-unknown-unknown

# wasm-tools wraps a core wasm module into a Component.
cargo install wasm-tools
```

You also want the Apalabrar repo cloned somewhere; the `pnpm
install && cargo build` from the root
README is enough, but for this guide you only need `cargo`.

## The mental model (3 minutes)

A plugin is a **WASM Component** that **imports** a small set of host functions and **exports** a
`run` entry point. The host loads it with three knobs:

| Knob         | What it controls                                                                                        |
| ------------ | ------------------------------------------------------------------------------------------------------- |
| **Manifest** | What the plugin _says_ it needs — a JSON document with `id`, `version`, and a list of capability names. |
| **Grants**   | What the host actually _approves_ — a strict subset of the manifest, declared at load time.             |
| **Quota**    | How much fuel and memory the plugin can consume before it traps.                                        |

There are five capability families today: `doc-read`, `doc-write`, `ui-panel`, `net-http`,
`cite-render`. The host **only registers imports for capabilities in `Grants`**, so an ungranted
import is _absent from the linker_ — calling it fails at _instantiation_ with
`Error::CapabilityDenied`. There is no runtime check to forget; the security boundary is structural.

## Step 1 — scaffold the crate (2 minutes)

```bash
mkdir -p my-word-counter/src my-word-counter/wit
cd my-word-counter
```

Create `Cargo.toml`:

```toml
[package]
name = "my-word-counter"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "0.34"
```

The `cdylib` target tells Cargo to emit a single `.wasm` blob; the `wit-bindgen` crate generates the
Rust glue that talks to the host's WIT interfaces.

## Step 2 — declare the WIT contract (5 minutes)

Save this as `wit/editor.wit`:

```wit
package apalabrar:editor@0.1.0;

interface doc {
    /// Read the full document text.
    read: func() -> string;
}

interface ui {
    /// Register a panel with the editor shell.
    register-panel: func(title: string, content: string);
}

interface plugin {
    /// Plugin entry point — the host calls this once after load.
    run: func();
}

/// Minimal world for the word-counter plugin.
/// Only `doc.read` and `ui.register-panel` are needed, so this
/// world declares only those imports. Plugins should declare the
/// smallest world they need so users grant the smallest set of
/// capabilities.
world word-counter {
    import doc;
    import ui;
    export plugin;
}
```

A few details worth pausing on:

- **The package name** (`apalabrar:editor@0.1.0`) and the **interface signatures** must match what
  the host exposes. Verbatim. The host's WIT lives at
  [`crates/plugin-host/wit/editor.wit`](../../crates/plugin-host/wit/editor.wit); copy interfaces
  from there and keep their signatures intact.
- **The world name** (`word-counter`) is yours to choose. The imports inside the world declare which
  interfaces this plugin _requires_ — fewer is better. A plugin that imports the four it doesn't use
  will fail to load unless the user grants four unnecessary capabilities.
- **Strings cross the canonical-ABI boundary.** `wit-bindgen` generates the marshalling code; you
  don't write it.

## Step 3 — write the plugin (5 minutes)

Save this as `src/lib.rs`:

```rust
#![allow(unsafe_op_in_unsafe_fn)]

wit_bindgen::generate!({
    world: "word-counter",
    path: "wit/",
    generate_all,
});

use exports::apalabrar::editor::plugin::Guest;

struct Component;

impl Guest for Component {
    fn run() {
        let text = apalabrar::editor::doc::read();
        let count = text.split_whitespace().count();
        apalabrar::editor::ui::register_panel(
            "Word Counter",
            &format!("{count} words"),
        );
    }
}

export!(Component);
```

What `generate!` produces:

- A module tree mirroring the WIT — `apalabrar::editor::doc` (imported), `apalabrar::editor::ui`
  (imported), `exports::apalabrar::editor::plugin` (exported).
- Wrappers around each imported function that handle the canonical-ABI lift/lower for strings.
- A `Guest` trait you implement for the `plugin` interface, plus an `export!` macro that wires your
  impl to the wasm export symbols the Component Model expects.

## Step 4 — build the Component (2 minutes)

```bash
cargo build --release --target wasm32-unknown-unknown
wasm-tools component new \
    target/wasm32-unknown-unknown/release/my_word_counter.wasm \
    -o my-word-counter.component.wasm
```

`cargo build` produces a _core_ wasm module. `wasm-tools component
new` wraps it into a Component —
the format the host can load. You should now have `my-word-counter.component.wasm` next to your
`Cargo.toml`. It will be roughly 25–30 KB.

## Step 5 — load it from a host (5 minutes)

The host crate is `apalabrar-plugin-host` (in [`crates/plugin-host`](../../crates/plugin-host)).
Inside any test or binary that depends on it, the load + run dance is:

```rust
use apalabrar_plugin_host::{Capability, Grants, Host, Manifest, Quota};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("my-word-counter.component.wasm")?;

    let manifest = Manifest {
        id: "my-word-counter".into(),
        version: "0.1.0".into(),
        capabilities: vec![
            Capability::DocRead,
            Capability::UiPanel,
        ],
    };

    let grants = Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::UiPanel);

    let host = Host::new()?;
    let mut plugin = host.load(&bytes, &manifest, grants, Quota::small())?;

    plugin.set_doc("hola mundo desde Apalabrar");
    plugin.run()?;

    let panels = plugin.ui_panels();
    println!("{} = {}", panels[0].0, panels[0].1);
    // Word Counter = 4 words

    Ok(())
}
```

Three things to notice:

1. **The manifest is honest.** It lists exactly the two capabilities the plugin imports. The host
   refuses to grant anything not declared (`Error::Manifest`).
2. **`Grants` is the security boundary.** If you grant only `UiPanel`, the load fails with
   `Error::CapabilityDenied(Capability::DocRead)` — the linker never wires up the missing import, so
   instantiation fails before any plugin code runs.
3. **`Quota::small()`** caps the plugin at 1M fuel + 1 MiB of memory. Enough for the word counter,
   not enough for an infinite loop.

## Step 6 — test it (3 minutes)

This is the test you would write to drive the plugin end-to-end. Drop it in `tests/integration.rs`
of any host crate:

```rust
#[test]
fn word_counter_counts_three_words() {
    let bytes: &[u8] =
        include_bytes!("fixtures/my-word-counter.component.wasm");

    let manifest = Manifest {
        id: "my-word-counter".into(),
        version: "0.1.0".into(),
        capabilities: vec![Capability::DocRead, Capability::UiPanel],
    };
    let grants = Grants::empty()
        .allow(Capability::DocRead)
        .allow(Capability::UiPanel);

    let host = Host::new().unwrap();
    let mut plugin = host
        .load(bytes, &manifest, grants, Quota::small())
        .unwrap();

    plugin.set_doc("hola que tal");
    plugin.run().unwrap();
    assert_eq!(plugin.ui_panels()[0].1, "3 words");
}
```

The full test suite for the reference plugin lives at
[`crates/plugin-host/tests/end_to_end.rs`](../../crates/plugin-host/tests/end_to_end.rs) — it covers
the happy path, isolation between two instances, fuel exhaustion, memory limits, and capability
denial.

## What you just learned

You walked through every layer of the plugin SDK:

- **The WIT contract** — the typed interface between host and plugin. The plugin's world declares
  which interfaces are required imports.
- **`wit-bindgen`** — the code generator that turns a WIT world into Rust glue.
- **`wasm-tools component new`** — the wrapper that turns a core wasm module into a Component.
- **`Manifest` + `Grants`** — the manifest is the plugin's _request_; grants are the host's
  _approval_. Grants must be a subset of the manifest; the linker only registers granted imports.
- **`Quota`** — fuel + memory limits enforced at runtime.
- **`Plugin::run`** — the host calls the exported `plugin.run` function once per load.

## Next steps

- **Add `doc-write`.** Replace the document with the word count appended at the end. You will need
  to add `import doc-mut;` to your world, declare `Capability::DocWrite` in the manifest, and grant
  it at load time.
- **Add `net-http`.** Pull a quote from a public API and add it to the panel content. Mock it in
  tests via `Plugin::set_http_responder(...)`.
- **Read the Zotero plugin source.** It uses all five capabilities (including `cite-render`) and is
  the closest thing to a production-shaped marketplace plugin in the repo:
  [`examples/plugin-zotero/src/lib.rs`](../../examples/plugin-zotero/src/lib.rs).
- **Read the host's tests.** Every capability-denial path has a test in
  [`crates/plugin-host/tests/host_integration.rs`](../../crates/plugin-host/tests/host_integration.rs);
  the patterns there are how to test plugins of your own.

## Where to ask for help

- File an issue with the `[plugin-sdk]` tag.
- The full SDK reference (rustdoc) lives next to the host crate. Run
  `cargo doc --no-deps -p apalabrar-plugin-host --open`.
