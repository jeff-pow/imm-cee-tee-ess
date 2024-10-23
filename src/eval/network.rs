use super::{util::f32_update, INPUT_SIZE, L1_SIZE, NET};

use crate::{
    board::Board,
    types::{
        bitboard::Bitboard,
        pieces::{Color, NUM_PIECES},
        square::Square,
    },
};
use arrayvec::ArrayVec;

pub(super) const SCALE: i32 = 400;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct Layer<const M: usize, const N: usize, T> {
    pub(super) weights: [[T; N]; M],
    pub(super) bias: [T; N],
}

impl<const M: usize, const N: usize> Layer<M, N, f32> {
    fn transform(&self, board: &Board) -> [[f32; N]; 2] {
        let mut output = [self.bias; 2];
        for view in Color::iter() {
            let mut threats = board.threats(!view);
            let mut defenders = board.threats(view);
            if view == Color::Black {
                threats = threats.flip_vertical();
                defenders = defenders.flip_vertical();
            }
            let mut features: ArrayVec<usize, 32> = ArrayVec::new();
            for sq in board.occupancies() {
                let p = board.piece_at(sq);

                features.push({
                    const COLOR_OFFSET: usize = 64 * NUM_PIECES;
                    const PIECE_OFFSET: usize = 64;

                    let map_feature = |feat| {
                        2 * 768 * usize::from(defenders.contains(sq)) + 768 * usize::from(threats.contains(sq)) + feat
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
            f32_update(&mut output[view], &features, &[]);
        }

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
    fn forward(&self, input: [[f32; M]; 2], stm: Color) -> [f32; N] {
        let mut output = self.bias;

        for c in Color::iter() {
            for (&i, col) in input[usize::from(c == stm)]
                .iter()
                .zip(self.weights[usize::from(c == stm)].iter())
            {
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
    pub fn i32_eval(&self) -> i32 {
        let raw = self.raw_evaluate();
        raw * self.mat_scale() / 1024
    }

    pub fn features(&self) -> (Vec<usize>, Vec<usize>) {
        let mut s = vec![];
        let mut xs = vec![];

        let threats = self.threats(!self.stm);
        let defenders = self.threats(self.stm);
        for sq in self.occupancies() {
            let piece = self.piece_at(sq).name();
            let color = self.piece_at(sq).color();
            let perspective_color = usize::from(color != self.stm);
            let map_feature = |feat, threats: Bitboard, defenders: Bitboard| {
                2 * 768 * usize::from(defenders.contains(sq)) + 768 * usize::from(threats.contains(sq)) + feat
                //feat
            };
            //let stm_feat = [0, 384][perspective_color] + 64 * usize::from(piece) + usize::from(sq);
            //let xstm_feat = [384, 0][perspective_color] + 64 * usize::from(piece) + usize::from(sq.flip_vertical());

            let stm_feat = [0, 384][perspective_color]
                + 64 * usize::from(piece)
                + if self.stm == Color::White {
                    usize::from(sq)
                } else {
                    usize::from(sq.flip_vertical())
                };
            let xstm_feat = [384, 0][perspective_color]
                + 64 * usize::from(piece)
                + if self.stm == Color::Black {
                    usize::from(sq)
                } else {
                    usize::from(sq.flip_vertical())
                };
            //dbg!(piece as usize, color as usize, sq.0, stm_feat, xstm_feat);
            s.push(map_feature(stm_feat, threats, defenders));
            xs.push(map_feature(xstm_feat, defenders, threats));
        }
        s.sort();
        xs.sort();
        (s, xs)
    }
}

fn screlu(x: f32) -> f32 {
    x.clamp(0., 1.).powi(2)
}

//#[repr(C)]
//pub struct UnquantizedNetwork {
//    pub(super) ft: Layer<INPUT_SIZE, L1_SIZE, f32>,
//    pub(super) l1: PerspectiveLayer<L1_SIZE, 16, f32>,
//    pub(super) l2: Layer<16, 16, f32>,
//    pub(super) l3: Layer<16, 1, f32>,
//}
//
//impl UnquantizedNetwork {
//    pub fn quantize(&self) -> Box<Network> {
//        let mut ret = unsafe {
//            let mut uninit = std::mem::MaybeUninit::<Network>::uninit();
//            let ptr = uninit.as_mut_ptr() as *mut u8;
//            std::ptr::write_bytes(ptr, 0, std::mem::size_of::<Network>());
//            let my_struct = uninit.assume_init();
//            Box::new(my_struct)
//        };
//
//        for (q, &raw) in ret
//            .ft
//            .weights
//            .iter_mut()
//            .flatten()
//            .zip(self.ft.weights.iter().flatten())
//        {
//            //*q = f32_to_i16(raw, QA);
//            todo!();
//        }
//
//        for (q, &raw) in ret.ft.bias.iter_mut().zip(self.ft.bias.iter()) {
//            //*q = f32_to_i16(raw, QA);
//            todo!();
//        }
//
//        for (q, &raw) in ret
//            .l1
//            .weights
//            .iter_mut()
//            .flatten()
//            .flatten()
//            .zip(self.l1.weights.iter().flatten().flatten())
//        {
//            todo!();
//            //*q = f32_to_i16(raw, QB);
//        }
//
//        for (q, &raw) in ret.l1.bias.iter_mut().zip(self.l1.bias.iter()) {
//            todo!();
//            //*q = f32_to_i16(raw, QB);
//        }
//
//        ret.l2 = self.l2;
//        ret.l3 = self.l3;
//        ret
//    }
//}
//
//fn f32_to_i16(raw: f32, q: i16) -> i16 {
//    let x = (f32::from(q) * raw) as i16;
//
//    if (f64::from(raw) * f64::from(q)).trunc() != f64::from(x) {
//        panic!("Quantization failed for value: {raw}");
//    }
//
//    x
//}
