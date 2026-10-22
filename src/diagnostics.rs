//! Allocation-free, bounded game-tree diagnostics. No playing policy or cache.
use crate::State;
use std::{fmt, ops::ControlFlow};
pub const MAX_PERFT_DEPTH: u8 = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerftError {
    DepthLimit,
    NodeLimit,
}
impl fmt::Display for PerftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PerftError {}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerftStats {
    pub nodes: u64,
    pub leaves: u64,
}
/// Count nodes (including root) and leaves at exactly `depth`. Depth zero has
/// one leaf even at terminal positions; terminal branches above zero have none.
/// The mandatory node budget bounds work; errors publish no partial statistics.
/// Uses bounded recursion and no heap allocation, history stack, or transposition table.
pub fn perft(state: &State, depth: u8, node_limit: u64) -> Result<PerftStats, PerftError> {
    fn walk(
        state: &State,
        depth: u8,
        limit: u64,
        stats: &mut PerftStats,
    ) -> Result<(), PerftError> {
        if stats.nodes == limit {
            return Err(PerftError::NodeLimit);
        }
        stats.nodes += 1;
        if depth == 0 {
            stats.leaves += 1;
            return Ok(());
        }
        match state.visit_legal_turns(|turn| {
            let next = state.played(turn).expect("generated legal turn");
            match walk(&next, depth - 1, limit, stats) {
                Ok(()) => ControlFlow::Continue(()),
                Err(e) => ControlFlow::Break(e),
            }
        }) {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(e) => Err(e),
        }
    }
    if depth > MAX_PERFT_DEPTH {
        return Err(PerftError::DepthLimit);
    }
    let mut result = PerftStats::default();
    walk(state, depth, node_limit, &mut result)?;
    Ok(result)
}
