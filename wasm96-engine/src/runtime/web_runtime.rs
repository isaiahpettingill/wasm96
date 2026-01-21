//! Browser-backed runtime glue for wasm96-engine using WebAssembly JS API.

use crate::{abi, state};
use anyhow::{Result, anyhow};
use js_sys::{Object, Reflect, WebAssembly};
use wasm_bindgen::JsCast;

pub type Module = WebAssembly::Module;
pub type Instance = WebAssembly::Instance;

/// WebAssembly runtime using the browser's native API via js-sys.
pub struct WebRuntime {
    /// The import object passed to WebAssembly.instantiate.
    pub imports: Object,
}

impl super::Runtime for WebRuntime {
    type Module = Module;
    type Instance = Instance;

    fn new() -> Result<Self> {
        Ok(Self {
            imports: Object::new(),
        })
    }

    fn define_imports(&mut self) -> Result<()> {
        super::imports::define_web_imports(&self.imports)
    }

    fn compile_module(&self, bytes: &[u8]) -> Result<Self::Module> {
        let normalized = crate::loader::normalize_to_wasm(bytes)
            .map_err(|e| anyhow!("Normalization failed: {}", e))?;

        let uint8_array = unsafe { js_sys::Uint8Array::view(&normalized.wasm_bytes) };
        WebAssembly::Module::new(&uint8_array.into())
            .map_err(|e| anyhow!("WebAssembly compilation failed: {:?}", e))
    }

    fn instantiate(
        &mut self,
        module: &Self::Module,
    ) -> Result<(Self::Instance, abi::GuestEntrypoints)> {
        let instance = WebAssembly::Instance::new(module, &self.imports)
            .map_err(|e| anyhow!("WebAssembly instantiation failed: {:?}", e))?;

        // Extract memory
        let exports = instance.exports();
        let memory = Reflect::get(&exports, &"memory".into())
            .map_err(|e| anyhow!("Failed to get memory export: {:?}", e))?;

        if !memory.is_undefined() {
            let memory: WebAssembly::Memory = memory.unchecked_into();
            state::set_guest_memory_web(memory);
        }

        // Validate and resolve entrypoints
        let entrypoints = abi::GuestEntrypoints::resolve_web(&instance)?;

        Ok((instance, entrypoints))
    }
}
