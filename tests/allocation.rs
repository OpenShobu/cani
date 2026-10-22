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
        let sym = cani::symmetry::Symmetry::new(31).unwrap();
        std::hint::black_box(sym.position(initial));
        std::hint::black_box(sym.turn(buffer[0]));
        assert_eq!(
            cani::neural::PositionTensor::encode(initial)
                .decode()
                .unwrap(),
            initial
        );
        assert_eq!(
            cani::neural::ActionIndex::encode(buffer[0])
                .unwrap()
                .decode()
                .unwrap(),
            buffer[0]
        );
        std::hint::black_box(cani::symmetry::canonicalize(initial));
        std::hint::black_box(cani::neural::ActionMask::for_position(&initial));
        let mut actions = [cani::neural::ActionIndex(0); MAX_LEGAL_TURNS];
        std::hint::black_box(cani::neural::legal_actions_into(&initial, &mut actions).unwrap());
        let mut tensors = [cani::neural::PositionTensor::encode(initial); 2];
        std::hint::black_box(cani::neural::encode_batch_into(&[initial; 2], &mut tensors).unwrap());
        std::hint::black_box(cani::diagnostics::perft(&initial, 1, 233).unwrap());
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
            let mut canonical = native;
            let mut orientation = 0;
            assert_eq!(
                cani_state_canonical(&native, &mut canonical, &mut orientation),
                OK
            );
            let mut mask = [0u8; cani::neural::ACTION_MASK_BYTES];
            assert_eq!(
                cani_state_action_mask(&native, mask.as_mut_ptr(), mask.len(), &mut needed),
                OK
            );
            let mut sparse = [0u16; MAX_LEGAL_TURNS];
            assert_eq!(
                cani_state_legal_actions(&native, sparse.as_mut_ptr(), sparse.len(), &mut needed),
                OK
            );
            let mut batch = [TensorData::from(cani::neural::PositionTensor::encode(initial)); 2];
            assert_eq!(
                cani_neural_encode_batch(
                    [native; 2].as_ptr(),
                    2,
                    batch.as_mut_ptr(),
                    batch.len(),
                    &mut needed
                ),
                OK
            );
            let mut stats = PerftData::default();
            assert_eq!(cani_state_perft(&native, 1, 233, &mut stats), OK);
        }
    }
    TRACK.with(|c| c.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
}
