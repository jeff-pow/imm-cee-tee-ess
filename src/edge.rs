use crate::chess_move::Move;

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    m: Move,
    visits: i32,
    child_ptr: Option<usize>,
    total_score: f32,
}

impl Edge {
    pub const fn new(m: Move, child_ptr: Option<usize>) -> Self {
        Self { m, child_ptr, visits: 0, total_score: 0. }
    }

    pub fn q(&self) -> f32 {
        assert_ne!(0, self.visits, "User must specify value they want if node hasn't been visited before.");
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

    pub const fn child(&self) -> Option<usize> {
        self.child_ptr
    }

    pub fn set_child(&mut self, child_ptr: Option<usize>) {
        self.child_ptr = child_ptr;
    }
}
