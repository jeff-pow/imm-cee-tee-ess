use crate::game_time::Clock;
use std::time::Instant;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchType {
    /// User has requested a search until a particular depth
    /// The u64 allows for a node maximum to be established to keep bench from going for too long
    Depth(u64, u64),
    /// Search determines how much time to allow itself.
    Time(Clock),
    /// Only search for N nodes
    Nodes(u64),
    /// Search for a mate at the provided depth
    Mate(i32),
    #[default]
    /// Search forever
    Infinite,
}

impl SearchType {
    pub fn should_stop(&self, nodes: u64, search_start: &Instant, depth: u64) -> bool {
        match self {
            Self::Depth(d, n) => depth > *d || nodes >= *n,
            Self::Time(clock) => clock.soft_termination(search_start),
            Self::Nodes(n) => nodes >= *n,
            Self::Mate(_) => todo!("Mate search not implemented yet"),
            Self::Infinite => false,
        }
    }

    pub fn hard_stop(&self, search_start: &Instant) -> bool {
        match self {
            Self::Time(clock) => clock.hard_termination(search_start),
            _ => false,
        }
    }
}
