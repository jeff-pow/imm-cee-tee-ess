use crate::game_time::Clock;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchType {
    /// User has requested a search until a particular depth
    Depth(u64),
    /// Search determines how much time to allow itself.
    Time(Clock),
    /// User has requested a search for a certain amount of time
    MoveTime(Duration),
    /// Only search for N nodes
    Nodes(u64),
    /// Search for a mate at the provided ply. UCI spec says search to a depth,
    /// but I convert to a ply when parsing because ply is much easier to
    /// think about...
    Mate(u64),
    #[default]
    /// Search forever
    Infinite,
}

impl SearchType {
    pub fn should_stop(&self, nodes: u64, search_start: &Instant, depth: u64, mate_ply: Option<u8>) -> bool {
        match self {
            Self::Depth(d) => depth >= *d,
            Self::Time(clock) => {
                nodes % 256 == 0 && clock.hard_termination(search_start)
                    || nodes % 4096 == 0 && clock.soft_termination(search_start)
            }
            Self::MoveTime(dur) => nodes % 256 == 0 && search_start.elapsed() > *dur,
            Self::Nodes(n) => nodes >= *n,
            Self::Mate(ply) => {
                if let Some(mate_ply) = mate_ply {
                    mate_ply <= *ply as u8
                } else {
                    false
                }
            }
            Self::Infinite => false,
        }
    }
}
