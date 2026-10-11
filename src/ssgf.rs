//! SSGF v1: validated single-line game records, readable JSON and packed binary.
use crate::{BoardId, MAX_PLY, Move, MoveError, Outcome, Player, Square, State, Turn};
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
        let mut object = parse_object(input)?;
        check_format(&mut object, "RSSGF")?;
        let encoded = object
            .remove("turns")
            .ok_or_else(|| RecordError::new(ErrorKind::Schema, "missing turns"))?;
        let encoded = encoded
            .as_array()
            .ok_or_else(|| RecordError::new(ErrorKind::Schema, "turns must be an array"))?;
        if encoded.len() > limits.turns {
            return Err(RecordError::new(ErrorKind::Limit, "turn count limit"));
        }
        // Metadata limit uses compact canonical JSON INCLUDING format and excluding turns.
        check_metadata_size(&object, "RSSGF", limits)?;
        let metadata = serde_json::from_value(Value::Object(object))
            .map_err(|e| RecordError::new(ErrorKind::Schema, e.to_string()))?;
        let mut turns = Vec::with_capacity(encoded.len());
        for (i, value) in encoded.iter().enumerate() {
            let s = value
                .as_str()
                .ok_or_else(|| RecordError::at(ErrorKind::Turn, i, "turn must be a string"))?;
            turns.push(
                s.parse()
                    .map_err(|e: MoveError| RecordError::at(ErrorKind::Turn, i, e.to_string()))?,
            );
        }
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
        let mut turns = Vec::with_capacity(count);
        for (i, bytes) in input[end..].as_chunks::<2>().0.iter().enumerate() {
            turns.push(
                unpack_turn(u16::from_le_bytes([bytes[0], bytes[1]]))
                    .map_err(|e| RecordError::at(ErrorKind::Turn, i, e.to_string()))?,
            );
        }
        Self::new(metadata, turns)
    }
    pub fn write(&self, encoding: Encoding, limits: Limits) -> Result<Vec<u8>, RecordError> {
        // Revalidate before writing, including adjudication and every transition.
        Self::new(self.metadata.clone(), self.turns.clone())?;
        if self.turns.len() > limits.turns {
            return Err(RecordError::new(ErrorKind::Limit, "turn count limit"));
        }
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
const DIRECTIONS: [(i8, i8); 8] = [
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];
/// Packing checks geometry only, not ownership, board color or position legality.
pub fn pack_turn(turn: Turn) -> Result<u16, MoveError> {
    fn delta(m: Move) -> (i8, i8) {
        (
            m.to.file() as i8 - m.from.file() as i8,
            m.to.rank() as i8 - m.from.rank() as i8,
        )
    }
    let (dx, dy) = delta(turn.passive);
    let n = dx.abs().max(dy.abs());
    if !(1..=2).contains(&n)
        || (dx != 0 && dx.abs() != n)
        || (dy != 0 && dy.abs() != n)
        || delta(turn.aggressive) != (dx, dy)
    {
        return Err(MoveError::Geometry);
    }
    let direction = DIRECTIONS
        .iter()
        .position(|&v| v == (dx / n, dy / n))
        .ok_or(MoveError::Geometry)? as u16;
    Ok(turn.passive.board.index() as u16
        | ((turn.aggressive.board.index() as u16) << 2)
        | ((turn.passive.from.index() as u16) << 4)
        | ((turn.aggressive.from.index() as u16) << 8)
        | (direction << 12)
        | (((n - 1) as u16) << 15))
}
pub fn unpack_turn(bits: u16) -> Result<Turn, MoveError> {
    let n = (((bits >> 15) & 1) + 1) as i8;
    let (dx, dy) = DIRECTIONS[((bits >> 12) & 7) as usize];
    fn leg(board: u8, from: u8, dx: i8, dy: i8) -> Result<Move, MoveError> {
        let x = (from % 4) as i8 + dx;
        let y = (from / 4) as i8 + dy;
        if !(0..4).contains(&x) || !(0..4).contains(&y) {
            return Err(MoveError::Geometry);
        }
        Ok(Move {
            board: BoardId::new(board).unwrap(),
            from: Square::new(from).unwrap(),
            to: Square::new((y * 4 + x) as u8).unwrap(),
        })
    }
    Ok(Turn {
        passive: leg((bits & 3) as u8, ((bits >> 4) & 15) as u8, dx * n, dy * n)?,
        aggressive: leg(
            ((bits >> 2) & 3) as u8,
            ((bits >> 8) & 15) as u8,
            dx * n,
            dy * n,
        )?,
    })
}
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
