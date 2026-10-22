//! SSGF v1: validated single-line game records, readable JSON and packed binary.
use crate::{MAX_PLY, MoveError, Outcome, Player, State, Turn};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;
pub mod ffi;

pub const MAGIC: &[u8; 8] = b"BSSGF\r\n\x1a";
pub const PROFILE: &str = "usp-game-1.0";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Readable,
    Binary,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub total_bytes: usize,
    pub metadata_bytes: usize,
    pub turns: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            total_bytes: 64 * 1024 * 1024,
            metadata_bytes: 1024 * 1024,
            turns: 1_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Json,
    Schema,
    Version,
    Profile,
    Limit,
    Binary,
    Metadata,
    Turn,
    IllegalTurn,
    Result,
    Index,
}
#[derive(Debug)]
pub struct RecordError {
    pub kind: ErrorKind,
    pub turn_index: Option<usize>,
    pub detail: String,
}
impl RecordError {
    fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            turn_index: None,
            detail: detail.into(),
        }
    }
    fn at(kind: ErrorKind, index: usize, detail: impl Into<String>) -> Self {
        Self {
            kind,
            turn_index: Some(index),
            detail: detail.into(),
        }
    }
}
impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)?;
        if let Some(i) = self.turn_index {
            write!(f, " at turn {i}")?;
        }
        write!(f, ": {}", self.detail)
    }
}
impl std::error::Error for RecordError {}
fn empty_map(v: &Map<String, Value>) -> bool {
    v.is_empty()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rating {
    pub system: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKind {
    Human,
    Bot,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<PlayerKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ratings: Vec<Rating>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Players {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub black: Option<PlayerData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white: Option<PlayerData>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizer: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedOutcome {
    Ongoing,
    #[serde(rename = "b")]
    Black,
    #[serde(rename = "w")]
    White,
    Draw,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    Rules,
    Resignation,
    Timeout,
    Agreement,
    Adjudication,
    Abandonment,
    Unfinished,
    CounterLimit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameResult {
    pub outcome: RecordedOutcome,
    pub reason: Termination,
}
impl Default for GameResult {
    fn default() -> Self {
        Self {
            outcome: RecordedOutcome::Ongoing,
            reason: Termination::Unfinished,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comment {
    pub turn: usize,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameMetadata {
    pub version: u16,
    pub rules: String,
    pub result: GameResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_sfen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players: Option<Players>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
    #[serde(default, skip_serializing_if = "empty_map")]
    pub extensions: Map<String, Value>,
}
impl Default for GameMetadata {
    fn default() -> Self {
        Self {
            version: 1,
            rules: PROFILE.into(),
            result: GameResult::default(),
            start_sfen: None,
            players: None,
            event: None,
            game_id: None,
            timestamp: None,
            source_url: None,
            comment: None,
            comments: Vec::new(),
            extensions: Map::new(),
        }
    }
}
/// Fields are immutable after validation. Build a new record to change metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct GameRecord {
    metadata: GameMetadata,
    turns: Vec<Turn>,
    initial: State,
    final_position: State,
}
/// Mutable recorder, independent of USP. Appending a turn updates rules results;
/// undo removes comments on the removed turn and clears external declarations.
#[derive(Clone, Debug)]
pub struct GameBuilder {
    metadata: GameMetadata,
    turns: Vec<Turn>,
    positions: Vec<State>,
}
fn automatic_result(state: State) -> GameResult {
    match state.outcome() {
        Outcome::Win(player) => GameResult {
            outcome: if player == Player::Black {
                RecordedOutcome::Black
            } else {
                RecordedOutcome::White
            },
            reason: Termination::Rules,
        },
        Outcome::Ongoing => GameResult {
            outcome: RecordedOutcome::Ongoing,
            reason: if state.ply() == MAX_PLY {
                Termination::CounterLimit
            } else {
                Termination::Unfinished
            },
        },
    }
}
impl GameBuilder {
    /// Starting position and automatic result replace metadata's start/result.
    /// Other metadata is validated; comments must reference existing turns.
    pub fn new(initial: State, mut metadata: GameMetadata) -> Result<Self, RecordError> {
        metadata.start_sfen = if initial == State::default() {
            None
        } else {
            Some(initial.sfen())
        };
        metadata.result = automatic_result(initial);
        GameRecord::new(metadata.clone(), vec![])?;
        Ok(Self {
            metadata,
            turns: vec![],
            positions: vec![initial],
        })
    }
    pub fn from_record(record: &GameRecord) -> Self {
        Self {
            metadata: record.metadata.clone(),
            turns: record.turns.clone(),
            positions: record.replay().collect(),
        }
    }
    pub fn position(&self) -> State {
        *self.positions.last().unwrap()
    }
    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }
    pub fn metadata(&self) -> &GameMetadata {
        &self.metadata
    }
    /// Atomic validated metadata replacement. The starting position cannot change.
    pub fn set_metadata(&mut self, metadata: GameMetadata) -> Result<(), RecordError> {
        let record = GameRecord::new(metadata, self.turns.clone())?;
        if record.initial != self.positions[0] {
            return Err(RecordError::new(
                ErrorKind::Metadata,
                "cannot change starting position",
            ));
        }
        self.metadata = record.metadata;
        Ok(())
    }
    pub fn append(&mut self, turn: Turn) -> Result<(), RecordError> {
        let index = self.turns.len();
        if !matches!(
            self.metadata.result.reason,
            Termination::Unfinished | Termination::CounterLimit | Termination::Rules
        ) {
            return Err(RecordError::at(
                ErrorKind::Result,
                index,
                "external result has closed this record",
            ));
        }
        let next = self
            .position()
            .played(turn)
            .map_err(|e| RecordError::at(ErrorKind::IllegalTurn, index, e.to_string()))?;
        self.turns.push(turn);
        self.positions.push(next);
        self.metadata.result = automatic_result(next);
        Ok(())
    }
    pub fn comment(&mut self, index: usize, text: String) -> Result<(), RecordError> {
        if index >= self.turns.len() {
            return Err(RecordError::at(
                ErrorKind::Index,
                index,
                "comment has no turn",
            ));
        }
        self.metadata.comments.push(Comment { turn: index, text });
        Ok(())
    }
    pub fn undo(&mut self) -> Result<Turn, RecordError> {
        let turn = self
            .turns
            .pop()
            .ok_or_else(|| RecordError::new(ErrorKind::Index, "no turn to undo"))?;
        self.positions.pop();
        self.metadata.comments.retain(|c| c.turn < self.turns.len());
        self.metadata.result = automatic_result(self.position());
        Ok(turn)
    }
    pub fn declare_result(&mut self, result: GameResult) -> Result<(), RecordError> {
        validate_result(result, self.position())?;
        self.metadata.result = result;
        Ok(())
    }
    pub fn record(&self) -> Result<GameRecord, RecordError> {
        GameRecord::new(self.metadata.clone(), self.turns.clone())
    }
    pub fn write(&self, encoding: Encoding, limits: Limits) -> Result<Vec<u8>, RecordError> {
        self.record()?.write(encoding, limits)
    }
}
impl GameRecord {
    pub fn new(metadata: GameMetadata, turns: Vec<Turn>) -> Result<Self, RecordError> {
        validate_metadata(&metadata, turns.len())?;
        let initial = match &metadata.start_sfen {
            Some(s) => State::parse_sfen(s)
                .map_err(|e| RecordError::new(ErrorKind::Metadata, e.to_string()))?,
            None => State::default(),
        };
        let mut final_position = initial;
        for (i, turn) in turns.iter().enumerate() {
            final_position = final_position
                .played(*turn)
                .map_err(|e| RecordError::at(ErrorKind::IllegalTurn, i, e.to_string()))?;
        }
        validate_result(metadata.result, final_position)?;
        Ok(Self {
            metadata,
            turns,
            initial,
            final_position,
        })
    }
    pub fn metadata(&self) -> &GameMetadata {
        &self.metadata
    }
    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }
    pub fn initial_position(&self) -> State {
        self.initial
    }
    pub fn final_position(&self) -> State {
        self.final_position
    }
    /// Includes the initial position, then one position per completed turn.
    pub fn replay(&self) -> Replay<'_> {
        Replay {
            record: self,
            position: self.initial,
            index: 0,
        }
    }
    pub fn position_at(&self, index: usize) -> Result<State, RecordError> {
        if index > self.turns.len() {
            return Err(RecordError::new(
                ErrorKind::Index,
                "position index exceeds turn count",
            ));
        }
        Ok(self.replay().nth(index).expect("validated record"))
    }
    pub fn metadata_json(&self) -> Result<Vec<u8>, RecordError> {
        json_bytes(&self.metadata, false)
    }
    pub fn parse(input: &[u8], limits: Limits) -> Result<Self, RecordError> {
        if input.len() > limits.total_bytes {
            return Err(RecordError::new(ErrorKind::Limit, "total byte limit"));
        }
        if input.starts_with(MAGIC) {
            Self::parse_binary(input, limits)
        } else {
            Self::parse_readable(input, limits)
        }
    }
    fn parse_readable(input: &[u8], limits: Limits) -> Result<Self, RecordError> {
        let (mut object, turns) = parse_readable_parts(input, limits)?;
        check_format(&mut object, "RSSGF")?;
        check_metadata_size(&object, "RSSGF", limits)?;
        let metadata = serde_json::from_value(Value::Object(object))
            .map_err(|e| RecordError::new(ErrorKind::Schema, e.to_string()))?;
        Self::new(metadata, turns)
    }

    fn parse_binary(input: &[u8], limits: Limits) -> Result<Self, RecordError> {
        if input.len() < 20 {
            return Err(RecordError::new(ErrorKind::Binary, "truncated header"));
        }
        let version = u16::from_le_bytes(input[8..10].try_into().unwrap());
        let flags = u16::from_le_bytes(input[10..12].try_into().unwrap());
        if version != 1 {
            return Err(RecordError::new(
                ErrorKind::Version,
                "unsupported binary version",
            ));
        }
        if flags != 0 {
            return Err(RecordError::new(ErrorKind::Binary, "nonzero flags"));
        }
        let metadata_len = u32::from_le_bytes(input[12..16].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(input[16..20].try_into().unwrap()) as usize;
        if metadata_len > limits.metadata_bytes || count > limits.turns {
            return Err(RecordError::new(ErrorKind::Limit, "metadata or turn limit"));
        }
        let end = 20usize
            .checked_add(metadata_len)
            .ok_or_else(|| RecordError::new(ErrorKind::Binary, "length overflow"))?;
        let total = count
            .checked_mul(2)
            .and_then(|n| end.checked_add(n))
            .ok_or_else(|| RecordError::new(ErrorKind::Binary, "length overflow"))?;
        if total != input.len() {
            return Err(RecordError::new(
                ErrorKind::Binary,
                "length mismatch or trailing bytes",
            ));
        }
        let mut object = parse_object(&input[20..end])?;
        check_format(&mut object, "BSSGF")?;
        let metadata = serde_json::from_value(Value::Object(object))
            .map_err(|e| RecordError::new(ErrorKind::Schema, e.to_string()))?;
        let metadata: GameMetadata = metadata;
        validate_metadata(&metadata, count)?;
        let initial = match &metadata.start_sfen {
            Some(s) => State::parse_sfen(s)
                .map_err(|e| RecordError::new(ErrorKind::Metadata, e.to_string()))?,
            None => State::default(),
        };
        let mut final_position = initial;
        let mut turns = Vec::new();
        for (i, bytes) in input[end..].as_chunks::<2>().0.iter().enumerate() {
            let turn = unpack_turn(u16::from_le_bytes(*bytes))
                .map_err(|e| RecordError::at(ErrorKind::Turn, i, e.to_string()))?;
            final_position = final_position
                .played(turn)
                .map_err(|e| RecordError::at(ErrorKind::IllegalTurn, i, e.to_string()))?;
            turns.push(turn);
        }
        validate_result(metadata.result, final_position)?;
        Ok(Self {
            metadata,
            turns,
            initial,
            final_position,
        })
    }

    pub fn write(&self, encoding: Encoding, limits: Limits) -> Result<Vec<u8>, RecordError> {
        if self.turns.len() > limits.turns {
            return Err(RecordError::new(ErrorKind::Limit, "turn count limit"));
        }
        // Reject cheap resource violations before replaying a large record.
        let minimum = match encoding {
            Encoding::Readable => self.turns.len().checked_mul(14),
            Encoding::Binary => self
                .turns
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(20)),
        };
        if minimum.is_none_or(|n| n > limits.total_bytes) {
            return Err(RecordError::new(ErrorKind::Limit, "total byte limit"));
        }
        // Records are immutable; validate without cloning the full history.
        validate_metadata(&self.metadata, self.turns.len())?;
        let mut final_position = self.initial;
        for (i, turn) in self.turns.iter().enumerate() {
            final_position = final_position
                .played(*turn)
                .map_err(|e| RecordError::at(ErrorKind::IllegalTurn, i, e.to_string()))?;
        }
        validate_result(self.metadata.result, final_position)?;
        let format = match encoding {
            Encoding::Readable => "RSSGF",
            Encoding::Binary => "BSSGF",
        };
        let value = serde_json::to_value(&self.metadata)
            .map_err(|e| RecordError::new(ErrorKind::Metadata, e.to_string()))?;
        let mut object = value.as_object().expect("metadata object").clone();
        object.insert("format".into(), Value::String(format.into()));
        let metadata = json_bytes(&Value::Object(object.clone()), false)?;
        if metadata.len() > limits.metadata_bytes {
            return Err(RecordError::new(ErrorKind::Limit, "metadata byte limit"));
        }
        let out = match encoding {
            Encoding::Readable => {
                object.insert(
                    "turns".into(),
                    Value::Array(
                        self.turns
                            .iter()
                            .map(|t| Value::String(t.to_string()))
                            .collect(),
                    ),
                );
                let mut out = json_bytes(&Value::Object(object), true)?;
                out.push(b'\n');
                out
            }
            Encoding::Binary => {
                let m = u32::try_from(metadata.len()).map_err(|_| {
                    RecordError::new(ErrorKind::Limit, "metadata length exceeds u32")
                })?;
                let n = u32::try_from(self.turns.len())
                    .map_err(|_| RecordError::new(ErrorKind::Limit, "turn count exceeds u32"))?;
                let mut out = Vec::new();
                out.extend_from_slice(MAGIC);
                out.extend_from_slice(&1u16.to_le_bytes());
                out.extend_from_slice(&0u16.to_le_bytes());
                out.extend_from_slice(&m.to_le_bytes());
                out.extend_from_slice(&n.to_le_bytes());
                out.extend_from_slice(&metadata);
                for (i, t) in self.turns.iter().enumerate() {
                    out.extend_from_slice(
                        &pack_turn(*t)
                            .map_err(|e| RecordError::at(ErrorKind::Turn, i, e.to_string()))?
                            .to_le_bytes(),
                    );
                }
                out
            }
        };
        if out.len() > limits.total_bytes {
            return Err(RecordError::new(ErrorKind::Limit, "total byte limit"));
        }
        Ok(out)
    }
}
pub struct Replay<'a> {
    record: &'a GameRecord,
    position: State,
    index: usize,
}
impl Iterator for Replay<'_> {
    type Item = State;
    fn next(&mut self) -> Option<State> {
        if self.index > self.record.turns.len() {
            return None;
        }
        if self.index > 0 {
            self.position = self
                .position
                .played(self.record.turns[self.index - 1])
                .expect("validated immutable record");
        }
        self.index += 1;
        Some(self.position)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.record.turns.len() + 1).saturating_sub(self.index);
        (n, Some(n))
    }
}
impl ExactSizeIterator for Replay<'_> {}
impl std::iter::FusedIterator for Replay<'_> {}
pub use crate::neural::{pack_turn, unpack_turn};
fn validate_metadata(m: &GameMetadata, count: usize) -> Result<(), RecordError> {
    if m.version != 1 {
        return Err(RecordError::new(
            ErrorKind::Version,
            "unsupported SSGF version",
        ));
    }
    if m.rules != PROFILE {
        return Err(RecordError::new(
            ErrorKind::Profile,
            "unsupported rules profile",
        ));
    }
    if let Some(players) = &m.players {
        for player in [&players.black, &players.white].into_iter().flatten() {
            for r in &player.ratings {
                if r.system.is_empty() || !r.value.is_finite() {
                    return Err(RecordError::new(
                        ErrorKind::Metadata,
                        "rating needs a nonempty system and finite value",
                    ));
                }
            }
        }
    }
    for comment in &m.comments {
        if comment.turn >= count {
            return Err(RecordError::at(
                ErrorKind::Metadata,
                comment.turn,
                "comment turn out of range",
            ));
        }
    }
    if m.timestamp.as_ref().is_some_and(|s| !valid_timestamp(s)) {
        return Err(RecordError::new(
            ErrorKind::Metadata,
            "timestamp requires RFC3339 date/time and timezone",
        ));
    }
    Ok(())
}
fn validate_result(r: GameResult, s: State) -> Result<(), RecordError> {
    let board = match s.outcome() {
        Outcome::Ongoing => RecordedOutcome::Ongoing,
        Outcome::Win(Player::Black) => RecordedOutcome::Black,
        Outcome::Win(Player::White) => RecordedOutcome::White,
    };
    let valid = match r.reason {
        Termination::Rules => board != RecordedOutcome::Ongoing && r.outcome == board,
        Termination::Unfinished | Termination::Abandonment => {
            board == RecordedOutcome::Ongoing && r.outcome == RecordedOutcome::Ongoing
        }
        Termination::CounterLimit => {
            board == RecordedOutcome::Ongoing
                && r.outcome == RecordedOutcome::Ongoing
                && s.ply() == MAX_PLY
        }
        Termination::Resignation | Termination::Timeout => {
            board == RecordedOutcome::Ongoing
                && matches!(r.outcome, RecordedOutcome::Black | RecordedOutcome::White)
        }
        Termination::Agreement => {
            board == RecordedOutcome::Ongoing && r.outcome == RecordedOutcome::Draw
        }
        Termination::Adjudication => {
            board == RecordedOutcome::Ongoing && r.outcome != RecordedOutcome::Ongoing
        }
    };
    if !valid {
        return Err(RecordError::new(
            ErrorKind::Result,
            "result/reason contradict replay or each other",
        ));
    }
    Ok(())
}
fn valid_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    fn number(b: &[u8]) -> Option<u32> {
        if b.is_empty() || !b.iter().all(u8::is_ascii_digit) {
            return None;
        }
        Some(b.iter().fold(0, |n, c| n * 10 + (c - b'0') as u32))
    }
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return false;
    }
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(&b[..4]),
        number(&b[5..7]),
        number(&b[8..10]),
        number(&b[11..13]),
        number(&b[14..16]),
        number(&b[17..19]),
    ) else {
        return false;
    };
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day == 0 || day > days {
        return false;
    }
    let mut i = 19;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    if b.len() == i + 1 && matches!(b[i], b'Z' | b'z') {
        return true;
    }
    b.len() == i + 6
        && matches!(b[i], b'+' | b'-')
        && b[i + 3] == b':'
        && number(&b[i + 1..i + 3]).is_some_and(|n| n <= 23)
        && number(&b[i + 4..i + 6]).is_some_and(|n| n <= 59)
}
fn check_format(object: &mut Map<String, Value>, format: &str) -> Result<(), RecordError> {
    if object.remove("format") != Some(Value::String(format.into())) {
        return Err(RecordError::new(
            ErrorKind::Schema,
            "missing or mismatched format",
        ));
    }
    Ok(())
}
fn check_metadata_size(
    object: &Map<String, Value>,
    format: &str,
    limits: Limits,
) -> Result<(), RecordError> {
    let mut object = object.clone();
    object.insert("format".into(), Value::String(format.into()));
    if json_bytes(&Value::Object(object), false)?.len() > limits.metadata_bytes {
        return Err(RecordError::new(ErrorKind::Limit, "metadata byte limit"));
    }
    Ok(())
}
fn json_bytes(value: &impl Serialize, pretty: bool) -> Result<Vec<u8>, RecordError> {
    // serde_json's Map is sorted unless preserve_order is enabled by another crate;
    // explicitly sort recursively so writers are deterministic either way.
    fn sorted(value: Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut pairs: Vec<_> = map.into_iter().collect();
                pairs.sort_by(|a, b| a.0.cmp(&b.0));
                let mut out = Map::new();
                for (k, v) in pairs {
                    out.insert(k, sorted(v));
                }
                Value::Object(out)
            }
            Value::Array(v) => Value::Array(v.into_iter().map(sorted).collect()),
            _ => value,
        }
    }
    let value = sorted(
        serde_json::to_value(value)
            .map_err(|e| RecordError::new(ErrorKind::Metadata, e.to_string()))?,
    );
    (if pretty {
        serde_json::to_vec_pretty(&value)
    } else {
        serde_json::to_vec(&value)
    })
    .map_err(|e| RecordError::new(ErrorKind::Json, e.to_string()))
}
// Custom Value deserialization rejects duplicate keys at EVERY nesting depth,
// including extensions; serde_json::Value alone otherwise silently overwrites.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.into())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(v)) = a.next_element()? {
                    values.push(v);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if values.contains_key(&k) {
                        return Err(serde::de::Error::custom(format!("duplicate key: {k}")));
                    }
                    let Unique(v) = a.next_value()?;
                    values.insert(k, v);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn parse_object(input: &[u8]) -> Result<Map<String, Value>, RecordError> {
    let Unique(v) = serde_json::from_slice(input)
        .map_err(|e| RecordError::new(ErrorKind::Json, e.to_string()))?;
    match v {
        Value::Object(object) => Ok(object),
        _ => Err(RecordError::new(
            ErrorKind::Schema,
            "record must be an object",
        )),
    }
}

// RSSGF keeps only typed turns, rather than a second JSON tree of turn strings.
// Seeds enforce limits before decoding the next element. Type failures retain
// structured record errors without depending on serde's diagnostic wording.
fn parse_readable_parts(
    input: &[u8],
    limits: Limits,
) -> Result<(Map<String, Value>, Vec<Turn>), RecordError> {
    use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
    macro_rules! reject_scalars {
        () => {
            fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
                Err(self.bad())
            }
            fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
                Err(self.bad())
            }
            fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
                Err(self.bad())
            }
            fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
                Err(self.bad())
            }
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Err(self.bad())
            }
        };
    }
    struct Root<'a> {
        limits: Limits,
        failure: &'a mut Option<RecordError>,
        metadata_used: usize,
    }
    impl Root<'_> {
        fn bad<E: Error>(self) -> E {
            *self.failure = Some(RecordError::new(
                ErrorKind::Schema,
                "record must be an object",
            ));
            E::custom("expected object")
        }
    }
    struct Turns<'a> {
        limit: usize,
        failure: &'a mut Option<RecordError>,
    }
    impl Turns<'_> {
        fn bad<E: Error>(self) -> E {
            *self.failure = Some(RecordError::new(
                ErrorKind::Schema,
                "turns must be an array",
            ));
            E::custom("expected array")
        }
    }
    struct Element<'a> {
        index: usize,
        limit: usize,
        failure: &'a mut Option<RecordError>,
    }
    impl Element<'_> {
        fn bad<E: Error>(self) -> E {
            *self.failure = Some(RecordError::at(
                ErrorKind::Turn,
                self.index,
                "turn must be a string",
            ));
            E::custom("expected turn string")
        }
    }
    impl<'de> DeserializeSeed<'de> for Element<'_> {
        type Value = Turn;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Turn, D::Error> {
            if self.index >= self.limit {
                *self.failure = Some(RecordError::new(ErrorKind::Limit, "turn count limit"));
                return Err(D::Error::custom("turn count limit"));
            }
            d.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Element<'_> {
        type Value = Turn;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("USP turn string")
        }
        fn visit_str<E: Error>(self, text: &str) -> Result<Turn, E> {
            text.parse().map_err(|e: MoveError| {
                *self.failure = Some(RecordError::at(ErrorKind::Turn, self.index, e.to_string()));
                E::custom("invalid turn")
            })
        }
        reject_scalars!();
        fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<Turn, A::Error> {
            Err(self.bad())
        }
        fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<Turn, A::Error> {
            Err(self.bad())
        }
    }
    impl<'de> DeserializeSeed<'de> for Turns<'_> {
        type Value = Vec<Turn>;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Turns<'_> {
        type Value = Vec<Turn>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("array of turns")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Turn>, A::Error> {
            let mut turns = Vec::new();
            while let Some(turn) = seq.next_element_seed(Element {
                index: turns.len(),
                limit: self.limit,
                failure: self.failure,
            })? {
                turns.push(turn);
            }
            Ok(turns)
        }
        reject_scalars!();
        fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
            Err(self.bad())
        }
        fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
            Err(self.bad())
        }
    }
    impl<'de> DeserializeSeed<'de> for Root<'_> {
        type Value = (Map<String, Value>, Vec<Turn>);
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Root<'_> {
        type Value = (Map<String, Value>, Vec<Turn>);
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("RSSGF object")
        }
        fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut object = Map::new();
            let mut turns = None;
            while let Some(key) = map.next_key::<String>()? {
                if object.contains_key(&key) || (key == "turns" && turns.is_some()) {
                    return Err(A::Error::custom("duplicate root key"));
                }
                if key == "turns" {
                    turns = Some(map.next_value_seed(Turns {
                        limit: self.limits.turns,
                        failure: self.failure,
                    })?);
                } else {
                    self.metadata_used = self.metadata_used.saturating_add(
                        json_string_size(&key) + 1 + usize::from(!object.is_empty()),
                    );
                    let raw: &serde_json::value::RawValue = map.next_value()?;
                    let remaining = self
                        .limits
                        .metadata_bytes
                        .saturating_sub(self.metadata_used);
                    let measured = measure_json(raw.get().as_bytes(), remaining);
                    let size = match measured {
                        Ok(size) if self.metadata_used <= self.limits.metadata_bytes => size,
                        Ok(_) => {
                            *self.failure =
                                Some(RecordError::new(ErrorKind::Limit, "metadata byte limit"));
                            return Err(A::Error::custom("metadata byte limit"));
                        }
                        Err(error) => {
                            *self.failure = Some(error);
                            return Err(A::Error::custom("invalid metadata"));
                        }
                    };
                    self.metadata_used += size;
                    let Unique(value) =
                        serde_json::from_str(raw.get()).map_err(A::Error::custom)?;
                    object.insert(key, value);
                }
            }
            let turns = turns.ok_or_else(|| {
                *self.failure = Some(RecordError::new(ErrorKind::Schema, "missing turns"));
                A::Error::custom("missing turns")
            })?;
            Ok((object, turns))
        }
        reject_scalars!();
        fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
            Err(self.bad())
        }
        fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
            Err(self.bad())
        }
    }
    let mut failure = None;
    let mut de = serde_json::Deserializer::from_slice(input);
    let parsed = Root {
        limits,
        failure: &mut failure,
        metadata_used: 2,
    }
    .deserialize(&mut de);
    match parsed {
        Ok(value) => {
            de.end()
                .map_err(|e| RecordError::new(ErrorKind::Json, e.to_string()))?;
            Ok(value)
        }
        Err(e) => Err(failure.unwrap_or_else(|| RecordError::new(ErrorKind::Json, e.to_string()))),
    }
}

fn json_string_size(text: &str) -> usize {
    2 + text
        .bytes()
        .map(|b| match b {
            b'"' | b'\\' | 8 | 9 | 10 | 12 | 13 => 2,
            0..=31 => 6,
            _ => 1,
        })
        .sum::<usize>()
}
fn measure_json(input: &[u8], limit: usize) -> Result<usize, RecordError> {
    use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
    struct Size<'a> {
        used: &'a mut usize,
        limit: usize,
    }
    impl Size<'_> {
        fn charge<E: Error>(&mut self, n: usize) -> Result<(), E> {
            *self.used = self.used.saturating_add(n);
            if *self.used > self.limit {
                Err(E::custom("metadata byte limit"))
            } else {
                Ok(())
            }
        }
    }
    impl<'de> DeserializeSeed<'de> for Size<'_> {
        type Value = ();
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
            d.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Size<'_> {
        type Value = ();
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded JSON metadata")
        }
        fn visit_bool<E: Error>(mut self, b: bool) -> Result<(), E> {
            self.charge(if b { 4 } else { 5 })
        }
        fn visit_unit<E: Error>(mut self) -> Result<(), E> {
            self.charge(4)
        }
        fn visit_i64<E: Error>(mut self, n: i64) -> Result<(), E> {
            self.charge(n.to_string().len())
        }
        fn visit_u64<E: Error>(mut self, n: u64) -> Result<(), E> {
            self.charge(n.to_string().len())
        }
        fn visit_f64<E: Error>(mut self, n: f64) -> Result<(), E> {
            let n = serde_json::Number::from_f64(n).ok_or_else(|| E::custom("nonfinite number"))?;
            self.charge(n.to_string().len())
        }
        fn visit_str<E: Error>(mut self, s: &str) -> Result<(), E> {
            self.charge(json_string_size(s))
        }
        fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<(), A::Error> {
            self.charge(2)?;
            let mut first = true;
            while seq
                .next_element_seed(Size {
                    used: self.used,
                    limit: self.limit,
                })?
                .is_some()
            {
                if !first {
                    self.charge(1)?;
                }
                first = false;
            }
            Ok(())
        }
        fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
            self.charge(2)?;
            let mut first = true;
            while map
                .next_key_seed(Size {
                    used: self.used,
                    limit: self.limit,
                })?
                .is_some()
            {
                self.charge(1 + usize::from(!first))?;
                first = false;
                map.next_value_seed(Size {
                    used: self.used,
                    limit: self.limit,
                })?;
            }
            Ok(())
        }
    }
    let mut used = 0;
    let mut de = serde_json::Deserializer::from_slice(input);
    match (Size {
        used: &mut used,
        limit,
    })
    .deserialize(&mut de)
    {
        Ok(()) => {
            de.end()
                .map_err(|e| RecordError::new(ErrorKind::Json, e.to_string()))?;
            Ok(used)
        }
        Err(e) => Err(RecordError::new(
            if used > limit {
                ErrorKind::Limit
            } else {
                ErrorKind::Json
            },
            e.to_string(),
        )),
    }
}
