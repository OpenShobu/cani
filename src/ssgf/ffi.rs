//! SSGF C ABI. Input/output pointers must be live, aligned and nonoverlapping.
use super::{Encoding, ErrorKind, GameRecord, Limits, RecordError};
use crate::ffi::{BUFFER, INVALID, OK, StateData, TurnData};
use std::{ptr, slice};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LimitsData {
    pub total_bytes: u64,
    pub metadata_bytes: u64,
    pub turns: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ErrorData {
    pub code: i32,
    pub reserved: u32,
    pub turn_index: u64,
}
fn code(kind: ErrorKind) -> i32 {
    match kind {
        ErrorKind::Json => 30,
        ErrorKind::Schema => 31,
        ErrorKind::Version => 32,
        ErrorKind::Profile => 33,
        ErrorKind::Limit => 34,
        ErrorKind::Binary => 35,
        ErrorKind::Metadata => 36,
        ErrorKind::Turn => 37,
        ErrorKind::IllegalTurn => 38,
        ErrorKind::Result => 39,
        ErrorKind::Index => 40,
    }
}
unsafe fn report(error: RecordError, out: *mut ErrorData) -> i32 {
    let status = code(error.kind);
    if !out.is_null() {
        unsafe {
            *out = ErrorData {
                code: status,
                reserved: 0,
                turn_index: error.turn_index.map_or(u64::MAX, |n| n as u64),
            };
        }
    }
    status
}
unsafe fn limits(data: *const LimitsData) -> Result<Limits, RecordError> {
    if data.is_null() {
        return Ok(Limits::default());
    }
    let d = unsafe { *data };
    Ok(Limits {
        total_bytes: usize::try_from(d.total_bytes)
            .map_err(|_| RecordError::new(ErrorKind::Limit, "limit exceeds platform size"))?,
        metadata_bytes: usize::try_from(d.metadata_bytes)
            .map_err(|_| RecordError::new(ErrorKind::Limit, "limit exceeds platform size"))?,
        turns: usize::try_from(d.turns)
            .map_err(|_| RecordError::new(ErrorKind::Limit, "limit exceeds platform size"))?,
    })
}
unsafe fn bytes(value: &[u8], out: *mut u8, cap: usize, needed: *mut usize, nul: bool) -> i32 {
    if needed.is_null() {
        return INVALID;
    }
    let n = value.len() + usize::from(nul);
    unsafe {
        *needed = n;
    }
    if cap < n {
        return BUFFER;
    }
    if n > 0 && out.is_null() {
        return INVALID;
    }
    if !value.is_empty() {
        unsafe {
            ptr::copy_nonoverlapping(value.as_ptr(), out, value.len());
        }
    }
    if nul {
        unsafe {
            *out.add(value.len()) = 0;
        }
    }
    OK
}

/// # Safety
/// Output must be writable and aligned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_default_limits(out: *mut LimitsData) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    let l = Limits::default();
    unsafe {
        *out = LimitsData {
            total_bytes: l.total_bytes as u64,
            metadata_bytes: l.metadata_bytes as u64,
            turns: l.turns as u64,
        };
    }
    OK
}
/// Auto-detect RSSGF/BSSGF. Success transfers a new owned handle to `out`.
/// # Safety
/// All buffers must be live, correctly sized, aligned and nonoverlapping.
/// Limits/error may be null. On failure out is preserved; on success error is
/// cleared. Free the resulting handle once with cani_ssgf_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_parse(
    data: *const u8,
    len: usize,
    limit: *const LimitsData,
    out: *mut *mut GameRecord,
    error: *mut ErrorData,
) -> i32 {
    if data.is_null() || out.is_null() {
        return INVALID;
    }
    let limits = match unsafe { limits(limit) } {
        Ok(l) => l,
        Err(e) => return unsafe { report(e, error) },
    };
    if len > limits.total_bytes {
        return unsafe {
            report(
                RecordError::new(ErrorKind::Limit, "total byte limit"),
                error,
            )
        };
    }
    match GameRecord::parse(unsafe { slice::from_raw_parts(data, len) }, limits) {
        Ok(r) => {
            unsafe {
                *out = Box::into_raw(Box::new(r));
            }
            if !error.is_null() {
                unsafe {
                    *error = ErrorData {
                        code: OK,
                        reserved: 0,
                        turn_index: u64::MAX,
                    };
                }
            }
            OK
        }
        Err(e) => unsafe { report(e, error) },
    }
}
/// # Safety
/// Null or one owned, live record handle. Release once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_free(record: *mut GameRecord) {
    if !record.is_null() {
        drop(unsafe { Box::from_raw(record) });
    }
}
/// encoding: 0 RSSGF, 1 BSSGF. Needed is bytes WITHOUT a NUL terminator.
/// # Safety
/// Live handle and nonoverlapping output/count/error/limit buffers required.
/// Out may be null when cap=0. BUFFER preserves output; error data is only
/// written on record errors. No file or game state is mutated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_write(
    record: *const GameRecord,
    encoding: u32,
    limit: *const LimitsData,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
    error: *mut ErrorData,
) -> i32 {
    if record.is_null() || needed.is_null() {
        return INVALID;
    }
    let encoding = match encoding {
        0 => Encoding::Readable,
        1 => Encoding::Binary,
        _ => return INVALID,
    };
    let limits = match unsafe { limits(limit) } {
        Ok(l) => l,
        Err(e) => return unsafe { report(e, error) },
    };
    match unsafe { &*record }.write(encoding, limits) {
        Ok(value) => unsafe { bytes(&value, out, cap, needed, false) },
        Err(e) => unsafe { report(e, error) },
    }
}
/// Common metadata JSON, no format or turns. Needed INCLUDES terminating NUL.
/// # Safety
/// Live handle, writable count and nonoverlapping output sized by cap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_metadata(
    record: *const GameRecord,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if record.is_null() {
        return INVALID;
    }
    match unsafe { &*record }.metadata_json() {
        Ok(v) => unsafe { bytes(&v, out, cap, needed, true) },
        Err(e) => code(e.kind),
    }
}
/// # Safety
/// Live record and writable/nonoverlapping count pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_turn_count(record: *const GameRecord, out: *mut usize) -> i32 {
    if record.is_null() || out.is_null() {
        return INVALID;
    }
    unsafe {
        *out = (&*record).turns().len();
    }
    OK
}
/// # Safety
/// Live record and writable/nonoverlapping turn storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_turn(
    record: *const GameRecord,
    index: usize,
    out: *mut TurnData,
) -> i32 {
    if record.is_null() || out.is_null() {
        return INVALID;
    }
    match unsafe { &*record }.turns().get(index) {
        Some(t) => {
            unsafe {
                *out = (*t).into();
            }
            OK
        }
        None => code(ErrorKind::Index),
    }
}
/// Position 0 is initial; position N follows turn N-1.
/// # Safety
/// Live record and writable/nonoverlapping position storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_position(
    record: *const GameRecord,
    index: usize,
    out: *mut StateData,
) -> i32 {
    if record.is_null() || out.is_null() {
        return INVALID;
    }
    match unsafe { &*record }.position_at(index) {
        Ok(s) => {
            unsafe {
                *out = s.into();
            }
            OK
        }
        Err(e) => code(e.kind),
    }
}
/// # Safety
/// Live record and writable/nonoverlapping position storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_final_position(
    record: *const GameRecord,
    out: *mut StateData,
) -> i32 {
    if record.is_null() || out.is_null() {
        return INVALID;
    }
    unsafe {
        *out = (&*record).final_position().into();
    }
    OK
}

use super::{GameBuilder, GameMetadata, GameResult, RecordedOutcome, Termination};
unsafe fn builder_result(result: Result<(), RecordError>, error: *mut ErrorData) -> i32 {
    match result {
        Err(e) => unsafe { report(e, error) },
        Ok(()) => {
            if !error.is_null() {
                unsafe {
                    *error = ErrorData {
                        code: OK,
                        reserved: 0,
                        turn_index: u64::MAX,
                    };
                }
            }
            OK
        }
    }
}
/// # Safety
/// Input/output pointers must be live, aligned and nonoverlapping. Free once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_new(
    initial: *const StateData,
    out: *mut *mut GameBuilder,
) -> i32 {
    if initial.is_null() || out.is_null() {
        return INVALID;
    }
    let Ok(state) = crate::State::try_from(unsafe { *initial }) else {
        return INVALID;
    };
    let builder = GameBuilder::new(state, GameMetadata::default()).expect("default metadata");
    unsafe {
        *out = Box::into_raw(Box::new(builder));
    }
    OK
}
/// # Safety
/// Pointers must be live, aligned and nonoverlapping. The record remains borrowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_from_record(
    record: *const GameRecord,
    out: *mut *mut GameBuilder,
) -> i32 {
    if record.is_null() || out.is_null() {
        return INVALID;
    }
    unsafe {
        *out = Box::into_raw(Box::new(GameBuilder::from_record(&*record)));
    }
    OK
}
/// # Safety
/// Handle must be null or uniquely owned and live. No concurrent access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_free(builder: *mut GameBuilder) {
    if !builder.is_null() {
        unsafe {
            drop(Box::from_raw(builder));
        }
    }
}
/// # Safety
/// Live, aligned, nonoverlapping pointers; optional error. Atomic on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_append(
    builder: *mut GameBuilder,
    turn: *const TurnData,
    error: *mut ErrorData,
) -> i32 {
    if builder.is_null() || turn.is_null() {
        return INVALID;
    }
    let Ok(turn) = crate::Turn::try_from(unsafe { *turn }) else {
        return INVALID;
    };
    unsafe { builder_result((*builder).append(turn), error) }
}
/// # Safety
/// Live, aligned, nonoverlapping pointers; optional error. Atomic on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_undo(
    builder: *mut GameBuilder,
    out: *mut TurnData,
    error: *mut ErrorData,
) -> i32 {
    if builder.is_null() || out.is_null() {
        return INVALID;
    }
    match unsafe { (*builder).undo() } {
        Err(e) => unsafe { report(e, error) },
        Ok(turn) => unsafe {
            *out = turn.into();
            builder_result(Ok(()), error)
        },
    }
}
/// # Safety
/// Live sized buffers and aligned pointers; optional error. UTF-8 without NUL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_comment(
    builder: *mut GameBuilder,
    index: usize,
    text: *const u8,
    len: usize,
    error: *mut ErrorData,
) -> i32 {
    if builder.is_null() || (len > 0 && text.is_null()) {
        return INVALID;
    }
    let data = if len == 0 {
        &[][..]
    } else {
        unsafe { slice::from_raw_parts(text, len) }
    };
    let Ok(text) = std::str::from_utf8(data) else {
        return INVALID;
    };
    unsafe { builder_result((*builder).comment(index, text.into()), error) }
}
/// # Safety
/// Live sized buffers and aligned pointers; optional error. Atomic on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_metadata(
    builder: *mut GameBuilder,
    data: *const u8,
    len: usize,
    error: *mut ErrorData,
) -> i32 {
    if builder.is_null() || (len > 0 && data.is_null()) {
        return INVALID;
    }
    let data = if len == 0 {
        &[][..]
    } else {
        unsafe { slice::from_raw_parts(data, len) }
    };
    let result = (|| {
        let value = serde_json::Value::Object(super::parse_object(data)?);
        let metadata: GameMetadata = serde_json::from_value(value)
            .map_err(|e| RecordError::new(ErrorKind::Schema, e.to_string()))?;
        unsafe { (*builder).set_metadata(metadata) }
    })();
    unsafe { builder_result(result, error) }
}
/// # Safety
/// Builder must be live and exclusive; optional writable error. Atomic on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_result(
    builder: *mut GameBuilder,
    outcome: u32,
    reason: u32,
    error: *mut ErrorData,
) -> i32 {
    if builder.is_null() {
        return INVALID;
    }
    let outcome = match outcome {
        0 => RecordedOutcome::Ongoing,
        1 => RecordedOutcome::Black,
        2 => RecordedOutcome::White,
        3 => RecordedOutcome::Draw,
        _ => return INVALID,
    };
    let reason = match reason {
        0 => Termination::Rules,
        1 => Termination::Resignation,
        2 => Termination::Timeout,
        3 => Termination::Agreement,
        4 => Termination::Adjudication,
        5 => Termination::Abandonment,
        6 => Termination::Unfinished,
        7 => Termination::CounterLimit,
        _ => return INVALID,
    };
    unsafe {
        builder_result(
            (*builder).declare_result(GameResult { outcome, reason }),
            error,
        )
    }
}
/// # Safety
/// Live aligned nonoverlapping pointers. Success transfers a new immutable record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_record(
    builder: *const GameBuilder,
    out: *mut *mut GameRecord,
    error: *mut ErrorData,
) -> i32 {
    if builder.is_null() || out.is_null() {
        return INVALID;
    }
    match unsafe { (*builder).record() } {
        Err(e) => unsafe { report(e, error) },
        Ok(record) => unsafe {
            *out = Box::into_raw(Box::new(record));
            builder_result(Ok(()), error)
        },
    }
}
/// # Safety
/// Live aligned nonoverlapping pointers; no concurrent builder mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_ssgf_builder_position(
    builder: *const GameBuilder,
    out: *mut StateData,
) -> i32 {
    if builder.is_null() || out.is_null() {
        return INVALID;
    }
    unsafe {
        *out = (*builder).position().into();
    }
    OK
}
