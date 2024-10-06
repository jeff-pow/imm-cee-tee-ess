use super::{util::update, INPUT_SIZE, L1_SIZE, NET};

use crate::{
    board::Board,
    types::pieces::{Color, NUM_PIECES},
};
use arrayvec::ArrayVec;
/**
* When changing activation functions, both the normalization factor and QA may need to change
* alongside changing the crelu calls to screlu in simd and serial code.
*/
const QA: i16 = 255;
const QB: i16 = 64;
pub(super) const QAB: i16 = QA * QB;

pub(super) const SCALE: i32 = 400;

#[derive(Debug)]
pub(super) struct Layer<const M: usize, const N: usize, T> {
    pub(super) weights: [[T; N]; M],
    pub(super) bias: [T; N],
}

impl<const M: usize, const N: usize> Layer<M, N, i16> {
    fn transform(&self, board: &Board) -> [[i16; N]; 2] {
        let mut output = [self.bias; 2];
        let mut threats = board.threats(!board.stm);
        let mut defenders = board.threats(board.stm);
        for view in Color::iter() {
            if view != board.stm {
                threats = threats.flip_vertical();
                defenders = defenders.flip_vertical();
            }
            let mut vec: ArrayVec<usize, 32> = ArrayVec::new();
            for sq in board.occupancies() {
                let p = board.piece_at(sq);

                vec.push({
                    const COLOR_OFFSET: usize = 64 * NUM_PIECES;
                    const PIECE_OFFSET: usize = 64;

                    let map_feature = |feat| {
                        2 * INPUT_SIZE * usize::from(defenders.contains(sq))
                            + INPUT_SIZE * usize::from(threats.contains(sq))
                            + feat
                    };

                    match view {
                        Color::White => map_feature(
                            usize::from(p.color()) * COLOR_OFFSET
                                + usize::from(p.name()) * PIECE_OFFSET
                                + usize::from(sq),
                        ),
                        Color::Black => map_feature(
                            usize::from(!p.color()) * COLOR_OFFSET
                                + usize::from(p.name()) * PIECE_OFFSET
                                + usize::from(sq.flip_vertical()),
                        ),
                    }
                });
            }
            update(&mut output[view], &vec, &[]);
        }

        // Activate it
        for o in output.iter_mut().flatten() {
            *o = (i32::from(*o).clamp(0, i32::from(QA)).pow(2)) as i16;
        }

        output
    }
}

#[derive(Debug)]
#[repr(C)]
pub(super) struct PerspectiveLayer<const M: usize, const N: usize, T> {
    pub(super) weights: [[[T; N]; M]; 2],
    pub(super) bias: [T; N],
}

impl<const M: usize, const N: usize> PerspectiveLayer<M, N, i16> {
    fn forward(&self, input: [[i16; M]; 2], stm: Color) -> [f32; N] {
        let mut output = [0; N];

        for c in Color::iter() {
            for (&i, col) in input[usize::from(c == stm)]
                .iter()
                .zip(self.weights[usize::from(c == stm)].iter())
            {
                for (o, c) in output.iter_mut().zip(col.iter()) {
                    *o += c * i;
                }
            }
        }

        let mut float = [0.0; N];

        for (f, (&o, &b)) in float.iter_mut().zip(output.iter().zip(self.bias.iter())) {
            *f = (o as f32 / QA as f32 + b as f32) / QAB as f32;
        }

        float
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

#[derive(Debug)]
#[repr(C, align(64))]
pub(super) struct Network {
    pub(super) ft: Layer<{ INPUT_SIZE * 4 }, L1_SIZE, i16>,
    pub(super) l1: PerspectiveLayer<L1_SIZE, 16, i16>,
    pub(super) l2: Layer<16, 16, f32>,
    pub(super) l3: Layer<16, 1, f32>,
}

impl Board {
    pub fn raw_evaluate(&self) -> i32 {
        let ft = NET.ft.transform(self);
        let l1 = NET.l1.forward(ft, self.stm);
        let l2 = NET.l2.forward(l1);
        let l3 = NET.l3.forward(l2);
        (l3[0] * SCALE as f32) as i32
    }

    /// Credit to viridithas for these values and concepts
    pub fn scaled_evaluate(&self) -> i32 {
        let raw = self.raw_evaluate();
        let eval = raw * self.mat_scale() / 1024;
        eval * (200 - i32::from(self.half_moves)) / 200
    }
}

//impl Network {
//    pub fn feature_idx(piece: Piece, sq: Square, view: Color, board: &Board) -> usize {
//        const COLOR_OFFSET: usize = 64 * NUM_PIECES;
//        const PIECE_OFFSET: usize = 64;
//        // TODO: This totally needs adjustment
//        let (threats, defenders) = (board.threats(!board.stm), board.threats(board.stm));
//
//        let map_feature = |feat, threats: Bitboard, defenders: Bitboard| {
//            2 * INPUT_SIZE * usize::from(defenders.contains(sq.into()))
//                + INPUT_SIZE * usize::from(threats.contains(sq.into()))
//                + feat
//        };
//
//        match view {
//            Color::White => map_feature(
//                usize::from(piece.color()) * COLOR_OFFSET + usize::from(piece.name()) * PIECE_OFFSET + usize::from(sq),
//                threats,
//                defenders,
//            ),
//            Color::Black => map_feature(
//                usize::from(!piece.color()) * COLOR_OFFSET
//                    + usize::from(piece.name()) * PIECE_OFFSET
//                    + usize::from(sq.flip_vertical()),
//                threats,
//                defenders,
//            ),
//        }
//    }
//}

fn screlu(x: f32) -> f32 {
    x.clamp(0., 1.).powi(2)
}
