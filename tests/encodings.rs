use cani::{neural::*, symmetry::Symmetry, *};
#[test]
fn symmetry_commutes_with_rules_and_inverse() {
    assert!(Symmetry::new(32).is_err());
    let mut state = State::default();
    for step in 0..12 {
        let legal = state.legal();
        for id in 0..32 {
            let sym = Symmetry::new(id).unwrap();
            let transformed = sym.position(state);
            assert_eq!(sym.inverse().position(transformed), state);
            assert_eq!(transformed.ply(), state.ply());
            let expected = match state.outcome() {
                Outcome::Ongoing => Outcome::Ongoing,
                Outcome::Win(p) => Outcome::Win(sym.player(p)),
            };
            assert_eq!(transformed.outcome(), expected);
            let mut mapped: Vec<_> = legal
                .iter()
                .map(|t| sym.turn(t.parse().unwrap()).to_string())
                .collect();
            mapped.sort();
            assert_eq!(mapped, transformed.legal());
            for text in &legal {
                let turn: Turn = text.parse().unwrap();
                assert_eq!(sym.inverse().turn(sym.turn(turn)), turn);
                assert_eq!(
                    sym.position(state.played(turn).unwrap()),
                    transformed.played(sym.turn(turn)).unwrap()
                );
            }
        }
        if legal.is_empty() {
            break;
        }
        state = state.play(&legal[(step * 37) % legal.len()]).unwrap();
    }
    for text in [
        "3W/4/B3/4|4/4/1BW1/4|3W/4/4/B3|3W/4/4/B3 b 0",
        "4/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB w 0",
    ] {
        let state = State::parse(text).unwrap();
        for id in 0..32 {
            let sym = Symmetry::new(id).unwrap();
            let expected = match state.outcome() {
                Outcome::Win(p) => Outcome::Win(sym.player(p)),
                o => o,
            };
            assert_eq!(sym.position(state).outcome(), expected);
        }
    }
}
#[test]
fn neural_fixtures_roundtrips_and_rejections() {
    let state = State::default();
    let tensor = PositionTensor::encode(state);
    assert_eq!(tensor.version, 1);
    assert_eq!(tensor.ply, 0);
    assert_eq!(tensor.cells[128], 0);
    for b in 0..4 {
        for q in 0..16 {
            assert_eq!(tensor.cells[b * 32 + q], u8::from(q < 4));
            assert_eq!(tensor.cells[b * 32 + 16 + q], u8::from(q >= 12));
        }
    }
    assert_eq!(tensor.decode().unwrap(), state);
    let mut invalid = tensor;
    invalid.version = 2;
    assert!(invalid.decode().is_err());
    invalid = tensor;
    invalid.cells[128] = 2;
    assert!(invalid.decode().is_err());
    invalid = tensor;
    invalid.cells[16] = 1;
    assert!(invalid.decode().is_err());
    invalid = tensor;
    invalid.ply = MAX_PLY + 1;
    assert!(invalid.decode().is_err());
    let mut state = state;
    for step in 0..50 {
        assert_eq!(PositionTensor::encode(state).decode().unwrap(), state);
        let legal = state.legal();
        if legal.is_empty() {
            break;
        }
        state = state.play(&legal[(step * 19) % legal.len()]).unwrap();
    }
    assert_eq!(
        ActionIndex::encode("0a1a2:1b1b2".parse().unwrap())
            .unwrap()
            .0,
        0x0104
    );
    assert_eq!(
        ActionIndex::encode("2a4a3:3b4b3".parse().unwrap())
            .unwrap()
            .0,
        0x4dce
    );
    let mut count = 0;
    for index in 0..=u16::MAX {
        if let Ok(turn) = ActionIndex(index).decode() {
            assert_eq!(ActionIndex::encode(turn).unwrap().0, index);
            count += 1;
        }
    }
    assert_eq!(count, 19520);
    assert!(ActionIndex::encode(Turn::default()).is_err());
}
#[test]
fn native_encoding_agrees_and_failures_are_atomic() {
    use cani::ffi::*;
    unsafe {
        let state = State::default();
        let data = StateData::from(state);
        let mut out = StateData::default();
        let mut tensor = TensorData {
            version: 0,
            ply: 0,
            cells: [0; 129],
            reserved: [0; 3],
        };
        assert_eq!(cani_neural_encode(&data, &mut tensor), OK);
        assert_eq!(tensor.cells, PositionTensor::encode(state).cells);
        assert_eq!(cani_neural_decode(&tensor, &mut out), OK);
        assert_eq!(State::try_from(out).unwrap(), state);
        tensor.reserved[0] = 1;
        assert_eq!(cani_neural_decode(&tensor, &mut out), INVALID);
        assert_eq!(State::try_from(out).unwrap(), state);
        for id in 0..32 {
            assert_eq!(cani_state_transform(&data, id, &mut out), OK);
            assert_eq!(
                State::try_from(out).unwrap(),
                Symmetry::new(id).unwrap().position(state)
            );
        }
        assert_eq!(cani_state_transform(&data, 32, &mut out), INVALID);
        let input = TurnData::from("0a1a2:1b1b2".parse::<Turn>().unwrap());
        let mut index = 0;
        assert_eq!(cani_action_encode(&input, &mut index), OK);
        assert_eq!(index, 0x0104);
        let mut turn = TurnData::default();
        assert_eq!(cani_action_decode(index, &mut turn), OK);
        assert_eq!(
            Turn::try_from(turn).unwrap(),
            Turn::try_from(input).unwrap()
        );
        let original = Turn::try_from(turn).unwrap();
        assert_eq!(cani_action_decode(0xffff, &mut turn), 10);
        assert_eq!(Turn::try_from(turn).unwrap(), original);
    }
}
