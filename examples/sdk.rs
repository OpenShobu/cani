//! Small dependency-free measurements for bot utilities.
use cani::{MAX_LEGAL_TURNS, State, Turn, diagnostics::perft, neural::*, symmetry::canonicalize};
use std::{hint::black_box, time::Instant};
fn measure(name: &str, n: usize, mut f: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..n {
        f();
    }
    println!(
        "{name}: {:.1} ns/op",
        start.elapsed().as_nanos() as f64 / n as f64
    );
}
fn main() {
    let state = State::default();
    let mut turns = [Turn::default(); MAX_LEGAL_TURNS];
    let mut actions = [ActionIndex(0); MAX_LEGAL_TURNS];
    let batch = [state; 32];
    let mut tensors = [PositionTensor::encode(state); 32];
    println!(
        "State: {} bytes; mask: {} bytes; sparse capacity: {} bytes",
        std::mem::size_of::<State>(),
        std::mem::size_of::<ActionMask>(),
        std::mem::size_of_val(&actions)
    );
    measure("legal turns", 10000, || {
        let count = state.legal_turns_into(&mut turns).unwrap();
        black_box(&turns[..count]);
    });
    measure("canonical v1", 10000, || {
        black_box(canonicalize(black_box(state)));
    });
    measure("action mask", 10000, || {
        black_box(ActionMask::for_position(black_box(&state)));
    });
    measure("sparse actions", 10000, || {
        let count = legal_actions_into(&state, &mut actions).unwrap();
        black_box(&actions[..count]);
    });
    measure("32 tensors", 10000, || {
        encode_batch_into(&batch, &mut tensors).unwrap();
        black_box(&tensors);
    });
    let start = Instant::now();
    let stats = perft(&state, 2, 100000).unwrap();
    println!(
        "perft depth 2: {} leaves / {} nodes in {:?}",
        stats.leaves,
        stats.nodes,
        start.elapsed()
    );
}
