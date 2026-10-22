use cani::{diagnostics::*, neural::*, symmetry::*, *};
fn order(s: State) -> ([u16; 8], u8) {
    let b = s.bitboards();
    (
        std::array::from_fn(|i| {
            if i % 2 == 0 {
                b[i / 2].black
            } else {
                b[i / 2].white
            }
        }),
        s.side_to_move() as u8,
    )
}
#[test]
fn canonical_orbits_order_ties_and_actions() {
    assert_eq!(CANONICAL_VERSION, 1);
    let mut state = State::default();
    let initial = canonicalize(state);
    assert_eq!(initial.position, state);
    assert_eq!(initial.to_canonical.index(), 0);
    for step in 0..12 {
        let canonical = canonicalize(state);
        assert_eq!(canonical.to_canonical.position(state), canonical.position);
        assert_eq!(
            canonical
                .to_canonical
                .inverse()
                .position(canonical.position),
            state
        );
        assert_eq!(canonicalize(canonical.position).to_canonical.index(), 0);
        let legal = state.legal();
        let mask = ActionMask::for_position(&canonical.position);
        for id in 0..32 {
            let sym = Symmetry::new(id).unwrap();
            let transformed = sym.position(state);
            assert_eq!(canonicalize(transformed).position, canonical.position);
            assert!(order(canonical.position) <= order(transformed));
            if order(canonical.position) == order(transformed) {
                assert!(canonical.to_canonical.index() <= id);
            }
            // Pin the previous square mapping independently of the optimized table.
            for q in 0..16 {
                let (mut x, mut y) = (q % 4, q / 4);
                if id & 4 != 0 {
                    x = 3 - x;
                }
                for _ in 0..(id & 3) {
                    (x, y) = (3 - y, x);
                }
                assert_eq!(
                    sym.square(Square::new(q).unwrap()).index(),
                    (y * 4 + x) as usize
                );
            }
        }
        for text in &legal {
            let turn: Turn = text.parse().unwrap();
            let mapped = canonical.to_canonical.turn(turn);
            assert!(mask.contains(ActionIndex::encode(mapped).unwrap()));
            assert_eq!(canonical.to_canonical.inverse().turn(mapped), turn);
        }
        if legal.is_empty() {
            break;
        }
        state = state.play(&legal[(step * 29) % legal.len()]).unwrap();
    }
    let later =
        State::from_bitboards(State::default().bitboards(), Player::White, MAX_PLY).unwrap();
    assert_eq!(canonicalize(later).position.ply(), MAX_PLY);
}
#[test]
fn compact_masks_sparse_order_and_batch_atomicity() {
    let state = State::default();
    let legal = state.legal();
    let mask = ActionMask::for_position(&state);
    assert_eq!(std::mem::size_of::<ActionMask>(), 8192);
    assert_eq!(mask.count(), 232);
    assert_eq!(&mask.as_bytes()[..2], &[0x12, 0x12]);
    let mut actions = [ActionIndex(0xffff); MAX_LEGAL_TURNS];
    let count = legal_actions_into(&state, &mut actions).unwrap();
    assert_eq!(count, 232);
    let mut enabled = [false; ACTION_COUNT];
    for (text, index) in legal.iter().zip(actions.iter()) {
        assert_eq!(index.decode().unwrap().to_string(), *text);
        enabled[index.0 as usize] = true;
    }
    for (index, &expected) in enabled.iter().enumerate() {
        assert_eq!(mask.contains(ActionIndex(index as u16)), expected);
    }
    assert_eq!(actions[count], ActionIndex(0xffff));
    let mut short = [ActionIndex(0xffff); 231];
    assert_eq!(
        legal_actions_into(&state, &mut short).unwrap_err().required,
        232
    );
    assert_eq!(short, [ActionIndex(0xffff); 231]);
    let terminal =
        State::parse("4/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB w 0").unwrap();
    let limit = State::from_bitboards(state.bitboards(), Player::Black, MAX_PLY).unwrap();
    for s in [terminal, limit] {
        assert_eq!(ActionMask::for_position(&s).count(), 0);
        assert_eq!(legal_actions_into(&s, &mut []).unwrap(), 0);
    }
    let batch = [state, state.play(&legal[0]).unwrap(), limit];
    let mut sentinel = PositionTensor::encode(state);
    sentinel.version = 99;
    let mut output = [sentinel; 4];
    assert_eq!(
        encode_batch_into(&batch, &mut output[..2])
            .unwrap_err()
            .required,
        3
    );
    assert_eq!(output, [sentinel; 4]);
    assert_eq!(encode_batch_into(&batch, &mut output).unwrap(), 3);
    for (i, s) in batch.iter().enumerate() {
        assert_eq!(output[i].decode().unwrap(), *s);
    }
    assert_eq!(output[3], sentinel);
    assert_eq!(encode_batch_into(&[], &mut []).unwrap(), 0);
}
#[test]
fn perft_independent_fixtures_budgets_and_terminal_positions() {
    let state = State::default();
    assert_eq!(
        perft(&state, 0, 1).unwrap(),
        PerftStats {
            nodes: 1,
            leaves: 1
        }
    );
    assert_eq!(
        perft(&state, 1, 233).unwrap(),
        PerftStats {
            nodes: 233,
            leaves: 232
        }
    );
    // Independently computed by the vendored Python rules oracle.
    assert_eq!(
        perft(&state, 2, 50741).unwrap(),
        PerftStats {
            nodes: 50741,
            leaves: 50508
        }
    );
    assert_eq!(perft(&state, 2, 50740), Err(PerftError::NodeLimit));
    assert_eq!(perft(&state, 0, 0), Err(PerftError::NodeLimit));
    assert_eq!(perft(&state, 65, 1), Err(PerftError::DepthLimit));
    assert_eq!(perft(&state, 64, 1), Err(PerftError::NodeLimit));
    let capture = State::parse("3W/4/B3/4|4/4/1BW1/4|3W/4/4/B3|3W/4/4/B3 b 0").unwrap();
    assert_eq!(
        perft(&capture, 2, 521).unwrap(),
        PerftStats {
            nodes: 521,
            leaves: 496
        }
    );
    for s in [
        capture.play("0a2c2:1b2d2").unwrap(),
        State::from_bitboards(state.bitboards(), Player::White, MAX_PLY).unwrap(),
    ] {
        assert_eq!(perft(&s, 0, 1).unwrap().leaves, 1);
        assert_eq!(
            perft(&s, 1, 1).unwrap(),
            PerftStats {
                nodes: 1,
                leaves: 0
            }
        );
    }
}
#[test]
fn native_parity_and_atomic_failures() {
    use cani::ffi::*;
    unsafe {
        let state = State::default();
        let data = StateData::from(state);
        let mut out = data;
        let mut id = 99;
        assert_eq!(cani_state_canonical(&data, &mut out, &mut id), OK);
        assert_eq!(State::try_from(out).unwrap(), canonicalize(state).position);
        assert_eq!(id, 0);
        let mut bad = data;
        bad.reserved[0] = 1;
        id = 99;
        assert_eq!(cani_state_canonical(&bad, &mut out, &mut id), INVALID);
        assert_eq!(id, 99);
        assert_eq!(State::try_from(out).unwrap(), state);
        let mut bytes = [0xa5; ACTION_MASK_BYTES + 1];
        let mut needed = 0;
        assert_eq!(
            cani_state_action_mask(
                &data,
                bytes.as_mut_ptr(),
                ACTION_MASK_BYTES - 1,
                &mut needed
            ),
            BUFFER
        );
        assert_eq!(needed, ACTION_MASK_BYTES);
        assert_eq!(bytes, [0xa5; ACTION_MASK_BYTES + 1]);
        assert_eq!(
            cani_state_action_mask(&data, bytes.as_mut_ptr(), bytes.len(), &mut needed),
            OK
        );
        assert_eq!(
            &bytes[..ACTION_MASK_BYTES],
            ActionMask::for_position(&state).as_bytes()
        );
        assert_eq!(bytes[ACTION_MASK_BYTES], 0xa5);
        let mut indices = [0xffff; MAX_LEGAL_TURNS];
        assert_eq!(
            cani_state_legal_actions(&data, indices.as_mut_ptr(), 231, &mut needed),
            BUFFER
        );
        assert_eq!(needed, 232);
        assert_eq!(indices, [0xffff; MAX_LEGAL_TURNS]);
        assert_eq!(
            cani_state_legal_actions(&data, indices.as_mut_ptr(), indices.len(), &mut needed),
            OK
        );
        assert_eq!(needed, 232);
        for &index in &indices[..needed] {
            assert!(state.played(ActionIndex(index).decode().unwrap()).is_ok());
        }
        let sentinel = TensorData {
            version: 99,
            ply: 0,
            cells: [0xa5; 129],
            reserved: [0; 3],
        };
        let mut tensors = [sentinel; 3];
        let inputs = [data, bad];
        needed = 77;
        assert_eq!(
            cani_neural_encode_batch(inputs.as_ptr(), 2, tensors.as_mut_ptr(), 3, &mut needed),
            INVALID
        );
        assert_eq!(needed, 77);
        assert!(
            tensors
                .iter()
                .all(|t| t.version == 99 && t.cells == sentinel.cells)
        );
        assert_eq!(
            cani_neural_encode_batch(inputs.as_ptr(), 2, std::ptr::null_mut(), 0, &mut needed),
            BUFFER
        );
        assert_eq!(needed, 2);
        assert_eq!(
            cani_neural_encode_batch(
                [data, data].as_ptr(),
                2,
                tensors.as_mut_ptr(),
                3,
                &mut needed
            ),
            OK
        );
        assert_eq!(tensors[0].cells, PositionTensor::encode(state).cells);
        assert_eq!(tensors[2].version, 99);
        assert_eq!(
            cani_neural_encode_batch(std::ptr::null(), 0, std::ptr::null_mut(), 0, &mut needed),
            OK
        );
        assert_eq!(needed, 0);
        let mut stats = PerftData {
            nodes: 123,
            leaves: 456,
        };
        assert_eq!(cani_state_perft(&data, 2, 1, &mut stats), 17);
        assert_eq!((stats.nodes, stats.leaves), (123, 456));
        assert_eq!(cani_state_perft(&data, 256, 1, &mut stats), 18);
        assert_eq!(cani_state_perft(&data, 2, 50741, &mut stats), OK);
        assert_eq!((stats.nodes, stats.leaves), (50741, 50508));
    }
}
