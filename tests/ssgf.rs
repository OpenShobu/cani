#![cfg(feature = "ssgf")]
use cani::{MAX_PLY, Outcome, Player, State, ssgf::*};
use serde_json::{Value, json};
const OPENING: &[u8] = include_bytes!("fixtures/ssgf/opening.rssgf");
fn parse(bytes: &[u8]) -> GameRecord {
    GameRecord::parse(bytes, Limits::default()).unwrap()
}
fn modified(f: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut v: Value = serde_json::from_slice(OPENING).unwrap();
    f(&mut v);
    serde_json::to_vec(&v).unwrap()
}
#[test]
fn formats_roundtrip_metadata_and_replay() {
    let record = parse(include_bytes!("../examples/ssgf/tournament.rssgf"));
    assert_eq!(
        parse(include_bytes!("../examples/ssgf/tournament.bssgf")),
        record
    );
    assert_eq!(
        record
            .metadata()
            .players
            .as_ref()
            .unwrap()
            .black
            .as_ref()
            .unwrap()
            .name
            .as_deref(),
        Some("山田 太郎")
    );
    assert_eq!(
        record
            .metadata()
            .players
            .as_ref()
            .unwrap()
            .white
            .as_ref()
            .unwrap()
            .ratings[0]
            .value,
        1750.5
    );
    assert_eq!(record.metadata().comments[1].turn, 1);
    assert_eq!(
        record.metadata().extensions["example.club"]["division"],
        "open"
    );
    for encoding in [Encoding::Readable, Encoding::Binary] {
        let bytes = record.write(encoding, Limits::default()).unwrap();
        assert_eq!(parse(&bytes), record);
        assert_eq!(record.write(encoding, Limits::default()).unwrap(), bytes);
    }
    let text = record.write(Encoding::Readable, Limits::default()).unwrap();
    assert_eq!(text.last(), Some(&b'\n'));
    assert!(
        std::str::from_utf8(&text)
            .unwrap()
            .starts_with("{\n  \"comment\":")
    );
    let positions: Vec<_> = record.replay().collect();
    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0], State::default());
    assert_eq!(positions[2], record.final_position());
    assert_eq!(
        record.position_at(1).unwrap(),
        State::default().play("0a1a2:1b1b2").unwrap()
    );
    assert_eq!(record.position_at(3).unwrap_err().kind, ErrorKind::Index);
    let mut replay = record.replay();
    assert_eq!(replay.len(), 3);
    replay.next();
    assert_eq!(replay.len(), 2);
}
#[test]
fn binary_golden_and_exhaustive_geometry() {
    let r = parse(OPENING);
    let expected = include_bytes!("fixtures/ssgf/opening.bssgf");
    assert_eq!(
        r.write(Encoding::Binary, Limits::default()).unwrap(),
        expected
    );
    assert_eq!(parse(expected), r);
    assert_eq!(pack_turn("0a1a2:1b1b2".parse().unwrap()).unwrap(), 0x0104);
    assert_eq!(pack_turn("2a4a3:3b4b3".parse().unwrap()).unwrap(), 0x4dce);
    let mut count = 0;
    for bits in 0..=u16::MAX {
        if let Ok(t) = unpack_turn(bits) {
            assert_eq!(pack_turn(t).unwrap(), bits);
            count += 1;
        }
    }
    assert_eq!(count, 19_520);
    assert!(pack_turn("0a1b3:1a1b3".parse().unwrap()).is_err());
    assert!(pack_turn("0a1a2:1b1b3".parse().unwrap()).is_err());
}
#[test]
fn result_validation_and_study_starts() {
    let win = parse(include_bytes!("fixtures/ssgf/win.rssgf"));
    assert_eq!(win.final_position().outcome(), Outcome::Win(Player::Black));
    assert_eq!(win.replay().len(), 2);
    for (reason, outcome) in [
        ("resignation", "w"),
        ("timeout", "b"),
        ("agreement", "draw"),
        ("adjudication", "draw"),
        ("abandonment", "ongoing"),
        ("unfinished", "ongoing"),
    ] {
        let bytes = modified(|v| v["result"] = json!({"outcome":outcome,"reason":reason}));
        assert!(GameRecord::parse(&bytes, Limits::default()).is_ok());
    }
    for result in [
        json!({"outcome":"b","reason":"rules"}),
        json!({"outcome":"ongoing","reason":"resignation"}),
        json!({"outcome":"b","reason":"agreement"}),
        json!({"outcome":"draw","reason":"unfinished"}),
        json!({"outcome":"ongoing","reason":"counter_limit"}),
    ] {
        let bytes = modified(|v| v["result"] = result);
        assert_eq!(
            GameRecord::parse(&bytes, Limits::default())
                .unwrap_err()
                .kind,
            ErrorKind::Result
        );
    }
    let mut v: Value = serde_json::from_slice(include_bytes!("fixtures/ssgf/win.rssgf")).unwrap();
    v["result"] = json!({"outcome":"b","reason":"resignation"});
    assert_eq!(
        GameRecord::parse(&serde_json::to_vec(&v).unwrap(), Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Result
    );
    let m = GameMetadata {
        start_sfen: Some(
            State::default()
                .sfen()
                .replace(" b 0", &format!(" b {MAX_PLY}")),
        ),
        result: GameResult {
            outcome: RecordedOutcome::Ongoing,
            reason: Termination::CounterLimit,
        },
        ..GameMetadata::default()
    };
    let r = GameRecord::new(m, Vec::new()).unwrap();
    assert_eq!(r.final_position().ply(), MAX_PLY);
}
#[test]
fn malformed_json_metadata_and_indexed_moves() {
    let duplicate = include_bytes!("fixtures/ssgf/invalid-duplicate.rssgf");
    assert_eq!(
        GameRecord::parse(duplicate, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Json
    );
    let nested=br#"{"format":"RSSGF","version":1,"rules":"usp-game-1.0","result":{"outcome":"ongoing","reason":"unfinished"},"turns":[],"extensions":{"x":{"key":1,"key":2}}}"#;
    assert_eq!(
        GameRecord::parse(nested, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Json
    );
    for bytes in [
        modified(|v| {
            v["typo"] = json!(1);
        }),
        modified(|v| {
            v["version"] = json!("1");
        }),
        modified(|v| {
            v["players"] = json!({"black":{"ratings":[{"system":"elo","value":"1800"}]}});
        }),
        modified(|v| {
            v.as_object_mut().unwrap().remove("result");
        }),
    ] {
        assert_eq!(
            GameRecord::parse(&bytes, Limits::default())
                .unwrap_err()
                .kind,
            ErrorKind::Schema
        );
    }
    assert_eq!(
        GameRecord::parse(&modified(|v| v["version"] = json!(2)), Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Version
    );
    assert_eq!(
        GameRecord::parse(
            &modified(|v| v["rules"] = json!("other")),
            Limits::default()
        )
        .unwrap_err()
        .kind,
        ErrorKind::Profile
    );
    let illegal = GameRecord::parse(
        include_bytes!("fixtures/ssgf/invalid-illegal.rssgf"),
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(illegal.kind, ErrorKind::IllegalTurn);
    assert_eq!(illegal.turn_index, Some(0));
    let malformed = GameRecord::parse(
        &modified(|v| v["turns"] = json!(["bad"])),
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(malformed.kind, ErrorKind::Turn);
    assert_eq!(malformed.turn_index, Some(0));
    assert_eq!(
        GameRecord::parse(
            &modified(|v| v["comments"] = json!([{"turn":1,"text":"bad index"}])),
            Limits::default()
        )
        .unwrap_err()
        .kind,
        ErrorKind::Metadata
    );
    let mut v: Value = serde_json::from_slice(include_bytes!("fixtures/ssgf/win.rssgf")).unwrap();
    v["turns"]
        .as_array_mut()
        .unwrap()
        .push(json!("2a1a2:3a1a2"));
    assert_eq!(
        GameRecord::parse(&serde_json::to_vec(&v).unwrap(), Limits::default())
            .unwrap_err()
            .turn_index,
        Some(1)
    );
    let m = GameMetadata {
        players: Some(Players {
            black: Some(PlayerData {
                ratings: vec![Rating {
                    system: "elo".into(),
                    value: f64::INFINITY,
                    provider: None,
                }],
                ..PlayerData::default()
            }),
            ..Players::default()
        }),
        ..GameMetadata::default()
    };
    assert_eq!(
        GameRecord::new(m, Vec::new()).unwrap_err().kind,
        ErrorKind::Metadata
    );
}
#[test]
fn timestamps() {
    for timestamp in [
        "2026-10-11T12:00:00-07:00",
        "2024-02-29T00:00:00Z",
        "2026-10-11t00:00:00.123z",
    ] {
        assert!(
            GameRecord::parse(
                &modified(|v| v["timestamp"] = json!(timestamp)),
                Limits::default()
            )
            .is_ok()
        );
    }
    for timestamp in [
        "2026-10-11",
        "2026-10-11T12:00:00",
        "2025-02-29T12:00:00Z",
        "2026-13-01T12:00:00Z",
        "2026-01-01T24:00:00Z",
        "2026-01-01T12:00:00+24:00",
    ] {
        assert_eq!(
            GameRecord::parse(
                &modified(|v| v["timestamp"] = json!(timestamp)),
                Limits::default()
            )
            .unwrap_err()
            .kind,
            ErrorKind::Metadata
        );
    }
}
#[test]
fn binary_errors_and_resource_limits() {
    let good = parse(OPENING)
        .write(Encoding::Binary, Limits::default())
        .unwrap();
    for len in 8..good.len() {
        assert!(GameRecord::parse(&good[..len], Limits::default()).is_err());
    }
    let mut bytes = good.clone();
    bytes.push(0);
    assert_eq!(
        GameRecord::parse(&bytes, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Binary
    );
    bytes = good.clone();
    bytes[8] = 2;
    assert_eq!(
        GameRecord::parse(&bytes, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Version
    );
    bytes = good.clone();
    bytes[10] = 1;
    assert_eq!(
        GameRecord::parse(&bytes, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Binary
    );
    bytes = good.clone();
    bytes[16..20].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        GameRecord::parse(&bytes, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Binary
    );
    bytes = good.clone();
    let end = bytes.len();
    bytes[end - 2..].copy_from_slice(&0xffffu16.to_le_bytes());
    assert_eq!(
        GameRecord::parse(&bytes, Limits::default())
            .unwrap_err()
            .turn_index,
        Some(0)
    );
    for input in [OPENING, good.as_slice()] {
        for limits in [
            Limits {
                turns: 0,
                ..Limits::default()
            },
            Limits {
                metadata_bytes: 0,
                ..Limits::default()
            },
            Limits {
                total_bytes: input.len() - 1,
                ..Limits::default()
            },
        ] {
            assert_eq!(
                GameRecord::parse(input, limits).unwrap_err().kind,
                ErrorKind::Limit
            );
        }
    }
    let exact = Limits {
        total_bytes: good.len(),
        ..Limits::default()
    };
    assert!(GameRecord::parse(&good, exact).is_ok());
    assert_eq!(
        parse(OPENING)
            .write(
                Encoding::Binary,
                Limits {
                    turns: 0,
                    ..Limits::default()
                }
            )
            .unwrap_err()
            .kind,
        ErrorKind::Limit
    );
}

#[test]
fn streaming_turn_limits_types_and_order() {
    let limits = Limits {
        turns: 0,
        ..Limits::default()
    };
    // Fail at the first over-limit element, before decoding its huge/broken body.
    let huge = format!("{{\"turns\":[\"{}", "x".repeat(100000));
    assert_eq!(
        GameRecord::parse(huge.as_bytes(), limits).unwrap_err().kind,
        ErrorKind::Limit
    );
    for turns in ["null", "0", "{}"] {
        let input = format!("{{\"turns\":{turns}}}");
        assert_eq!(
            GameRecord::parse(input.as_bytes(), Limits::default())
                .unwrap_err()
                .kind,
            ErrorKind::Schema
        );
    }
    for turn in ["null", "0", "{}", "[]"] {
        let input = format!("{{\"turns\":[{turn}]}}");
        let error = GameRecord::parse(input.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Turn);
        assert_eq!(error.turn_index, Some(0));
    }
    let input=br#"{"turns":["0a1a2:1b1b2"],"version":1,"result":{"outcome":"ongoing","reason":"unfinished"},"format":"RSSGF","rules":"usp-game-1.0"}"#;
    assert_eq!(
        GameRecord::parse(input, Limits::default())
            .unwrap()
            .turns()
            .len(),
        1
    );
    let duplicate = br#"{"turns":[],"turn\u0073":[]}"#;
    assert_eq!(
        GameRecord::parse(duplicate, Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Json
    );
    // Binary syntax after the first illegal move must not force a full decode pass.
    let mut binary = parse(OPENING)
        .write(Encoding::Binary, Limits::default())
        .unwrap();
    binary[16..20].copy_from_slice(&2u32.to_le_bytes());
    let end = binary.len();
    binary[end - 2..].copy_from_slice(&0x0108u16.to_le_bytes()); // wrong board color
    binary.extend_from_slice(&0xffffu16.to_le_bytes());
    let error = GameRecord::parse(&binary, Limits::default()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::IllegalTurn);
    assert_eq!(error.turn_index, Some(0));
}

#[test]
fn metadata_budget_uses_canonical_bytes_before_materialization() {
    let mut value: Value = serde_json::from_slice(OPENING).unwrap();
    value["extensions"] =
        json!({"unicode":"\u{00e9}\n\t\"\\","values":[true,false,null,1,1.0,-0.0,1e30]});
    let input = serde_json::to_vec_pretty(&value).unwrap();
    value.as_object_mut().unwrap().remove("turns");
    let size = serde_json::to_vec(&value).unwrap().len();
    assert!(
        GameRecord::parse(
            &input,
            Limits {
                metadata_bytes: size,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        GameRecord::parse(
            &input,
            Limits {
                metadata_bytes: size - 1,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit
    );
    let extended=br#"{"format":"RSSGF","version":1,"rules":"usp-game-1.0","result":{"outcome":"ongoing","reason":"unfinished"},"extensions":{"text":"\u00e9\u0000\b\f\n\r\t\\\""},"turns":[]}"#;
    let mut normalized: Value = serde_json::from_slice(extended).unwrap();
    normalized.as_object_mut().unwrap().remove("turns");
    let size = serde_json::to_vec(&normalized).unwrap().len();
    assert!(
        GameRecord::parse(
            extended,
            Limits {
                metadata_bytes: size,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}
