use cani::*;
use std::ops::ControlFlow;

fn turn(s: &str) -> Turn {
    s.parse().unwrap()
}

#[test]
fn typed_construction_and_notation() {
    assert!(BoardId::new(4).is_err());
    assert!(Square::new(16).is_err());
    assert_eq!(Square::new(15).unwrap().to_string(), "d4");
    for text in ["0a1a2:1b1b2", "2d4b2:3c3a1"] {
        assert_eq!(turn(text).to_string(), text);
    }
    for text in ["0a1a2:1b1b2x", "4a1a2:1b1b2", "0é1a2:1b1b2", "0a0a2:1b1b2"] {
        assert_eq!(text.parse::<Turn>(), Err(MoveError::Malformed));
    }
    let mut boards = State::default().bitboards();
    boards[0].white |= 1;
    assert_eq!(
        State::from_bitboards(boards, Player::Black, 0),
        Err(ValidationError::Overlap)
    );
    boards = State::default().bitboards();
    boards[0].black |= 16;
    assert_eq!(
        State::from_bitboards(boards, Player::Black, 0),
        Err(ValidationError::PieceCount)
    );
    boards = State::default().bitboards();
    boards[0].black = 0;
    boards[1].white = 0;
    assert_eq!(
        State::from_bitboards(boards, Player::Black, 0),
        Err(ValidationError::ContradictoryWinners)
    );
    assert!(std::mem::size_of::<State>() <= 24);
    fn copy<T: Copy>() {}
    copy::<State>();
}

#[test]
fn generation_order_capacity_and_early_exit() {
    let state = State::default();
    let sentinel = turn("3d4d4:3d4d4");
    let mut small = [sentinel; 231];
    assert_eq!(
        state.legal_turns_into(&mut small),
        Err(BufferTooSmall { required: 232 })
    );
    assert!(small.iter().all(|t| *t == sentinel));
    let mut turns = [Turn::default(); MAX_LEGAL_TURNS];
    let n = state.legal_turns_into(&mut turns).unwrap();
    assert_eq!(n, 232);
    let strings: Vec<_> = turns[..n].iter().map(ToString::to_string).collect();
    assert!(strings.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(strings, state.legal());
    let mut calls = 0;
    assert_eq!(
        state.visit_legal_turns(|t| {
            calls += 1;
            ControlFlow::Break(t)
        }),
        ControlFlow::Break(turns[0])
    );
    assert_eq!(calls, 1);
    for t in &turns[..n] {
        assert!(state.played(*t).is_ok());
    }
}

#[test]
fn apply_undo_stack_and_atomic_errors() {
    let initial = State::default();
    let mut state = initial;
    let a = turn("0a1a2:1b1b2");
    let u1 = state.apply(a).unwrap();
    assert_eq!(state, initial.played(a).unwrap());
    let middle = state;
    let b = turn("2a4a3:3b4b3");
    let u2 = state.apply(b).unwrap();
    state.undo(u2).unwrap();
    assert_eq!(state, middle);
    state.undo(u1).unwrap();
    assert_eq!(state, initial);
    let token = state.apply(a).unwrap();
    state.apply(b).unwrap();
    let child = state;
    assert_eq!(state.undo(token), Err(UndoMismatch));
    assert_eq!(state, child);
    for (text, error) in [
        ("0a1a1:1b1b1", MoveError::Geometry),
        ("2a1a2:1b1b2", MoveError::WrongHomeboard),
        ("0a1a2:2b1b2", MoveError::BoardColor),
        ("0a2a3:1b1b2", MoveError::WrongStone),
        ("0a1b1:1a1b1", MoveError::Blocked),
    ] {
        let mut s = initial;
        assert_eq!(s.apply(turn(text)).unwrap_err(), error);
        assert_eq!(s, initial);
    }
    let max = State::from_bitboards(initial.bitboards(), Player::Black, MAX_PLY).unwrap();
    assert_eq!(max.outcome(), Outcome::Ongoing);
    assert_eq!(max.played(a), Err(MoveError::CounterLimit));
    let terminal = State::parse("4/4/3B/4|3W/4/4/B3|3W/4/4/B3|3W/4/4/B3 w 1").unwrap();
    assert_eq!(terminal.outcome(), Outcome::Win(Player::Black));
    assert_eq!(terminal.played(a), Err(MoveError::Terminal));

    // Undo must restore an enemy captured off-board, including terminal state.
    let mut capture = State::parse("3W/4/B3/4|4/4/1BW1/4|3W/4/4/B3|3W/4/4/B3 b 0").unwrap();
    let before = capture;
    let token = capture.apply(turn("0a2c2:1b2d2")).unwrap();
    assert_eq!(capture.outcome(), Outcome::Win(Player::Black));
    capture.undo(token).unwrap();
    assert_eq!(capture, before);
    assert_eq!(capture.state_key(), before.state_key());
}

#[test]
fn selectors_and_stream_failure() {
    let mut engine = Engine::with_selector(|_: &State, legal: &[Turn]| legal[legal.len() - 1]);
    engine.handle("hello 1").unwrap();
    let expected = engine.state().legal().last().unwrap().clone();
    assert_eq!(
        engine.handle_checked("go").unwrap(),
        format!("bestmove {expected}")
    );
    assert_eq!(*engine.state(), State::default());
    let mut invalid = Engine::with_selector(|_: &State, _: &[Turn]| Turn::default());
    invalid.handle("hello 1").unwrap();
    assert_eq!(
        invalid.handle_checked("go"),
        Err(SessionError::Selection(SelectionError))
    );
    assert_eq!(*invalid.state(), State::default());
    let mut output = Vec::new();
    let error = run_with_selector(
        &mut b"hello 1\ngo\n".as_slice(),
        &mut output,
        |_: &State, _: &[Turn]| Turn::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"ok usp 1\n");
    let mut bypass = Engine::with_selector(|_: &State, _: &[Turn]| panic!("terminal callback"));
    bypass.handle("hello 1").unwrap();
    bypass
        .handle("position 4/4/3B/4|3W/4/4/B3|3W/4/4/B3|3W/4/4/B3 w 1")
        .unwrap();
    assert_eq!(bypass.handle("go").unwrap(), "bestmove none");
    bypass
        .handle(&format!(
            "position {}",
            INITIAL.replace(" b 0", " b 2147483647")
        ))
        .unwrap();
    assert_eq!(bypass.handle("go").unwrap(), "bestmove none");
}

#[test]
fn deterministic_keys() {
    let s = State::default();
    assert_eq!(s.position_key(), PositionKey(0xcf87b888d934b52e));
    assert_eq!(s.state_key(), StateKey(0x5afa352fa525047c));
    let after = s.played(turn("0a1a2:1b1b2")).unwrap();
    assert_eq!(after.position_key(), PositionKey(0x89762bf94edc2c16));
    assert_eq!(after.state_key(), StateKey(0xeb73baf62b36d8d9));
    let later = State::from_bitboards(s.bitboards(), Player::Black, 12).unwrap();
    let white = State::from_bitboards(s.bitboards(), Player::White, 0).unwrap();
    assert_eq!(s.position_key(), later.position_key());
    assert_ne!(s.state_key(), later.state_key());
    assert_ne!(s.position_key(), white.position_key());
    assert_ne!(
        s.position_key(),
        s.played(turn("0a1a2:1b1b2")).unwrap().position_key()
    );
    assert_eq!(State::parse(&s.sfen()).unwrap().state_key(), s.state_key());
}
