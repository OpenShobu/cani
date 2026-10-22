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
        let (mut x, mut y) = (square.file(), square.rank() - 1);
        if self.0 & 4 != 0 {
            x = 3 - x;
        }
        for _ in 0..(self.0 & 3) {
            (x, y) = (3 - y, x);
        }
        Square::new(y * 4 + x).unwrap()
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
        for b in 0..4 {
            let target = self.board(BoardId::new(b).unwrap()).index();
            let source = state.bitboards()[b as usize];
            for q in 0..16 {
                let bit = 1 << self.square(Square::new(q).unwrap()).index();
                if source.black & (1 << q) != 0 {
                    if self.0 & 16 == 0 {
                        boards[target].black |= bit;
                    } else {
                        boards[target].white |= bit;
                    }
                }
                if source.white & (1 << q) != 0 {
                    if self.0 & 16 == 0 {
                        boards[target].white |= bit;
                    } else {
                        boards[target].black |= bit;
                    }
                }
            }
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
