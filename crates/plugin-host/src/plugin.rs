//! [`Plugin`] — one loaded WASM Component instance.
//!
//! A [`Plugin`] owns its `Store` (and therefore its [`HostState`])
//! exclusively, which gives plugin-isolation for free: nothing in
//! one plugin's store is visible to another.
//!
//! Capability gating happens at link time in [`Plugin::instantiate`]:
//! only the imports for granted capabilities are registered with
//! the wasmtime [`Linker`](wasmtime::component::Linker). A plugin
//! that imports an ungranted interface fails at instantiation with
//! [`Error::CapabilityDenied`].

use crate::capability::{Capability, Grants};
use crate::error::{Error, Result};
use crate::quota::Quota;

/// The wasmtime store data. Holds the doc text the plugin reads /
/// writes, the registered UI panels, and an optional HTTP
/// responder for tests + future real adapter.
pub struct HostState {
    doc: String,
    ui_panels: Vec<(String, String)>,
    http_responder: HttpResponder,
    mem_pages: u32,
}

type HttpResponder = Box<dyn Fn(&str, &str) -> std::result::Result<String, String> + Send + Sync>;

impl wasmtime::ResourceLimiter for HostState {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        let cap = (self.mem_pages as usize) * 65536;
        Ok(desired <= cap)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// One loaded plugin. Drop it to release the wasmtime instance.
pub struct Plugin {
    store: wasmtime::Store<HostState>,
    instance: wasmtime::component::Instance,
}

const DOC_INTERFACE: &str = "apalabrar:editor/doc@0.1.0";
const DOC_MUT_INTERFACE: &str = "apalabrar:editor/doc-mut@0.1.0";
const UI_INTERFACE: &str = "apalabrar:editor/ui@0.1.0";
const NET_INTERFACE: &str = "apalabrar:editor/net@0.1.0";
const PLUGIN_INTERFACE: &str = "apalabrar:editor/plugin@0.1.0";

impl Plugin {
    pub(crate) fn instantiate(
        engine: &wasmtime::Engine,
        bytes: &[u8],
        grants: Grants,
        quota: Quota,
    ) -> Result<Self> {
        let component = wasmtime::component::Component::new(engine, bytes)
            .map_err(|e| Error::InvalidComponent(e.to_string()))?;

        let mut linker = wasmtime::component::Linker::new(engine);
        if grants.contains(Capability::DocRead) {
            let mut inst = linker
                .instance(DOC_INTERFACE)
                .expect("linker name uniqueness is enforced by capability gating");
            inst.func_wrap(
                "read",
                |ctx: wasmtime::StoreContextMut<'_, HostState>,
                 _: ()|
                 -> wasmtime::Result<(String,)> { Ok((ctx.data().doc.clone(),)) },
            )
            .expect("func_wrap is infallible for distinct function names");
        }
        if grants.contains(Capability::DocWrite) {
            let mut inst = linker
                .instance(DOC_MUT_INTERFACE)
                .expect("linker name uniqueness is enforced by capability gating");
            inst.func_wrap(
                "write",
                |mut ctx: wasmtime::StoreContextMut<'_, HostState>,
                 (text,): (String,)|
                 -> wasmtime::Result<()> {
                    ctx.data_mut().doc = text;
                    Ok(())
                },
            )
            .expect("func_wrap is infallible for distinct function names");
        }
        if grants.contains(Capability::UiPanel) {
            let mut inst = linker
                .instance(UI_INTERFACE)
                .expect("linker name uniqueness is enforced by capability gating");
            inst.func_wrap(
                "register-panel",
                |mut ctx: wasmtime::StoreContextMut<'_, HostState>,
                 (title, content): (String, String)|
                 -> wasmtime::Result<()> {
                    ctx.data_mut().ui_panels.push((title, content));
                    Ok(())
                },
            )
            .expect("func_wrap is infallible for distinct function names");
        }
        if grants.contains(Capability::NetHttp) {
            let mut inst = linker
                .instance(NET_INTERFACE)
                .expect("linker name uniqueness is enforced by capability gating");
            inst.func_wrap(
                "request",
                |ctx: wasmtime::StoreContextMut<'_, HostState>,
                 (url, method): (String, String)|
                 -> wasmtime::Result<(std::result::Result<String, String>,)> {
                    let r = (ctx.data().http_responder)(&url, &method);
                    Ok((r,))
                },
            )
            .expect("func_wrap is infallible for distinct function names");
        }

        let state = HostState {
            doc: String::new(),
            ui_panels: Vec::new(),
            http_responder: Box::new(|_, _| Err("no http responder configured".to_string())),
            mem_pages: quota.memory_pages,
        };
        let mut store = wasmtime::Store::new(engine, state);
        store
            .set_fuel(quota.fuel)
            .expect("engine config enables consume_fuel");
        store.limiter(|s| s);

        let instance = linker
            .instantiate(&mut store, &component)
            .map_err(|e| classify_link_error(&e, grants))?;

        Ok(Self { store, instance })
    }

    /// Invoke the plugin's `run` export.
    pub fn run(&mut self) -> Result<()> {
        let plugin_idx = self
            .instance
            .get_export(&mut self.store, None, PLUGIN_INTERFACE)
            .ok_or_else(|| Error::Trap("plugin interface not exported".into()))?;
        let run_idx = self
            .instance
            .get_export(&mut self.store, Some(&plugin_idx), "run")
            .ok_or_else(|| Error::Trap("plugin.run not exported".into()))?;
        let func = self
            .instance
            .get_func(&mut self.store, run_idx)
            .ok_or_else(|| Error::Trap("plugin.run is not a function".into()))?;
        let typed = func
            .typed::<(), ()>(&self.store)
            .map_err(|e| Error::Trap(e.to_string()))?;
        typed
            .call(&mut self.store, ())
            .map_err(classify_call_error)?;
        typed
            .post_return(&mut self.store)
            .expect("post_return is infallible after a successful call");
        Ok(())
    }

    /// Current document text.
    pub fn doc(&self) -> &str {
        &self.store.data().doc
    }

    /// Replace the document text (host-side; before/after a run).
    pub fn set_doc(&mut self, s: impl Into<String>) {
        self.store.data_mut().doc = s.into();
    }

    /// Panels the plugin registered via `ui.register-panel`.
    pub fn ui_panels(&self) -> &[(String, String)] {
        &self.store.data().ui_panels
    }

    /// Install an HTTP responder used for `net.request`. The
    /// production host wires this to a real HTTP client; tests
    /// install deterministic mocks.
    pub fn set_http_responder<F>(&mut self, f: F)
    where
        F: Fn(&str, &str) -> std::result::Result<String, String> + Send + Sync + 'static,
    {
        self.store.data_mut().http_responder = Box::new(f);
    }
}

fn classify_link_error(e: &anyhow::Error, grants: Grants) -> Error {
    let msg = e.to_string();
    for (interface, cap) in [
        (DOC_INTERFACE, Capability::DocRead),
        (DOC_MUT_INTERFACE, Capability::DocWrite),
        (UI_INTERFACE, Capability::UiPanel),
        (NET_INTERFACE, Capability::NetHttp),
    ] {
        if msg.contains(interface) && !grants.contains(cap) {
            return Error::CapabilityDenied(cap);
        }
    }
    if msg.contains("exceeds memory limit") {
        return Error::MemoryExceeded;
    }
    Error::Trap(msg)
}

fn classify_call_error(e: anyhow::Error) -> Error {
    if let Some(t) = e.downcast_ref::<wasmtime::Trap>() {
        if matches!(t, wasmtime::Trap::OutOfFuel) {
            return Error::FuelExhausted;
        }
        if matches!(t, wasmtime::Trap::MemoryOutOfBounds) {
            return Error::MemoryExceeded;
        }
    }
    Error::Trap(e.to_string())
}
