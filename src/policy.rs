use crate::{board::Board, chess_move::Move, historized_board::HistorizedBoard, movegen::MAX_MOVES};
use arrayvec::ArrayVec;

impl Board {
    pub fn policies(&self) -> ArrayVec<(Move, f32), { MAX_MOVES }> {
        let mut policies = ArrayVec::<(Move, f32), 256>::new_const();
        let mut total = 0.0;

        // Softmax
        let legal_moves = self.legal_moves();
        legal_moves.iter().for_each(|m| {
            // Bet you've never seen hand crafted policy before :)
            let pol = 1. / legal_moves.len() as f32
                + 0.05 * f32::from(self.see(*m, -100))
                + 0.05 * f32::from(self.see(*m, 1));
            policies.push((*m, pol));
            total += pol.exp();
        });
        policies.iter_mut().for_each(|(_, pol)| *pol = pol.exp() / total);
        policies
    }
}

impl HistorizedBoard {
    pub fn policies(&self) -> ArrayVec<(Move, f32), { MAX_MOVES }> {
        self.board().policies()
    }
}
