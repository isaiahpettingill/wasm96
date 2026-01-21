//! Wasmtime-backed runtime glue for wasm96-core.

use crate::{abi, state};
use anyhow::Result;
use wasmtime::{Extern, Linker, Store};
use wasmtime_wasi::WasiCtxBuilder;

/// Wasmtime Module type.
pub type Module = wasmtime::Module;
/// Wasmtime Instance type.
pub type Instance = wasmtime::Instance;

/// Host-side runtime container.
pub struct WasmtimeRuntime {
    pub engine: wasmtime::Engine,
    pub store: Store<state::Wasm96Ctx>,
    pub linker: Linker<state::Wasm96Ctx>,
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

        // Setup WASI with a host-backed directory that we sync from/to our VFS
        let mut wasi_builder = WasiCtxBuilder::new();

        // Inherit stdout/stderr for logging
        wasi_builder.inherit_stdout().inherit_stderr();

        // Create a temporary directory to act as the WASI root
        let temp_dir = tempfile::tempdir()?;

        // Sync our VFS into this directory if available
        {
            let gs = state::global().lock().unwrap();
            for (i, disk_opt) in gs.vfs.disks.iter().enumerate() {
                if let Some(disk) = disk_opt {
                    let disk_path = temp_dir.path().join(format!("disk{}", i));
                    std::fs::create_dir_all(&disk_path)?;
                    disk.extract_to_host(&disk_path)?;

                    // Open the directory for WASI.
                    // DISK0 is also mapped to "." for compatibility with games expecting root access.
                    if i == 0 {
                        wasi_builder.preopened_dir(
                            &disk_path,
                            ".",
                            wasmtime_wasi::DirPerms::all(),
                            wasmtime_wasi::FilePerms::all(),
                        )?;
                    }

                    wasi_builder.preopened_dir(
                        disk_path,
                        format!("disk{}", i),
                        wasmtime_wasi::DirPerms::all(),
                        wasmtime_wasi::FilePerms::all(),
                    )?;
                }
            }
        }

        let wasi = wasi_builder.build_p1();

        let store = Store::new(&engine, state::Wasm96Ctx { wasi, temp_dir });
        let mut linker = Linker::new(&engine);

        // Link WASI Preview 1 imports
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |ctx: &mut state::Wasm96Ctx| {
            &mut ctx.wasi
        })?;

        Ok(Self {
            engine,
            store,
            linker,
        })
    }

    /// Register the standard wasm96 host imports.
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

    /// Instantiate a module and resolve its entrypoints.
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

impl WasmtimeRuntime {
    /// Sync from the WASI directory back to the FAT VFS disk
    /// Sync from the WASI directory back to the FAT VFS disk
    pub fn sync_wasi_to_vfs(&mut self) -> Result<()> {
        let gs = state::global().lock().unwrap();
        let ctx = self.store.data();
        for (i, disk_opt) in gs.vfs.disks.iter().enumerate() {
            if let Some(disk) = disk_opt {
                let disk_path = ctx.temp_dir.path().join(format!("disk{}", i));
                if disk_path.exists() {
                    disk.pack_from_host(&disk_path)?;
                }
            }
        }
        Ok(())
    }
}
