use super::{util::f32_update, INPUT_SIZE, L1_SIZE, NET};

use crate::{
    board::Board,
    types::{bitboard::Bitboard, pieces::Color},
    value::SCALE,
};
use arrayvec::ArrayVec;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct Layer<const M: usize, const N: usize, T> {
    pub(super) weights: [[T; N]; M],
    pub(super) bias: [T; N],
}

impl<const M: usize, const N: usize> Layer<M, N, f32> {
    /// This function returns transformed feature vectors in the order [stm, nstm] instead of the commonly seen
    /// [`Color::White`, `Color::Black`]. This simplifies the calculation of which weights to use in the next function call.
    fn transform(&self, board: &Board) -> [[f32; N]; 2] {
        let mut output = [self.bias; 2];
        let mut stm_feats = ArrayVec::<usize, 32>::new();
        let mut xstm_feats = ArrayVec::<usize, 32>::new();

        let threats = board.threats(!board.stm);
        let defenders = board.threats(board.stm);
        for sq in board.occupancies() {
            let piece = board.piece_at(sq);
            let is_opp = piece.color() != board.stm;
            let map_feature = |feat, threats: Bitboard, defenders: Bitboard| {
                2 * 768 * usize::from(defenders.contains(sq)) + 768 * usize::from(threats.contains(sq)) + feat
            };

            let stm_feat = 384 * usize::from(is_opp)
                + 64 * usize::from(piece.name())
                + if board.stm == Color::White {
                    usize::from(sq)
                } else {
                    usize::from(sq.flip_vertical())
                };
            let xstm_feat = 384 * usize::from(!is_opp)
                + 64 * usize::from(piece.name())
                + if board.stm == Color::Black {
                    usize::from(sq)
                } else {
                    usize::from(sq.flip_vertical())
                };
            stm_feats.push(map_feature(stm_feat, threats, defenders));
            xstm_feats.push(map_feature(xstm_feat, defenders, threats));
        }

        f32_update(&mut output[0], &stm_feats, &[]);
        f32_update(&mut output[1], &xstm_feats, &[]);
        output
    }
}

impl<const M: usize, const N: usize> Layer<M, N, f32> {
    fn forward(&self, input: [f32; M]) -> [f32; N] {
        let mut output = self.bias;
        for (&i, col) in input.iter().zip(self.weights.iter()) {
            for (o, c) in output.iter_mut().zip(col.iter()) {
                *o += c * screlu(i);
            }
        }
        output
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(super) struct PerspectiveLayer<const M: usize, const N: usize, T> {
    pub(super) weights: [[[T; N]; M]; 2],
    pub(super) bias: [T; N],
}

impl<const M: usize, const N: usize> PerspectiveLayer<M, N, f32> {
    fn forward(&self, input: [[f32; M]; 2]) -> [f32; N] {
        let mut output = self.bias;

        for (input, weights) in input.iter().zip(self.weights.iter()) {
            for (&i, col) in input.iter().zip(weights.iter()) {
                for (o, c) in output.iter_mut().zip(col.iter()) {
                    *o += c * screlu(i);
                }
            }
        }

        output
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Network {
    pub(super) ft: Layer<INPUT_SIZE, L1_SIZE, f32>,
    pub(super) l1: PerspectiveLayer<L1_SIZE, 16, f32>,
    pub(super) l2: Layer<16, 16, f32>,
    pub(super) l3: Layer<16, 16, f32>,
    pub(super) l4: Layer<16, 16, f32>,
    pub(super) l5: Layer<16, 1, f32>,
}

impl Board {
    pub fn raw_eval(&self) -> f32 {
        let ft = NET.ft.transform(self);
        let l1 = NET.l1.forward(ft);
        let l2 = NET.l2.forward(l1);
        let l3 = NET.l3.forward(l2);
        let l4 = NET.l4.forward(l3);
        let l5 = NET.l5.forward(l4);
        l5[0] * SCALE
    }

    /// Credit to viridithas for these values and concepts
    pub fn scaled_eval(&self) -> i32 {
        let raw = self.raw_eval() as i32;
        raw * self.mat_scale() / 1024
    }
}

fn screlu(x: f32) -> f32 {
    x.clamp(0., 1.).powi(2)
}
