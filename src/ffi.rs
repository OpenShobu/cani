//! C ABI. All pointers must refer to live, correctly aligned objects/buffers.
//! Handles have one owner and must not be accessed concurrently during mutation.
use crate::{
    Board, BoardId, Engine, MAX_LEGAL_TURNS, Move, MoveError, MoveSelector, Outcome, Player,
    Square, State, Turn, Undo,
};
use std::{ptr, slice, str};

pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const ILLEGAL: i32 = 2;
pub const BUFFER: i32 = 3;
pub const SELECTOR: i32 = 4;
pub const UNDO_MISMATCH: i32 = 5;

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct BoardData {
    pub black: u16,
    pub white: u16,
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct StateData {
    pub boards: [BoardData; 4],
    pub ply: u32,
    pub side: u8,
    pub reserved: [u8; 3],
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct MoveData {
    pub board: u8,
    pub from: u8,
    pub to: u8,
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TurnData {
    pub passive: MoveData,
    pub aggressive: MoveData,
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct UndoData {
    pub before: StateData,
    pub after: StateData,
}
impl From<State> for StateData {
    fn from(s: State) -> Self {
        Self {
            boards: s.bitboards().map(|b| BoardData {
                black: b.black,
                white: b.white,
            }),
            ply: s.ply(),
            side: s.side_to_move() as u8,
            reserved: [0; 3],
        }
    }
}
impl TryFrom<StateData> for State {
    type Error = i32;
    fn try_from(s: StateData) -> Result<Self, i32> {
        let side = match s.side {
            0 => Player::Black,
            1 => Player::White,
            _ => return Err(INVALID),
        };
        if s.reserved != [0; 3] {
            return Err(INVALID);
        }
        Self::from_bitboards(
            s.boards.map(|b| Board {
                black: b.black,
                white: b.white,
            }),
            side,
            s.ply,
        )
        .map_err(|_| INVALID)
    }
}
impl From<Turn> for TurnData {
    fn from(t: Turn) -> Self {
        fn leg(m: Move) -> MoveData {
            MoveData {
                board: m.board.index() as u8,
                from: m.from.index() as u8,
                to: m.to.index() as u8,
            }
        }
        Self {
            passive: leg(t.passive),
            aggressive: leg(t.aggressive),
        }
    }
}
impl TryFrom<TurnData> for Turn {
    type Error = i32;
    fn try_from(t: TurnData) -> Result<Self, i32> {
        fn leg(m: MoveData) -> Result<Move, i32> {
            Ok(Move {
                board: BoardId::new(m.board).map_err(|_| INVALID)?,
                from: Square::new(m.from).map_err(|_| INVALID)?,
                to: Square::new(m.to).map_err(|_| INVALID)?,
            })
        }
        Ok(Self {
            passive: leg(t.passive)?,
            aggressive: leg(t.aggressive)?,
        })
    }
}
pub type SelectorCallback = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    *const StateData,
    *const TurnData,
    usize,
    *mut TurnData,
) -> i32;
#[derive(Clone, Copy, Default)]
pub struct NativeSelector {
    callback: Option<SelectorCallback>,
    context: *mut std::ffi::c_void,
}
impl MoveSelector for NativeSelector {
    fn select(&mut self, position: &State, legal: &[Turn]) -> Turn {
        let Some(callback) = self.callback else {
            return legal[0];
        };
        let state = StateData::from(*position);
        let mut turns = [TurnData::default(); MAX_LEGAL_TURNS];
        for (out, t) in turns.iter_mut().zip(legal) {
            *out = (*t).into();
        }
        let mut selected = TurnData::default();
        let status = unsafe {
            callback(
                self.context,
                &state,
                turns.as_ptr(),
                legal.len(),
                &mut selected,
            )
        };
        if status != OK {
            return Turn::default();
        }
        Turn::try_from(selected).unwrap_or_default()
    }
}
pub type NativeEngine = Engine<NativeSelector>;

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
pub extern "C" fn cani_engine_new() -> *mut NativeEngine {
    Box::into_raw(Box::new(Engine::with_selector(NativeSelector::default())))
}

/// # Safety
/// `engine` must be null or an owned live engine handle; release once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_engine_free(engine: *mut NativeEngine) {
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
    engine: *mut NativeEngine,
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
    // The bestmove length is fixed for nonterminal positions. Determine capacity
    // before calling user code: queries/retries must not repeat selector work.
    if current.negotiated && request == "go" {
        let has_move = current
            .state
            .visit_legal_turns(|_| std::ops::ControlFlow::Break(()))
            .is_break();
        let required = if has_move { 21 } else { 14 };
        if needed.is_null() {
            return INVALID;
        }
        unsafe {
            *needed = required;
        }
        if cap < required {
            return BUFFER;
        }
        if out.is_null() {
            return INVALID;
        }
    }
    let mut next = current.clone();
    let reply = match next.handle(request) {
        Ok(reply) => reply,
        Err("selector") => return SELECTOR,
        Err(e) => format!("error {e}"),
    };
    let code = unsafe { write_text(&reply, out, cap, needed) };
    if code == OK {
        unsafe {
            *engine = next;
        }
    }
    code
}

/// Register or clear a selector. A null callback restores first-legal selection.
/// # Safety
/// The context must outlive registration; callbacks must not unwind, retain
/// borrowed pointers, or reenter/mutate/free this session. Calls are serialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_engine_set_selector(
    engine: *mut NativeEngine,
    callback: Option<SelectorCallback>,
    context: *mut std::ffi::c_void,
) -> i32 {
    if engine.is_null() {
        return INVALID;
    }
    unsafe {
        (*engine).selector = NativeSelector { callback, context };
    }
    OK
}
fn move_code(e: MoveError) -> i32 {
    match e {
        MoveError::Malformed => INVALID,
        MoveError::Geometry => 10,
        MoveError::WrongHomeboard => 11,
        MoveError::BoardColor => 12,
        MoveError::WrongStone => 13,
        MoveError::Blocked => 14,
        MoveError::Terminal => 15,
        MoveError::CounterLimit => 16,
    }
}
unsafe fn read_state(data: *const StateData) -> Result<State, i32> {
    if data.is_null() {
        Err(INVALID)
    } else {
        State::try_from(unsafe { *data })
    }
}

/// # Safety
/// Output must point to writable, aligned storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_init(out: *mut StateData) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    unsafe {
        *out = State::default().into();
    }
    OK
}
/// # Safety
/// Input must point to readable, aligned storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_validate(data: *const StateData) -> i32 {
    unsafe { read_state(data) }.map_or_else(|e| e, |_| OK)
}
/// # Safety
/// Buffers must be live and sized. Output cannot overlap input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_parse(out: *mut StateData, data: *const u8, len: usize) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    let state = unsafe { text(data, len) }.and_then(|s| State::parse_sfen(s).map_err(|_| INVALID));
    match state {
        Ok(s) => {
            unsafe {
                *out = s.into();
            }
            OK
        }
        Err(e) => e,
    }
}
/// # Safety
/// Live, nonoverlapping input and output buffers are required.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_sfen(
    data: *const StateData,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match unsafe { read_state(data) } {
        Ok(s) => unsafe { write_text(&s.sfen(), out, cap, needed) },
        Err(e) => e,
    }
}
/// # Safety
/// Input must be readable; output must be writable and nonoverlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_stone(
    data: *const StateData,
    board: u8,
    square: u8,
    out: *mut i32,
) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    let (Ok(s), Ok(b), Ok(q)) = (
        unsafe { read_state(data) },
        BoardId::new(board),
        Square::new(square),
    ) else {
        return INVALID;
    };
    unsafe {
        *out = match s.at(b, q) {
            None => 0,
            Some(Player::Black) => 1,
            Some(Player::White) => 2,
        };
    }
    OK
}
/// # Safety
/// Position and count must be live; output must hold cap turns. Null output is
/// permitted when cap is zero. All buffers must be nonoverlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_legal(
    data: *const StateData,
    out: *mut TurnData,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if needed.is_null() {
        return INVALID;
    }
    let state = match unsafe { read_state(data) } {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut turns = [Turn::default(); MAX_LEGAL_TURNS];
    let count = state.legal_turns_into(&mut turns).expect("bounded turns");
    unsafe {
        *needed = count;
    }
    if cap < count {
        return BUFFER;
    }
    if count > 0 && out.is_null() {
        return INVALID;
    }
    for (i, t) in turns[..count].iter().enumerate() {
        unsafe {
            *out.add(i) = (*t).into();
        }
    }
    OK
}
/// # Safety
/// All pointers must be live, aligned, and nonoverlapping. Failure preserves
/// position and token storage. Tokens must be used in reverse application order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_apply(
    data: *mut StateData,
    turn: *const TurnData,
    undo: *mut UndoData,
) -> i32 {
    if turn.is_null() || undo.is_null() {
        return INVALID;
    }
    let mut state = match unsafe { read_state(data) } {
        Ok(s) => s,
        Err(e) => return e,
    };
    let turn = match Turn::try_from(unsafe { *turn }) {
        Ok(t) => t,
        Err(e) => return e,
    };
    match state.apply(turn) {
        Ok(token) => {
            unsafe {
                *undo = UndoData {
                    before: token.before.into(),
                    after: token.after.into(),
                };
                *data = state.into();
            }
            OK
        }
        Err(e) => move_code(e),
    }
}
/// # Safety
/// All pointers must be live and aligned. Output may equal input but cannot
/// overlap turn storage; other partial overlap is unsupported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_played(
    data: *const StateData,
    turn: *const TurnData,
    out: *mut StateData,
) -> i32 {
    if turn.is_null() || out.is_null() {
        return INVALID;
    }
    let state = match unsafe { read_state(data) } {
        Ok(s) => s,
        Err(e) => return e,
    };
    let turn = match Turn::try_from(unsafe { *turn }) {
        Ok(t) => t,
        Err(e) => return e,
    };
    match state.played(turn) {
        Ok(s) => {
            unsafe {
                *out = s.into();
            }
            OK
        }
        Err(e) => move_code(e),
    }
}
/// # Safety
/// Data and token must be readable/aligned, data writable, and nonoverlapping.
/// Tokens are validated structurally and for matching resulting state, not
/// authenticated; callers must retain unmodified tokens produced by apply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_undo(data: *mut StateData, token: *const UndoData) -> i32 {
    if token.is_null() {
        return INVALID;
    }
    let mut state = match unsafe { read_state(data) } {
        Ok(s) => s,
        Err(e) => return e,
    };
    let token = unsafe { *token };
    let (Ok(before), Ok(after)) = (State::try_from(token.before), State::try_from(token.after))
    else {
        return INVALID;
    };
    match state.undo(Undo { before, after }) {
        Ok(()) => {
            unsafe {
                *data = state.into();
            }
            OK
        }
        Err(_) => UNDO_MISMATCH,
    }
}
/// # Safety
/// Position must be readable and outcome writable, aligned and nonoverlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_outcome(data: *const StateData, out: *mut i32) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    match unsafe { read_state(data) } {
        Ok(s) => {
            unsafe {
                *out = match s.outcome() {
                    Outcome::Ongoing => 0,
                    Outcome::Win(Player::Black) => 1,
                    Outcome::Win(Player::White) => 2,
                };
            }
            OK
        }
        Err(e) => e,
    }
}
/// # Safety
/// Position readable, keys writable/aligned; all storage nonoverlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_state_keys(
    data: *const StateData,
    position_key: *mut u64,
    state_key: *mut u64,
) -> i32 {
    if position_key.is_null() || state_key.is_null() {
        return INVALID;
    }
    match unsafe { read_state(data) } {
        Ok(s) => {
            unsafe {
                *position_key = s.position_key().0;
                *state_key = s.state_key().0;
            }
            OK
        }
        Err(e) => e,
    }
}
/// # Safety
/// Live handle and writable, nonoverlapping output storage required.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_export(state: *const State, out: *mut StateData) -> i32 {
    if state.is_null() || out.is_null() {
        return INVALID;
    }
    unsafe {
        *out = (*state).into();
    }
    OK
}
/// # Safety
/// Live handle and readable, nonoverlapping input storage required.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_position_import(state: *mut State, data: *const StateData) -> i32 {
    if state.is_null() {
        return INVALID;
    }
    match unsafe { read_state(data) } {
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
/// Input and output live, sized, nonoverlapping. Needed includes NUL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_turn_format(
    turn: *const TurnData,
    out: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if turn.is_null() {
        return INVALID;
    }
    match Turn::try_from(unsafe { *turn }) {
        Ok(t) => unsafe { write_text(&t.to_string(), out, cap, needed) },
        Err(e) => e,
    }
}
/// # Safety
/// Input and output live, sized and nonoverlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cani_turn_parse(data: *const u8, len: usize, out: *mut TurnData) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    let turn = unsafe { text(data, len) }.and_then(|s| s.parse::<Turn>().map_err(|_| INVALID));
    match turn {
        Ok(t) => {
            unsafe {
                *out = t.into();
            }
            OK
        }
        Err(e) => e,
    }
}
