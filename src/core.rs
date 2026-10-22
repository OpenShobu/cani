//! Typed, allocation-free rules primitives. Coordinates are absolute (Black's seat).
use std::{fmt, ops::ControlFlow, str::FromStr};

pub const MAX_PLY: u32 = 2_147_483_647;
pub const MAX_LEGAL_TURNS: usize = 1024;
pub const INITIAL: &str = "WWWW/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB b 0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Player {
    Black = 0,
    White = 1,
}
impl Player {
    pub const fn opponent(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoardId(u8);
impl BoardId {
    pub const fn new(id: u8) -> Result<Self, ValidationError> {
        if id < 4 {
            Ok(Self(id))
        } else {
            Err(ValidationError::Board)
        }
    }
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Square(u8);
impl Square {
    pub const fn new(index: u8) -> Result<Self, ValidationError> {
        if index < 16 {
            Ok(Self(index))
        } else {
            Err(ValidationError::Square)
        }
    }
    pub const fn index(self) -> usize {
        self.0 as usize
    }
    pub const fn file(self) -> u8 {
        self.0 % 4
    }
    pub const fn rank(self) -> u8 {
        self.0 / 4 + 1
    }
    fn bit(self) -> u16 {
        1 << self.0
    }
}
impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", (b'a' + self.file()) as char, self.rank())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub board: BoardId,
    pub from: Square,
    pub to: Square,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Turn {
    pub passive: Move,
    pub aggressive: Move,
}
impl Default for Turn {
    fn default() -> Self {
        let m = Move {
            board: BoardId(0),
            from: Square(0),
            to: Square(0),
        };
        Self {
            passive: m,
            aggressive: m,
        }
    }
}
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.board.0, self.from, self.to)
    }
}
impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.passive, self.aggressive)
    }
}
fn parse_move(b: &[u8]) -> Result<Move, MoveError> {
    if b.len() != 5
        || !(b'0'..=b'3').contains(&b[0])
        || ![1, 3]
            .iter()
            .all(|&i| (b'a'..=b'd').contains(&b[i]) && (b'1'..=b'4').contains(&b[i + 1]))
    {
        return Err(MoveError::Malformed);
    }
    Ok(Move {
        board: BoardId(b[0] - b'0'),
        from: Square((b[2] - b'1') * 4 + b[1] - b'a'),
        to: Square((b[4] - b'1') * 4 + b[3] - b'a'),
    })
}
impl FromStr for Move {
    type Err = MoveError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_move(s.as_bytes())
    }
}
impl FromStr for Turn {
    type Err = MoveError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        if b.len() != 11 || b[5] != b':' {
            return Err(MoveError::Malformed);
        }
        Ok(Self {
            passive: parse_move(&b[..5])?,
            aggressive: parse_move(&b[6..])?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Win(Player),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    Encoding,
    Board,
    Square,
    Overlap,
    PieceCount,
    ContradictoryWinners,
    CounterLimit,
    Sfen,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    Malformed,
    Geometry,
    WrongHomeboard,
    BoardColor,
    WrongStone,
    Blocked,
    Terminal,
    CounterLimit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferTooSmall {
    pub required: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UndoMismatch;
macro_rules! error_impl {
    ($($t:ty),*) => { $(impl fmt::Display for $t { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") } } impl std::error::Error for $t {})* };
}
error_impl!(ValidationError, MoveError, BufferTooSmall, UndoMismatch);

/// One board's occupancy. State construction validates these public masks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Board {
    pub black: u16,
    pub white: u16,
}
impl Board {
    pub fn stones(self, player: Player) -> u16 {
        match player {
            Player::Black => self.black,
            Player::White => self.white,
        }
    }
    fn mask_mut(&mut self, player: Player) -> &mut u16 {
        match player {
            Player::Black => &mut self.black,
            Player::White => &mut self.white,
        }
    }
    fn occupied(self) -> u16 {
        self.black | self.white
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    boards: [Board; 4],
    ply: u32,
    side: Player,
}
#[derive(Debug)]
pub struct Undo {
    pub(crate) before: State,
    pub(crate) after: State,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PositionKey(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateKey(pub u64);

impl Default for State {
    fn default() -> Self {
        Self {
            boards: [Board {
                black: 0x000f,
                white: 0xf000,
            }; 4],
            ply: 0,
            side: Player::Black,
        }
    }
}
fn square(x: i8, y: i8) -> Option<Square> {
    if (0..4).contains(&x) && (0..4).contains(&y) {
        Some(Square((y * 4 + x) as u8))
    } else {
        None
    }
}
fn displacement(m: Move) -> (i8, i8) {
    (
        m.to.file() as i8 - m.from.file() as i8,
        m.to.0 as i8 / 4 - m.from.0 as i8 / 4,
    )
}
impl State {
    pub fn from_bitboards(
        boards: [Board; 4],
        side: Player,
        ply: u32,
    ) -> Result<Self, ValidationError> {
        if ply > MAX_PLY {
            return Err(ValidationError::CounterLimit);
        }
        let (mut eb, mut ew) = (false, false);
        for b in boards {
            if b.black & b.white != 0 {
                return Err(ValidationError::Overlap);
            }
            if b.black.count_ones() > 4 || b.white.count_ones() > 4 {
                return Err(ValidationError::PieceCount);
            }
            eb |= b.black == 0;
            ew |= b.white == 0;
        }
        if eb && ew {
            return Err(ValidationError::ContradictoryWinners);
        }
        Ok(Self { boards, side, ply })
    }
    pub fn board(&self, board: BoardId) -> Board {
        self.boards[board.index()]
    }
    pub fn bitboards(&self) -> [Board; 4] {
        self.boards
    }
    pub fn at(&self, board: BoardId, square: Square) -> Option<Player> {
        let b = self.board(board);
        if b.black & square.bit() != 0 {
            Some(Player::Black)
        } else if b.white & square.bit() != 0 {
            Some(Player::White)
        } else {
            None
        }
    }
    pub fn stone(&self, board: usize, sq: usize) -> Option<Player> {
        self.at(
            BoardId::new(u8::try_from(board).ok()?).ok()?,
            Square::new(u8::try_from(sq).ok()?).ok()?,
        )
    }
    pub fn side_to_move(&self) -> Player {
        self.side
    }
    pub fn ply(&self) -> u32 {
        self.ply
    }
    fn empty_winner(&self) -> Option<Player> {
        if self.boards.iter().any(|b| b.black == 0) {
            Some(Player::White)
        } else if self.boards.iter().any(|b| b.white == 0) {
            Some(Player::Black)
        } else {
            None
        }
    }
    fn leg(&mut self, m: Move, dx: i8, dy: i8, n: i8, aggressive: bool) -> Result<(), MoveError> {
        let b = &mut self.boards[m.board.index()];
        if b.stones(self.side) & m.from.bit() == 0 {
            return Err(MoveError::WrongStone);
        }
        let mut pos = m.from;
        let mut pushed: Option<Option<Square>> = None;
        for _ in 0..n {
            let next =
                square(pos.file() as i8 + dx, pos.0 as i8 / 4 + dy).ok_or(MoveError::Geometry)?;
            if b.occupied() & next.bit() != 0 {
                if !aggressive
                    || b.stones(self.side) & next.bit() != 0
                    || pushed.is_some_and(|p| p != Some(next))
                {
                    return Err(MoveError::Blocked);
                }
                let target = square(next.file() as i8 + dx, next.0 as i8 / 4 + dy);
                if target.is_some_and(|s| b.occupied() & s.bit() != 0) {
                    return Err(MoveError::Blocked);
                }
                *b.mask_mut(self.side.opponent()) &= !next.bit();
                if let Some(s) = target {
                    *b.mask_mut(self.side.opponent()) |= s.bit();
                }
                pushed = Some(target);
            }
            *b.mask_mut(self.side) &= !pos.bit();
            *b.mask_mut(self.side) |= next.bit();
            pos = next;
        }
        Ok(())
    }
    fn raw(&self, turn: Turn) -> Result<Self, MoveError> {
        let p = turn.passive;
        let a = turn.aggressive;
        let (dx, dy) = displacement(p);
        let n = dx.abs().max(dy.abs());
        if !(1..=2).contains(&n)
            || (dx != 0 && dx.abs() != n)
            || (dy != 0 && dy.abs() != n)
            || displacement(a) != (dx, dy)
        {
            return Err(MoveError::Geometry);
        }
        if p.board.0 / 2 != self.side as u8 {
            return Err(MoveError::WrongHomeboard);
        }
        if p.board.0 % 2 == a.board.0 % 2 {
            return Err(MoveError::BoardColor);
        }
        let mut next = *self;
        next.leg(p, dx / n, dy / n, n, false)?;
        next.leg(a, dx / n, dy / n, n, true)?;
        next.side = self.side.opponent();
        next.ply = self.ply.saturating_add(1).min(MAX_PLY);
        Ok(next)
    }
    pub fn played(&self, turn: Turn) -> Result<Self, MoveError> {
        if self.ply == MAX_PLY {
            return Err(MoveError::CounterLimit);
        }
        if self.empty_winner().is_some() {
            return Err(MoveError::Terminal);
        }
        // A blocked player cannot pass raw validation of any complete turn.
        if self.visit(false, |_| ControlFlow::Break(())).is_continue() {
            return Err(MoveError::Terminal);
        }
        self.raw(turn)
    }
    pub fn apply(&mut self, turn: Turn) -> Result<Undo, MoveError> {
        let after = self.played(turn)?;
        let before = *self;
        *self = after;
        Ok(Undo { before, after })
    }
    pub fn undo(&mut self, token: Undo) -> Result<(), UndoMismatch> {
        if *self != token.after {
            return Err(UndoMismatch);
        }
        *self = token.before;
        Ok(())
    }
    fn visit<B>(
        &self,
        limit: bool,
        mut visitor: impl FnMut(Turn) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        if self.empty_winner().is_some() || (limit && self.ply == MAX_PLY) {
            return ControlFlow::Continue(());
        }
        // File before rank gives exactly ASCII notation order. Destination and
        // source loops are structural; raw validation never parses or allocates.
        for pb in (self.side as u8 * 2)..(self.side as u8 * 2 + 2) {
            for pf in 0..4 {
                for pr in 0..4 {
                    let from = Square(pr * 4 + pf);
                    if self.boards[pb as usize].stones(self.side) & from.bit() == 0 {
                        continue;
                    }
                    for tf in 0..4 {
                        for tr in 0..4 {
                            let to = Square(tr * 4 + tf);
                            let p = Move {
                                board: BoardId(pb),
                                from,
                                to,
                            };
                            let (dx, dy) = displacement(p);
                            let n = dx.abs().max(dy.abs());
                            if !(1..=2).contains(&n)
                                || (dx != 0 && dx.abs() != n)
                                || (dy != 0 && dy.abs() != n)
                            {
                                continue;
                            }
                            let mut passive = *self;
                            if passive.leg(p, dx / n, dy / n, n, false).is_err() {
                                continue;
                            }
                            for ab in 0..4 {
                                if pb % 2 == ab % 2 {
                                    continue;
                                }
                                for af in 0..4 {
                                    for ar in 0..4 {
                                        let from = Square(ar * 4 + af);
                                        if self.boards[ab as usize].stones(self.side) & from.bit()
                                            == 0
                                        {
                                            continue;
                                        }
                                        if let Some(to) = square(af as i8 + dx, ar as i8 + dy) {
                                            let turn = Turn {
                                                passive: p,
                                                aggressive: Move {
                                                    board: BoardId(ab),
                                                    from,
                                                    to,
                                                },
                                            };
                                            let mut next = passive;
                                            if next
                                                .leg(turn.aggressive, dx / n, dy / n, n, true)
                                                .is_ok()
                                                && let ControlFlow::Break(value) = visitor(turn)
                                            {
                                                return ControlFlow::Break(value);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
    pub fn visit_legal_turns<B>(
        &self,
        visitor: impl FnMut(Turn) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        self.visit(true, visitor)
    }
    pub fn legal_turns_into(&self, output: &mut [Turn]) -> Result<usize, BufferTooSmall> {
        let mut required = 0;
        let _: ControlFlow<()> = self.visit_legal_turns(|_| {
            required += 1;
            ControlFlow::Continue(())
        });
        if output.len() < required {
            return Err(BufferTooSmall { required });
        }
        let mut i = 0;
        let _: ControlFlow<()> = self.visit_legal_turns(|t| {
            output[i] = t;
            i += 1;
            ControlFlow::Continue(())
        });
        Ok(required)
    }
    pub fn outcome(&self) -> Outcome {
        if let Some(p) = self.empty_winner() {
            Outcome::Win(p)
        } else if self.visit(false, |_| ControlFlow::Break(())).is_continue() {
            Outcome::Win(self.side.opponent())
        } else {
            Outcome::Ongoing
        }
    }
    pub fn winner(&self) -> Option<Player> {
        match self.outcome() {
            Outcome::Ongoing => None,
            Outcome::Win(p) => Some(p),
        }
    }
    pub fn result(&self) -> String {
        match self.outcome() {
            Outcome::Ongoing => "ongoing",
            Outcome::Win(Player::Black) => "b",
            Outcome::Win(Player::White) => "w",
        }
        .into()
    }
    pub fn legal(&self) -> Vec<String> {
        let mut turns = Vec::new();
        let _: ControlFlow<()> = self.visit_legal_turns(|t| {
            turns.push(t.to_string());
            ControlFlow::Continue(())
        });
        turns
    }
    pub fn play(&self, text: &str) -> Result<Self, crate::Error> {
        let turn = text.parse::<Turn>().map_err(|_| "syntax")?;
        self.played(turn).map_err(|_| "illegal")
    }
    pub fn parse(text: &str) -> Result<Self, crate::Error> {
        Self::parse_sfen(text).map_err(|_| "sfen")
    }
    pub fn parse_sfen(text: &str) -> Result<Self, ValidationError> {
        let bad = ValidationError::Sfen;
        if text.len() > 4096 {
            return Err(bad);
        }
        let mut parts = text.split(' ');
        let layout = parts.next().ok_or(bad)?;
        let side = match parts.next() {
            Some("b") => Player::Black,
            Some("w") => Player::White,
            _ => return Err(bad),
        };
        let counter = parts.next().ok_or(bad)?;
        if parts.next().is_some()
            || counter.is_empty()
            || counter.len() > 10
            || !counter.bytes().all(|b| b.is_ascii_digit())
            || (counter.len() > 1 && counter.starts_with('0'))
        {
            return Err(bad);
        }
        let ply = counter.parse::<u32>().map_err(|_| bad)?;
        let mut boards = [Board::default(); 4];
        let mut encoded = layout.split('|');
        for board in &mut boards {
            let mut ranks = encoded.next().ok_or(bad)?.split('/');
            for y in (0..4).rev() {
                let rank = ranks.next().ok_or(bad)?;
                let (mut x, mut digit) = (0, false);
                for c in rank.bytes() {
                    match c {
                        b'1'..=b'4' => {
                            if digit {
                                return Err(bad);
                            }
                            x += c - b'0';
                            digit = true;
                        }
                        b'B' | b'W' => {
                            if x >= 4 {
                                return Err(bad);
                            }
                            *board.mask_mut(if c == b'B' {
                                Player::Black
                            } else {
                                Player::White
                            }) |= 1 << (y * 4 + x);
                            x += 1;
                            digit = false;
                        }
                        _ => return Err(bad),
                    }
                    if x > 4 {
                        return Err(bad);
                    }
                }
                if x != 4 {
                    return Err(bad);
                }
            }
            if ranks.next().is_some() {
                return Err(bad);
            }
        }
        if encoded.next().is_some() {
            return Err(bad);
        }
        Self::from_bitboards(boards, side, ply)
    }
    pub fn sfen(&self) -> String {
        let mut out = String::new();
        for bid in 0..4 {
            if bid > 0 {
                out.push('|');
            }
            for y in (0..4).rev() {
                if y < 3 {
                    out.push('/');
                }
                let mut empty = 0;
                for x in 0..4 {
                    match self.at(BoardId(bid), Square(y * 4 + x)) {
                        None => empty += 1,
                        Some(p) => {
                            if empty > 0 {
                                out.push((b'0' + empty) as char);
                            }
                            empty = 0;
                            out.push(if p == Player::Black { 'B' } else { 'W' });
                        }
                    }
                }
                if empty > 0 {
                    out.push((b'0' + empty) as char);
                }
            }
        }
        use fmt::Write;
        write!(
            &mut out,
            " {} {}",
            if self.side == Player::Black { 'b' } else { 'w' },
            self.ply
        )
        .expect("string writer");
        out
    }
    /// Zobrist v1: board-major, player-major, square a1..d4, then White side.
    /// Seed 0x43414e495f4b4559 ("CANI_KEY"); SplitMix64's published constants.
    /// Collisions are possible. This key excludes ply and its play limit.
    pub fn position_key(&self) -> PositionKey {
        let mut hash = 0;
        for (board, b) in self.boards.iter().enumerate() {
            for (player, mut mask) in [b.black, b.white].into_iter().enumerate() {
                while mask != 0 {
                    let sq = mask.trailing_zeros() as usize;
                    mask &= mask - 1;
                    hash ^= ZOBRIST[board * 32 + player * 16 + sq];
                }
            }
        }
        if self.side == Player::White {
            hash ^= ZOBRIST[128];
        }
        PositionKey(hash)
    }
    /// Includes ply; still a hash, not an equality proof.
    pub fn state_key(&self) -> StateKey {
        StateKey(self.position_key().0 ^ mix(ZOBRIST[129] ^ self.ply as u64))
    }
}
const fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
const fn zobrist() -> [u64; 130] {
    let mut result = [0; 130];
    let mut seed: u64 = 0x43414e495f4b4559;
    let mut i = 0;
    while i < 130 {
        seed = seed.wrapping_add(0x9e3779b97f4a7c15);
        result[i] = mix(seed);
        i += 1;
    }
    result
}
const ZOBRIST: [u64; 130] = zobrist();
