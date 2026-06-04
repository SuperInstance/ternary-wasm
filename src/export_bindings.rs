//! Export bindings — wasm-bindgen style exports without the dependency.
//!
//! These are `#[no_mangle] extern "C"` functions that a WASM host can call.
//! On native target they just expose C ABI functions.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use crate::browser_engine::BrowserEngine;
use crate::cell_engine::CellEngine;
use crate::wasm_bridge::{Trit, Tryte, WasmBridge};
use crate::wasm_config::WasmConfig;

// We use a simple global state pattern for the WASM exports.
// In a real WASM module, these would be thread-local or singleton.
// For native testing, we use static mut (safe because single-threaded tests).

static mut ENGINE: Option<BrowserEngine> = None;
static mut CELLS: Option<CellEngine> = None;

fn get_engine() -> &'static mut BrowserEngine {
    unsafe { ENGINE.get_or_insert_with(|| BrowserEngine::new(WasmConfig::default())) }
}

fn get_cells() -> &'static mut CellEngine {
    unsafe { CELLS.get_or_insert_with(|| CellEngine::new(&WasmConfig::default())) }
}

/// Initialize the engine with a PRNG seed.
#[no_mangle]
pub extern "C" fn ternary_init(seed: u64) {
    unsafe {
        let config = WasmConfig::with_seed(seed);
        ENGINE = Some(BrowserEngine::new(config.clone()));
        CELLS = Some(CellEngine::new(&config));
    }
}

/// Spawn a new agent; returns its ID or -1 on failure.
#[no_mangle]
pub extern "C" fn ternary_spawn_agent() -> i32 {
    get_engine().spawn_agent().map(|id| id as i32).unwrap_or(-1)
}

/// Spawn an agent with a specific tryte value.
#[no_mangle]
pub extern "C" fn ternary_spawn_agent_value(low: i32, high: i32) -> i32 {
    // Pack two i16s into a tryte approximation
    let combined = (low & 0xFF) | ((high & 0xFF) << 8);
    let tryte = Tryte::from_i32(combined);
    get_engine().spawn_agent_with_state(tryte).map(|id| id as i32).unwrap_or(-1)
}

/// Run N evolution ticks.
#[no_mangle]
pub extern "C" fn ternary_tick(n: u32) {
    get_engine().tick_n(n);
}

/// Get an agent's state as a packed i32 (lower 16 bits = tryte value).
#[no_mangle]
pub extern "C" fn ternary_get_agent_state(id: u32) -> i32 {
    get_engine()
        .get_agent(id)
        .map(|a| a.state.to_i32())
        .unwrap_or(0)
}

/// Get the current tick.
#[no_mangle]
pub extern "C" fn ternary_current_tick() -> u64 {
    get_engine().current_tick()
}

/// Get alive agent count.
#[no_mangle]
pub extern "C" fn ternary_alive_count() -> u32 {
    get_engine().alive_count() as u32
}

/// Get total agent count.
#[no_mangle]
pub extern "C" fn ternary_agent_count() -> u32 {
    get_engine().agent_count() as u32
}

/// Get engine state hash.
#[no_mangle]
pub extern "C" fn ternary_state_hash() -> u64 {
    get_engine().state_hash()
}

/// Serialize trits into a buffer. Returns number of bytes written.
/// `trit_data` is a pointer to i8 array of `trit_count` trits.
/// `out_buf` is a pointer to u8 array of `out_len` bytes.
///
/// # Safety
/// Caller must ensure valid pointers and sufficient buffer size.
#[no_mangle]
pub unsafe extern "C" fn ternary_serialize_trits(
    trit_data: *const i8,
    trit_count: usize,
    out_buf: *mut u8,
    out_len: usize,
) -> usize {
    if trit_data.is_null() || out_buf.is_null() {
        return 0;
    }
    let trits: Vec<Trit> = (0..trit_count)
        .map(|i| Trit::from_i8(*trit_data.add(i)))
        .collect();
    let serialized = WasmBridge::serialize_trits(&trits);
    let copy_len = serialized.len().min(out_len);
    core::ptr::copy_nonoverlapping(serialized.as_ptr(), out_buf, copy_len);
    copy_len
}

/// Set a cell expression as a literal value.
#[no_mangle]
pub extern "C" fn ternary_cell_set_literal(row: u32, col: u32, value: i32) {
    get_cells().set_cell(row, col, crate::cell_engine::CellExpr::Literal(Tryte::from_i32(value)));
}

/// Evaluate all cells.
#[no_mangle]
pub extern "C" fn ternary_cell_evaluate_all() {
    get_cells().evaluate_all();
}

/// Get a cell's value.
#[no_mangle]
pub extern "C" fn ternary_cell_get_value(row: u32, col: u32) -> i32 {
    get_cells().get_value(row, col).to_i32()
}
