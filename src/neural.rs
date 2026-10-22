//! Neural encoding v1: absolute coordinates, binary occupancy, explicit side and ply.
use crate::{Board, BoardId, Move, MoveError, Player, Square, State, Turn, ValidationError};
pub const VERSION: u32 = 1;
pub const TENSOR_LEN: usize = 129;
pub const ACTION_COUNT: usize = 65536;
/// Eight board-major, Black-then-White occupancy planes of 16 squares, followed
/// by a side scalar (0 Black, 1 White). Ply is separate and must accompany data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionTensor {
    pub version: u32,
    pub cells: [u8; TENSOR_LEN],
    pub ply: u32,
}
impl PositionTensor {
    pub fn encode(state: State) -> Self {
        let mut cells = [0; TENSOR_LEN];
        for (b, board) in state.bitboards().iter().enumerate() {
            for (p, mask) in [board.black, board.white].iter().enumerate() {
                for q in 0..16 {
                    cells[b * 32 + p * 16 + q] = ((mask >> q) & 1) as u8;
                }
            }
        }
        cells[128] = state.side_to_move() as u8;
        Self {
            version: VERSION,
            cells,
            ply: state.ply(),
        }
    }
    pub fn decode(self) -> Result<State, ValidationError> {
        if self.version != VERSION || self.cells.iter().any(|&v| v > 1) {
            return Err(ValidationError::Encoding);
        }
        let mut boards = [Board::default(); 4];
        for (b, board) in boards.iter_mut().enumerate() {
            for q in 0..16 {
                board.black |= (self.cells[b * 32 + q] as u16) << q;
                board.white |= (self.cells[b * 32 + 16 + q] as u16) << q;
            }
        }
        State::from_bitboards(
            boards,
            if self.cells[128] == 0 {
                Player::Black
            } else {
                Player::White
            },
            self.ply,
        )
    }
}
/// Action indices v1 use the SSGF 16-bit packing. Some indices have invalid
/// geometry; many geometrically valid indices are illegal in a given position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionIndex(pub u16);
impl ActionIndex {
    pub fn encode(turn: Turn) -> Result<Self, MoveError> {
        pack_turn(turn).map(Self)
    }
    pub fn decode(self) -> Result<Turn, MoveError> {
        unpack_turn(self.0)
    }
}
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

pub const ACTION_MASK_BYTES: usize = ACTION_COUNT / 8;
/// Little-bit-first policy mask: action i is bit (i % 8) in byte (i / 8).
/// Only generated legal actions are set, including at terminal/counter positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionMask {
    bytes: [u8; ACTION_MASK_BYTES],
}
impl Default for ActionMask {
    fn default() -> Self {
        Self {
            bytes: [0; ACTION_MASK_BYTES],
        }
    }
}
impl ActionMask {
    pub fn for_position(state: &State) -> Self {
        let mut result = Self::default();
        let _: std::ops::ControlFlow<()> = state.visit_legal_turns(|turn| {
            let index = ActionIndex::encode(turn).expect("legal turn geometry").0 as usize;
            result.bytes[index / 8] |= 1 << (index % 8);
            std::ops::ControlFlow::Continue(())
        });
        result
    }
    pub fn as_bytes(&self) -> &[u8; ACTION_MASK_BYTES] {
        &self.bytes
    }
    pub fn contains(&self, index: ActionIndex) -> bool {
        self.bytes[index.0 as usize / 8] & (1 << (index.0 % 8)) != 0
    }
    pub fn count(&self) -> usize {
        self.bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| u64::from_le_bytes(*b).count_ones() as usize)
            .sum()
    }
}
/// Sparse policy actions in existing legal-turn order. Undersized output is
/// untouched; required is an element count. Output past the written prefix stays intact.
pub fn legal_actions_into(
    state: &State,
    output: &mut [ActionIndex],
) -> Result<usize, crate::BufferTooSmall> {
    let mut required = 0;
    let _: std::ops::ControlFlow<()> = state.visit_legal_turns(|_| {
        required += 1;
        std::ops::ControlFlow::Continue(())
    });
    if output.len() < required {
        return Err(crate::BufferTooSmall { required });
    }
    let mut n = 0;
    let _: std::ops::ControlFlow<()> = state.visit_legal_turns(|turn| {
        output[n] = ActionIndex::encode(turn).expect("legal turn geometry");
        n += 1;
        std::ops::ControlFlow::Continue(())
    });
    Ok(required)
}
/// Encode a batch without allocating. Capacity is in tensors; failures leave
/// the entire output untouched and successful writes preserve its unused tail.
pub fn encode_batch_into(
    states: &[State],
    output: &mut [PositionTensor],
) -> Result<usize, crate::BufferTooSmall> {
    if output.len() < states.len() {
        return Err(crate::BufferTooSmall {
            required: states.len(),
        });
    }
    for (input, out) in states.iter().zip(output.iter_mut()) {
        *out = PositionTensor::encode(*input);
    }
    Ok(states.len())
}
