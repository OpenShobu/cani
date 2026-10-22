#![cfg(feature = "ssgf")]
use cani::{ssgf::*, *};
#[test]
fn recording_results_comments_undo_and_atomic_errors() {
    let initial = State::default();
    let mut builder = GameBuilder::new(initial, GameMetadata::default()).unwrap();
    let first: Turn = "0a1a2:1b1b2".parse().unwrap();
    let second: Turn = "2a4a3:3b4b3".parse().unwrap();
    assert!(builder.undo().is_err());
    assert!(builder.comment(0, "missing".into()).is_err());
    assert!(builder.append(Turn::default()).is_err());
    assert_eq!(builder.position(), initial);
    builder.append(first).unwrap();
    builder
        .comment(0, String::from_utf8(vec![0xe6, 0x97, 0xa5]).unwrap())
        .unwrap();
    builder.append(second).unwrap();
    builder.comment(1, "removed".into()).unwrap();
    let snapshot = builder.record().unwrap();
    for encoding in [Encoding::Readable, Encoding::Binary] {
        let bytes = builder.write(encoding, Limits::default()).unwrap();
        assert_eq!(
            GameRecord::parse(&bytes, Limits::default()).unwrap(),
            snapshot
        );
    }
    let external = GameResult {
        outcome: RecordedOutcome::White,
        reason: Termination::Resignation,
    };
    builder.declare_result(external).unwrap();
    assert!(builder.append(first).is_err());
    assert_eq!(builder.position(), snapshot.final_position());
    assert_eq!(builder.undo().unwrap(), second);
    assert_eq!(builder.metadata().comments.len(), 1);
    assert_eq!(builder.metadata().result, GameResult::default());
    let mut metadata = builder.metadata().clone();
    metadata.comment = Some("game".into());
    builder.set_metadata(metadata).unwrap();
    let before = builder.record().unwrap();
    let mut metadata = builder.metadata().clone();
    metadata.start_sfen = Some(snapshot.final_position().sfen());
    assert!(builder.set_metadata(metadata).is_err());
    assert_eq!(builder.record().unwrap(), before);
    assert_eq!(builder.undo().unwrap(), first);
    assert_eq!(builder.position(), initial);
    assert!(builder.metadata().comments.is_empty());
    assert_eq!(snapshot.turns().len(), 2);
    let mut copied = GameBuilder::from_record(&snapshot);
    assert_eq!(copied.record().unwrap(), snapshot);
    copied.undo().unwrap();
    assert_eq!(snapshot.turns().len(), 2);
}
#[test]
fn terminal_study_capture_counter_and_declarations() {
    let state = State::parse("3W/4/B3/4|4/4/1BW1/4|3W/4/4/B3|3W/4/4/B3 b 0").unwrap();
    let mut builder = GameBuilder::new(state, GameMetadata::default()).unwrap();
    let capture = "0a2c2:1b2d2".parse().unwrap();
    builder.append(capture).unwrap();
    assert_eq!(
        builder.metadata().result,
        GameResult {
            outcome: RecordedOutcome::Black,
            reason: Termination::Rules
        }
    );
    assert!(
        builder
            .declare_result(GameResult {
                outcome: RecordedOutcome::White,
                reason: Termination::Timeout
            })
            .is_err()
    );
    assert!(builder.append(capture).is_err());
    let terminal = GameBuilder::new(builder.position(), GameMetadata::default()).unwrap();
    assert_eq!(terminal.metadata().result, builder.metadata().result);
    builder.undo().unwrap();
    assert_eq!(builder.position(), state);
    let state =
        State::from_bitboards(State::default().bitboards(), Player::White, MAX_PLY).unwrap();
    let mut limit = GameBuilder::new(state, GameMetadata::default()).unwrap();
    assert_eq!(limit.metadata().result.reason, Termination::CounterLimit);
    assert!(limit.append(capture).is_err());
    assert_eq!(limit.record().unwrap().final_position(), state);
}
