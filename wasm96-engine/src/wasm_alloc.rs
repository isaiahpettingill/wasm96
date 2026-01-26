use core::mem::MaybeUninit;
#[cfg(target_arch = "wasm32")]
use talc::TalckWasm;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static GLOBAL: TalckWasm = unsafe { TalckWasm::new_global() };
