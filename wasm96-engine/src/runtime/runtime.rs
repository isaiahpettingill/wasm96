//! Wasmtime-backed runtime glue for wasm96-core.

use crate::{abi, state};
use anyhow::Result;
use wasmtime::{Extern, Linker, Store};

/// Wasmtime Module type.
pub type Module = wasmtime::Module;
/// Wasmtime Instance type.
pub type Instance = wasmtime::Instance;

/// Host-side runtime container.
pub struct WasmtimeRuntime {
    pub engine: wasmtime::Engine,
    pub store: Store<()>,
    pub linker: Linker<()>,
}

impl super::Runtime for WasmtimeRuntime {
    type Module = Module;
    type Instance = Instance;

    /// Create a new Wasmtime runtime with a broad set of WebAssembly features enabled.
    fn new() -> Result<Self> {
        let mut cfg = wasmtime::Config::new();

        // Broadly supported/expected features for "modern" Wasm modules.
        cfg.wasm_multi_value(true);
        cfg.wasm_bulk_memory(true);
        cfg.wasm_reference_types(true);
        cfg.wasm_simd(true);

        // Additional proposal support.
        cfg.wasm_multi_memory(true);
        cfg.wasm_memory64(true);
        cfg.wasm_relaxed_simd(true);
        cfg.wasm_tail_call(true);
        cfg.wasm_function_references(true);
        cfg.wasm_gc(true);

        // Conservative but enabled, so guests using shared memories / atomics can at least load.
        cfg.wasm_threads(true);

        // Exception handling proposal is useful for some toolchains.
        cfg.wasm_exceptions(true);

        let engine = wasmtime::Engine::new(&cfg)?;
        let store = Store::new(&engine, ());
        let linker = Linker::new(&engine);

        Ok(Self {
            engine,
            store,
            linker,
        })
    }

    /// Define all host imports expected by guests under module `"env"`.
    fn define_imports(&mut self) -> Result<()> {
        super::imports::define_imports(&mut self.linker)
    }

    /// Compile raw WASM/WAT bytes into a module.
    fn compile_module(&self, bytes: &[u8]) -> Result<Self::Module> {
        let normalized = crate::loader::normalize_to_wasm(bytes)
            .map_err(|e| anyhow::anyhow!("Normalization failed: {}", e))?;
        wasmtime::Module::new(&self.engine, normalized.wasm_bytes.as_slice())
            .map_err(|e| anyhow::anyhow!("Compilation failed: {}", e))
    }

    /// Instantiate a module and wire up exports/memory.
    fn instantiate(
        &mut self,
        module: &Self::Module,
    ) -> Result<(Self::Instance, abi::GuestEntrypoints)> {
        let instance = self.linker.instantiate(&mut self.store, module)?;

        // Register memory in global state (best-effort).
        let memory = instance
            .get_export(&mut self.store, "memory")
            .and_then(Extern::into_memory);

        if let Some(mem) = memory.as_ref() {
            state::set_guest_memory_wasmtime(mem);
        }

        // Validate & resolve entrypoints via ABI helpers (single source of truth).
        abi::validate::required_exports_present_wasmtime(&instance, &mut self.store)
            .map_err(|e| anyhow::anyhow!("guest missing required export: {:?}", e))?;
        let entrypoints = abi::GuestEntrypoints::resolve_wasmtime(&instance, &mut self.store)?;

        Ok((instance, entrypoints))
    }
}
