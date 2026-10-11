use cani::{Engine, INITIAL, Player, State};

#[test]
fn opening_and_atomic_errors() {
    let state = State::default();
    assert_eq!(state.legal().len(), 232);
    assert_eq!(state.legal()[0], "0a1a2:1a1a2");
    assert_eq!(state.stone(0, 0), Some(Player::Black));
    assert_eq!(state.side_to_move(), Player::Black);
    let after = state.play("0a1a2:1b1b2").unwrap();
    assert_eq!(
        after.sfen(),
        "WWWW/4/B3/1BBB|WWWW/4/1B2/B1BB|WWWW/4/4/BBBB|WWWW/4/4/BBBB w 1"
    );
    assert_eq!(after.ply(), 1);
    assert!(state.play("0a1a2:2a1a2").is_err());
    assert_eq!(state.sfen(), INITIAL);
}

#[test]
fn pushes_capture_and_blocked_loss() {
    let prefix = "3W/4/B3/4|";
    let suffix = "|3W/4/4/B3|3W/4/4/B3 b 0";
    for board in ["4/4/BW2/4", "4/4/B1W1/4"] {
        let state = State::parse(&format!("{prefix}{board}{suffix}")).unwrap();
        let after = state.play("0a2c2:1a2c2").unwrap();
        assert_eq!(after.stone(1, 7), Some(Player::White));
        assert_eq!(after.stone(1, 6), Some(Player::Black));
    }
    let state = State::parse(&format!("{prefix}4/4/1BW1/4{suffix}")).unwrap();
    let won = state.play("0a2c2:1b2d2").unwrap();
    assert_eq!(won.winner(), Some(Player::Black));
    assert!(won.legal().is_empty());
    assert!(won.play("2d4d3:3d4d3").is_err());
    let blocked = State::parse("4/4/WW2/BW2|4/4/WW2/BW2|3W/4/4/B3|3W/4/4/B3 b 0").unwrap();
    assert_eq!(blocked.winner(), Some(Player::White));
}

#[test]
fn limit_is_not_loss_and_canonical_parser() {
    let state = State::parse(&INITIAL.replace(" b 0", " b 2147483647")).unwrap();
    assert!(state.legal().is_empty());
    assert_eq!(state.result(), "ongoing");
    for bad in [
        INITIAL.replace(" b 0", " b 00"),
        INITIAL.replace("/4/", "/22/"),
        format!("{INITIAL}\n"),
    ] {
        assert!(State::parse(&bad).is_err());
    }
}

#[test]
fn protocol_and_stream_recovery() {
    let mut engine = Engine::default();
    assert_eq!(engine.handle("state"), Err("handshake"));
    assert_eq!(engine.handle("hello 1").unwrap(), "ok usp 1");
    assert!(engine.handle("position invalid").is_err());
    assert_eq!(engine.state().sfen(), INITIAL);
    assert_eq!(engine.handle("go").unwrap(), "bestmove 0a1a2:1a1a2");
    assert_eq!(engine.state().sfen(), INITIAL);
    let mut input = format!("hello 1\r\n{}\nready\nstate", "x".repeat(10000))
        .into_bytes()
        .as_slice()
        .to_owned();
    let mut output = Vec::new();
    cani::run(&mut input.as_slice(), &mut output).unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "ok usp 1\nerror syntax\nok ready\n"
    );
    input.clear();
}

#[test]
fn ffi_buffer_retry_preserves_session() {
    use cani::ffi::*;
    unsafe {
        let engine = cani_engine_new();
        let request = b"hello 1";
        let mut needed = 0;
        assert_eq!(
            cani_engine_handle(
                engine,
                request.as_ptr(),
                request.len(),
                std::ptr::null_mut(),
                0,
                &mut needed
            ),
            BUFFER
        );
        let mut output = vec![0; needed];
        assert_eq!(
            cani_engine_handle(
                engine,
                request.as_ptr(),
                request.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut needed
            ),
            OK
        );
        assert_eq!(&output, b"ok usp 1\0");
        cani_engine_free(engine);
        let state = cani_position_new();
        assert_eq!(cani_position_parse(state, b"bad".as_ptr(), 3), INVALID);
        assert_eq!(cani_position_result(state), 0);
        cani_position_free(state);
    }
}
