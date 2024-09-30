use crate::board::Board;

pub const SCALE: f32 = 400.;

impl Board {
    /// Uses a sigmoid to scale an integer evaluation from 0.0 to 1.0
    pub fn scaled_eval(&self) -> f32 {
        let acc = self.new_accumulator();
        let eval = acc.scaled_evaluate(self);
        1.0 / (1.0 + (-eval as f32 / SCALE).exp())
    }
}
