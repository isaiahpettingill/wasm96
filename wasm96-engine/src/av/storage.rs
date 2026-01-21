// Needed for `alloc::` in this crate.
extern crate alloc;

use crate::state::global;

#[cfg(not(target_arch = "wasm32"))]
use wasmtime::Caller;

use super::utils::{guest_alloc, guest_free};

/// Save raw bytes to the persistent storage.
pub fn storage_save_raw(key: u64, data: &[u8]) {
    let mut s = global().lock().unwrap();
    s.storage.kv.insert(key, data.to_vec());
}

/// Load raw bytes from the persistent storage for web.
#[cfg(target_arch = "wasm32")]
pub fn storage_load_raw(key: u64) -> u64 {
    let s = global().lock().unwrap();
    let data = match s.storage.kv.get(&key) {
        Some(v) => v.clone(),
        None => return 0,
    };
    drop(s);

    let Some(dst_ptr) = guest_alloc(data.len() as u32) else {
        return 0;
    };

    let s = global().lock().unwrap();
    if let Some(memory) = &s.memory_web {
        let buffer = memory.buffer();
        let array = js_sys::Uint8Array::new_with_byte_offset_and_length(
            &buffer,
            dst_ptr,
            data.len() as u32,
        );
        array.copy_from(&data);
        return ((dst_ptr as u64) << 32) | (data.len() as u64);
    }

    guest_free(dst_ptr, data.len() as u32);
    0
}

/// Native Wasmtime wrapper for saving storage.
#[cfg(not(target_arch = "wasm32"))]
pub fn storage_save(env: &mut Caller<'_, crate::state::Wasm96Ctx>, key: u64, ptr: u32, len: u32) {
    if let Ok(data) = crate::av::utils::read_guest_bytes(env, ptr, len) {
        storage_save_raw(key, &data);
    }
}

/// Native Wasmtime wrapper for loading storage.
#[cfg(not(target_arch = "wasm32"))]
pub fn storage_load(env: &mut Caller<'_, crate::state::Wasm96Ctx>, key: u64) -> u64 {
    let s = global().lock().unwrap();
    let data = match s.storage.kv.get(&key) {
        Some(v) => v.clone(),
        None => return 0,
    };
    drop(s);

    let Some(dst_ptr) = guest_alloc(env, data.len() as u32) else {
        return 0;
    };

    let memory = env.get_export("memory").and_then(wasmtime::Extern::into_memory);

    if let Some(mem) = memory {
        if mem.write(&mut *env, dst_ptr as usize, &data).is_ok() {
            return ((dst_ptr as u64) << 32) | (data.len() as u64);
        }
    }

    guest_free(env, dst_ptr, data.len() as u32);
    0
}

/// Native Wasmtime wrapper for freeing storage.
#[cfg(not(target_arch = "wasm32"))]
pub fn storage_free(env: &mut Caller<'_, crate::state::Wasm96Ctx>, ptr: u32, len: u32) {
    guest_free(env, ptr, len);
}
