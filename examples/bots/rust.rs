//! External bot policy, deliberately outside the SDK's rules/protocol modules.
use cani::{MoveSelector, Outcome, Player, State, Turn};
#[derive(Clone, Copy)]
struct Entry {
    key: u64,
    position: State,
    black_score: i32,
}
struct Bot {
    cache: [Option<Entry>; 64],
}
impl Bot {
    fn evaluate(&mut self, position: State) -> i32 {
        let key = position.position_key().0;
        let slot = (key & 63) as usize;
        if let Some(entry) = self.cache[slot]
            && entry.key == key
            && entry.position == position
        {
            return entry.black_score;
        }
        let score = match position.outcome() {
            Outcome::Win(Player::Black) => 10_000,
            Outcome::Win(Player::White) => -10_000,
            Outcome::Ongoing => position
                .bitboards()
                .iter()
                .map(|b| b.black.count_ones() as i32 - b.white.count_ones() as i32)
                .sum(),
        };
        self.cache[slot] = Some(Entry {
            key,
            position,
            black_score: score,
        });
        score
    }
}
impl MoveSelector for Bot {
    fn select(&mut self, position: &State, legal: &[Turn]) -> Turn {
        let sign = if position.side_to_move() == Player::Black {
            1
        } else {
            -1
        };
        let mut best = legal[0];
        let mut score = i32::MIN;
        for &turn in legal {
            let child = position.played(turn).expect("engine supplies legal turns");
            let value = sign * self.evaluate(child);
            if value > score {
                best = turn;
                score = value;
            }
        }
        best
    }
}
fn main() -> std::io::Result<()> {
    cani::run_with_selector(
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        Bot { cache: [None; 64] },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_hash_does_not_replace_full_position_equality() {
        let position = State::default();
        let key = position.position_key().0;
        let different = position.play("0a1a2:1b1b2").unwrap();
        let mut bot = Bot { cache: [None; 64] };
        bot.cache[(key & 63) as usize] = Some(Entry {
            key,
            position: different,
            black_score: 123456,
        });
        assert_eq!(bot.evaluate(position), 0);
        assert_eq!(bot.cache[(key & 63) as usize].unwrap().position, position);
    }
}
