use crate::{arena::ArenaIndex, chess_move::Move};

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    m: Move,
    visits: i32,
    child_ptr: Option<ArenaIndex>,
    total_score: f32,
    policy: f32,
}

impl Edge {
    pub const fn new(m: Move, child_ptr: Option<ArenaIndex>, policy: f32) -> Self {
        Self {
            m,
            child_ptr,
            visits: 0,
            total_score: 0.,
            policy,
        }
    }

    pub fn q(&self) -> f32 {
        assert_ne!(
            0, self.visits,
            "User must specify FPU if node hasn't been visited before."
        );
        self.total_score / self.visits as f32
    }

    pub fn update_stats(&mut self, u: f32) {
        self.visits += 1;
        self.total_score += u;
    }

    pub const fn m(&self) -> Move {
        self.m
    }

    pub const fn visits(&self) -> i32 {
        self.visits
    }

    pub const fn total_score(&self) -> f32 {
        self.total_score
    }

    pub const fn child(&self) -> Option<ArenaIndex> {
        self.child_ptr
    }

    pub fn set_child(&mut self, child_ptr: Option<ArenaIndex>) {
        self.child_ptr = child_ptr;
    }

    pub const fn policy(&self) -> f32 {
        self.policy
    }
}
