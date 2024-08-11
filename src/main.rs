// #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
// #![allow(
//     clippy::cast_sign_loss,
//     clippy::cast_possible_truncation,
//     clippy::cast_precision_loss,
//     clippy::cast_possible_wrap
// )]

mod arena;
mod attack_boards;
mod bench;
mod board;
mod chess_move;
mod edge;
mod eval;
mod fen;
mod game_time;
mod hashtable;
mod historized_board;
mod magics;
mod movegen;
mod node;
mod perft;
mod search;
mod see;
mod types;
mod uci;
mod value;
mod zobrist;

use crate::bench::bench;
use std::env;
use uci::main_loop;

fn main() {
    if env::args().any(|x| x == *"bench") {
        bench();
    } else {
        main_loop();
    }
}
