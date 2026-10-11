//! Separate test binary: allocation counting is restricted to this test thread.
use cani::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    ops::ControlFlow,
};
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
struct Counting;
fn count() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
    }
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
#[test]
fn hot_paths_allocate_nothing() {
    let initial = State::default();
    let mut buffer = [Turn::default(); MAX_LEGAL_TURNS];
    COUNT.with(|c| c.set(0));
    TRACK.with(|c| c.set(true));
    for _ in 0..20 {
        let n = initial.legal_turns_into(&mut buffer).unwrap();
        std::hint::black_box(n);
        let _: ControlFlow<()> = initial.visit_legal_turns(|_| ControlFlow::Continue(()));
        std::hint::black_box(initial.outcome());
        let mut s = initial;
        let u = s.apply(buffer[0]).unwrap();
        std::hint::black_box(s.position_key());
        std::hint::black_box(s.state_key());
        std::hint::black_box(initial.played(buffer[0]).unwrap());
        s.undo(u).unwrap();
        assert_eq!(s, initial);
        // Native caller-owned value paths share the same allocation-free core.
        unsafe {
            use cani::ffi::*;
            let mut native = StateData::from(initial);
            let mut moves = [TurnData::default(); MAX_LEGAL_TURNS];
            let mut needed = 0;
            assert_eq!(
                cani_state_legal(&native, moves.as_mut_ptr(), moves.len(), &mut needed),
                OK
            );
            let mut undo = UndoData::default();
            assert_eq!(cani_state_apply(&mut native, &moves[0], &mut undo), OK);
            let (mut p, mut k, mut outcome) = (0, 0, 0);
            assert_eq!(cani_state_keys(&native, &mut p, &mut k), OK);
            assert_eq!(cani_state_outcome(&native, &mut outcome), OK);
            assert_eq!(cani_state_undo(&mut native, &undo), OK);
        }
    }
    TRACK.with(|c| c.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}
