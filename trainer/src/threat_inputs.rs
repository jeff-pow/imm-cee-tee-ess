use bullet::{
    format::{chess::BoardIter, ChessBoard},
    inputs::InputType,
};
use imm_cee_tee_ess::{
    board::Board,
    types::{bitboard::Bitboard, pieces::Color, square::Square},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct ThreatInput;

impl InputType for ThreatInput {
    type RequiredDataType = ChessBoard;
    type FeatureIter = ThreatIter;

    fn max_active_inputs(&self) -> usize {
        32
    }

    fn inputs(&self) -> usize {
        768 * 4
    }

    fn buckets(&self) -> usize {
        1
    }

    fn feature_iter(&self, pos: &Self::RequiredDataType) -> Self::FeatureIter {
        let mut pieces = [Bitboard::EMPTY; 6];
        let mut colors = [Bitboard::EMPTY; 2];
        for (piece, sq) in pos.into_iter() {
            let sq = Square(sq);
            let c = usize::from(piece & 8 > 0);
            let pc = usize::from(piece & 7);
            pieces[pc] |= sq.bitboard();
            colors[c] |= sq.bitboard();
        }
        // Bulletformat is always stm relative, so white is stm
        let board = Board::from_bbs(pieces, colors, Color::White);
        let threats = board.threats(board.stm);
        let defenders = board.threats(!board.stm);
        ThreatIter {
            board_iter: pos.into_iter(),
            threats,
            defenders,
        }
    }

    fn size(&self) -> usize {
        self.inputs() * self.buckets()
    }
}

pub struct ThreatIter {
    board_iter: BoardIter,
    threats: Bitboard,
    defenders: Bitboard,
}

impl Iterator for ThreatIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.board_iter.next().map(|(piece, square)| {
            let c = usize::from(piece & 8 > 0);
            let p = 64 * usize::from(piece & 7);
            let sq = usize::from(square);

            let map_feature = |feat, threats: Bitboard, defenders: Bitboard| {
                2 * 768 * usize::from(defenders.contains(sq.into()))
                    + 768 * usize::from(threats.contains(sq.into()))
                    + feat
            };

            let stm_feat = [0, 384][c] + p + sq;
            let xstm_feat = [384, 0][c] + p + (sq ^ 56);

            (
                map_feature(stm_feat, self.threats, self.defenders),
                map_feature(xstm_feat, self.threats.flip_vertical(), self.defenders.flip_vertical()),
            )
        })
    }
}
