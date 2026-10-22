//! Rules automorphisms: uniform square D4, global color-board swap, and
//! homeboard swap coupled to player relabeling. No independent board rotations.
use crate::{Board, BoardId, Move, Player, Square, State, Turn, ValidationError};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symmetry(u8);
impl Symmetry {
    pub const COUNT: u8 = 32;
    pub const fn new(id: u8) -> Result<Self, ValidationError> {
        if id < 32 {
            Ok(Self(id))
        } else {
            Err(ValidationError::Encoding)
        }
    }
    pub const fn index(self) -> u8 {
        self.0
    }
    pub fn square(self, square: Square) -> Square {
        Square::new(SQUARE_MAP[(self.0 & 7) as usize][square.index()]).unwrap()
    }
    fn mask(self, mut source: u16) -> u16 {
        let map = &SQUARE_MAP[(self.0 & 7) as usize];
        let mut result = 0;
        while source != 0 {
            result |= 1 << map[source.trailing_zeros() as usize];
            source &= source - 1;
        }
        result
    }
    pub fn board(self, board: BoardId) -> BoardId {
        BoardId::new(board.index() as u8 ^ (self.0 >> 3)).unwrap()
    }
    pub fn player(self, player: Player) -> Player {
        if self.0 & 16 != 0 {
            player.opponent()
        } else {
            player
        }
    }
    pub fn turn(self, turn: Turn) -> Turn {
        let leg = |m: Move| Move {
            board: self.board(m.board),
            from: self.square(m.from),
            to: self.square(m.to),
        };
        Turn {
            passive: leg(turn.passive),
            aggressive: leg(turn.aggressive),
        }
    }
    pub fn position(self, state: State) -> State {
        let mut boards = [Board::default(); 4];
        for (b, source) in state.bitboards().into_iter().enumerate() {
            let target = b ^ (self.0 >> 3) as usize;
            let (black, white) = (self.mask(source.black), self.mask(source.white));
            boards[target] = if self.0 & 16 == 0 {
                Board { black, white }
            } else {
                Board {
                    black: white,
                    white: black,
                }
            };
        }
        State::from_bitboards(boards, self.player(state.side_to_move()), state.ply())
            .expect("rules automorphism")
    }
    pub fn inverse(self) -> Self {
        let rotation = if self.0 & 4 != 0 {
            self.0 & 3
        } else {
            (4 - (self.0 & 3)) & 3
        };
        Self((self.0 & !3) | rotation)
    }
}

// 128 bytes, generated at compile time. Uniform square permutations preserve
// every existing symmetry ID; iterate occupied bits instead of all squares.
const SQUARE_MAP: [[u8; 16]; 8] = {
    let mut map = [[0; 16]; 8];
    let mut id = 0;
    while id < 8 {
        let mut q = 0;
        while q < 16 {
            let (mut x, mut y) = (q % 4, q / 4);
            if id & 4 != 0 {
                x = 3 - x;
            }
            let mut n = 0;
            while n < (id & 3) {
                (x, y) = (3 - y, x);
                n += 1;
            }
            map[id][q] = (y * 4 + x) as u8;
            q += 1;
        }
        id += 1;
    }
    map
};
pub const CANONICAL_VERSION: u32 = 1;
/// The mapping is from the input position TO this representative. Use its
/// inverse to return cached turns to the original position. Player labels may swap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanonicalPosition {
    pub position: State,
    pub to_canonical: Symmetry,
}
/// Stable v1 representative: smallest numeric board-major Black/White mask
/// tuple, then side; ties use the lowest symmetry ID. Ply is preserved but does
/// not participate in ordering. Selection never depends on hash values.
pub fn canonicalize(state: State) -> CanonicalPosition {
    fn order(s: State) -> ([u16; 8], u8) {
        let boards = s.bitboards();
        (
            std::array::from_fn(|i| {
                if i % 2 == 0 {
                    boards[i / 2].black
                } else {
                    boards[i / 2].white
                }
            }),
            s.side_to_move() as u8,
        )
    }
    let mut best = CanonicalPosition {
        position: state,
        to_canonical: Symmetry(0),
    };
    let mut smallest = order(state);
    for id in 1..Symmetry::COUNT {
        let symmetry = Symmetry(id);
        let candidate = symmetry.position(state);
        let key = order(candidate);
        if key < smallest {
            smallest = key;
            best = CanonicalPosition {
                position: candidate,
                to_canonical: symmetry,
            };
        }
    }
    best
}
