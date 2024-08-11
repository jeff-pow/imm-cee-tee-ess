use crate::{
    arena::{ArenaIndex, FPU},
    chess_move::Move,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    m: Move,
    visits: i32,
    child_ptr: Option<ArenaIndex>,
    total_score: f32,
}

impl Edge {
    pub const fn new(m: Move, child_ptr: Option<ArenaIndex>) -> Self {
        Self {
            m,
            child_ptr,
            visits: 0,
            total_score: 0.,
        }
    }

    pub fn q(&self) -> f32 {
        if self.visits == 0 {
            FPU
        } else {
            self.total_score / self.visits as f32
        }
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

    pub const fn child(&self) -> Option<ArenaIndex> {
        self.child_ptr
    }

    pub fn set_child(&mut self, child_ptr: ArenaIndex) {
        self.child_ptr = Some(child_ptr);
    }
}
