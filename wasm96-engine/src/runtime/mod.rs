//! wasm96-engine runtime abstraction.
//!
//! Provides a common interface for running WASM modules either via Wasmtime (native)
//! or the browser's WebAssembly API (web).

pub mod imports;

#[cfg(not(target_arch = "wasm32"))]
pub mod runtime;

#[cfg(target_arch = "wasm32")]
pub mod web_runtime;

use crate::abi::GuestEntrypoints;
use anyhow::Result;

/// Abstract interface for a WebAssembly runtime.
pub trait Runtime {
    /// The type used for a compiled module.
    type Module;
    /// The type used for an instantiated module.
    type Instance;

    /// Create a new runtime instance.
    fn new() -> Result<Self>
    where
        Self: Sized;

    /// Register the standard wasm96 host imports.
    fn define_imports(&mut self) -> Result<()>;

    /// Compile raw WASM bytes into a module.
    fn compile_module(&self, bytes: &[u8]) -> Result<Self::Module>;

    /// Instantiate a module and resolve its entrypoints.
    fn instantiate(&mut self, module: &Self::Module) -> Result<(Self::Instance, GuestEntrypoints)>;
}

// Re-export the active backend's implementation and types.

#[cfg(not(target_arch = "wasm32"))]
pub use self::runtime::{Instance, Module, WasmtimeRuntime as BackendRuntime};

#[cfg(target_arch = "wasm32")]
pub use self::web_runtime::{Instance, Module, WebRuntime as BackendRuntime};
