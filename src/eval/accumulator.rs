use crate::{
    board::Board,
    chess_move::Move,
    eval::HIDDEN_SIZE,
    types::pieces::{Color, Piece},
};

use super::{
    network::{flatten, Network, NORMALIZATION_FACTOR, QAB, SCALE},
    Align64, Block, NET,
};
use arrayvec::ArrayVec;
use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C, align(64))]
pub struct Accumulator {
    pub vals: [Align64<Block>; 2],
    pub correct: [bool; 2],
    pub m: Option<Move>,
    pub capture: Piece,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self { vals: [NET.feature_bias; 2], correct: [true; 2], m: Move::NULL, capture: Piece::None }
    }
}

impl Index<Color> for Accumulator {
    type Output = Block;

    fn index(&self, index: Color) -> &Self::Output {
        &self.vals[index]
    }
}

impl IndexMut<Color> for Accumulator {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        &mut self.vals[index]
    }
}

impl Accumulator {
    pub fn raw_evaluate(&self, stm: Color) -> i32 {
        let (us, them) = (&self[stm], &self[!stm]);
        let weights = &NET.output_weights;
        let output = flatten(us, &weights[0]) + flatten(them, &weights[1]);
        (i32::from(NET.output_bias) + output / NORMALIZATION_FACTOR) * SCALE / QAB
    }

    /// Credit to viridithas for these values and concepts
    pub fn scaled_evaluate(&self, board: &Board) -> i32 {
        let raw = self.raw_evaluate(board.stm);
        let eval = raw * board.mat_scale() / 1024;
        eval * (200 - board.half_moves as i32) / 200
    }
}

// Credit to akimbo. This function streamlines the assembly generated and prevents unnecessary
// redundant loads and stores to the same simd vectors.
pub fn update(acc: &mut Align64<Block>, adds: &[u16], subs: &[u16]) {
    const REGISTERS: usize = 8;
    const ELEMENTS_PER_LOOP: usize = REGISTERS * 256 / 16;

    let mut regs = [0i16; ELEMENTS_PER_LOOP];

    for i in 0..HIDDEN_SIZE / ELEMENTS_PER_LOOP {
        let offset = ELEMENTS_PER_LOOP * i;

        for (reg, &j) in regs.iter_mut().zip(acc[offset..].iter()) {
            *reg = j;
        }

        for &add in adds {
            let weights = &NET.feature_weights[usize::from(add)];

            for (reg, &w) in regs.iter_mut().zip(weights[offset..].iter()) {
                *reg += w;
            }
        }

        for &sub in subs {
            let weights = &NET.feature_weights[usize::from(sub)];

            for (reg, &w) in regs.iter_mut().zip(weights[offset..].iter()) {
                *reg -= w;
            }
        }

        for (a, &r) in acc[offset..].iter_mut().zip(regs.iter()) {
            *a = r;
        }
    }
}

impl Board {
    pub fn new_accumulator(&self) -> Accumulator {
        let mut acc = Accumulator::default();
        for view in Color::iter() {
            acc.vals[view] = NET.feature_bias;
            let mut vec: ArrayVec<u16, 32> = ArrayVec::new();
            for sq in self.occupancies() {
                let p = self.piece_at(sq);
                let idx = Network::feature_idx(p, sq, self.king_square(view), view);
                vec.push(idx as u16);
            }
            update(&mut acc.vals[view], &vec, &[]);
        }
        acc
    }
}
