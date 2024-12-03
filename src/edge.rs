use crate::{arena::ArenaIndex, chess_move::Move};

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    m: Move,
    child_ptr: Option<ArenaIndex>,
    policy: f32,
}

impl Edge {
    pub const fn new(m: Move, child_ptr: Option<ArenaIndex>, policy: f32) -> Self {
        Self { m, child_ptr, policy }
    }

    pub const fn m(&self) -> Move {
        self.m
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
