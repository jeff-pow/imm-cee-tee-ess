use crate::{board::Board, chess_move::Move, historized_board::HistorizedBoard, movegen::MAX_MOVES};
use arrayvec::ArrayVec;

impl Board {
    pub fn policies(&self) -> ArrayVec<(Move, f32), { MAX_MOVES }> {
        let mut policies = ArrayVec::<(Move, f32), 256>::new_const();
        let mut denom = 0.0;

        // Softmax
        let legal_moves = self.legal_moves();
        legal_moves.iter().for_each(|m| {
            // Bet you've never seen hand crafted policy before :)
            let pol = f32::from(self.see(*m, -100)) + f32::from(self.see(*m, 1));
            policies.push((*m, pol));
            denom += pol.exp();
        });
        policies.iter_mut().for_each(|(_, pol)| *pol = pol.exp() / denom);
        policies
    }
}

impl HistorizedBoard {
    pub fn policies(&self) -> ArrayVec<(Move, f32), { MAX_MOVES }> {
        self.board().policies()
    }
}
