//! C ABI. All pointers must refer to live, correctly aligned objects/buffers.
//! Handles have one owner and must not be accessed concurrently during mutation.
use crate::{Engine, State};
use std::{ptr, slice, str};

pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const ILLEGAL: i32 = 2;
pub const BUFFER: i32 = 3;

unsafe fn text<'a>(data: *const u8, len: usize) -> Result<&'a str, i32> {
    if data.is_null() || len > 4096 {
        return Err(INVALID);
    }
    str::from_utf8(unsafe { slice::from_raw_parts(data, len) }).map_err(|_| INVALID)
}
unsafe fn write_text(value: &str, out: *mut u8, capacity: usize, needed: *mut usize) -> i32 {
    if needed.is_null() {
        return INVALID;
    }
    unsafe {
        *needed = value.len() + 1;
    }
    if capacity < value.len() + 1 {
        return BUFFER;
    }
    if out.is_null() {
        return INVALID;
    }
    unsafe {
        ptr::copy_nonoverlapping(value.as_ptr(), out, value.len());
        *out.add(value.len()) = 0;
    }
    OK
}

#[unsafe(no_mangle)]
pub extern "C" fn cani_position_new() -> *mut State {
    Box::into_raw(Box::new(State::default()))
}

/// # Safety
/// `state` must be null or an owned live handle from this library; release once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_free(state: *mut State) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// Pointers must be live, nonoverlapping buffers/handles of the specified sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_parse(
    state: *mut State,
    data: *const u8,
    len: usize,
) -> i32 {
    if state.is_null() {
        return INVALID;
    }
    let parsed = unsafe { text(data, len) }.and_then(|s| State::parse(s).map_err(|_| INVALID));
    match parsed {
        Ok(s) => {
            unsafe {
                *state = s;
            }
            OK
        }
        Err(e) => e,
    }
}

/// # Safety
/// Pointers must be live, nonoverlapping buffers/handles of the specified sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_play(state: *mut State, data: *const u8, len: usize) -> i32 {
    if state.is_null() {
        return INVALID;
    }
    let played = unsafe { text(data, len) }.and_then(|s| {
        unsafe { &*state }
            .play(s)
            .map_err(|e| if e == "illegal" { ILLEGAL } else { INVALID })
    });
    match played {
        Ok(s) => {
            unsafe {
                *state = s;
            }
            OK
        }
        Err(e) => e,
    }
}

/// # Safety
/// Live handle and output buffers are required. `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_sfen(
    state: *const State,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if state.is_null() {
        return INVALID;
    }
    unsafe { write_text(&(&*state).sfen(), out, cap, needed) }
}

/// # Safety
/// Live handle and output buffers are required. `needed` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_legal(
    state: *const State,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if state.is_null() {
        return INVALID;
    }
    unsafe { write_text(&(&*state).legal().join(" "), out, cap, needed) }
}

/// Returns 0 ongoing, 1 Black wins, 2 White wins, -1 invalid handle.
/// # Safety
/// `state` must be null or a live position handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_result(state: *const State) -> i32 {
    if state.is_null() {
        return -1;
    }
    match unsafe { &*state }.result().as_str() {
        "b" => 1,
        "w" => 2,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cani_engine_new() -> *mut Engine {
    Box::into_raw(Box::new(Engine::default()))
}

/// # Safety
/// `engine` must be null or an owned live engine handle; release once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_engine_free(engine: *mut Engine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

/// Process one USP request without its newline. A buffer of 12294 bytes suffices.
/// Insufficient output capacity preserves the session; retry the same request.
/// Protocol errors are returned as response text with status OK.
/// # Safety
/// All handles and nonoverlapping input/output buffers must be live and sized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_engine_handle(
    engine: *mut Engine,
    data: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if engine.is_null() {
        return INVALID;
    }
    let request = match unsafe { text(data, len) } {
        Ok(s) => s,
        Err(e) => return e,
    };
    let current = unsafe { &*engine };
    let mut next = Engine {
        state: current.state.clone(),
        negotiated: current.negotiated,
    };
    let reply = next
        .handle(request)
        .unwrap_or_else(|e| format!("error {e}"));
    let code = unsafe { write_text(&reply, out, cap, needed) };
    if code == OK {
        unsafe {
            *engine = next;
        }
    }
    code
}
