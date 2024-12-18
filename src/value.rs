use crate::board::Board;

pub const SCALE: f32 = 400.;

impl Board {
    /// Uses a sigmoid to scale an integer evaluation from 0.0 to 1.0 using a sigmoid
    pub fn wdl(&self) -> f32 {
        1.0 / (1.0 + (-self.scaled_eval() as f32 / SCALE).exp())
    }
}
