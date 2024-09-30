// #![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// #![allow(
//     clippy::cast_sign_loss,
//     clippy::module_name_repetitions,
//     clippy::cast_possible_truncation,
//     clippy::cast_precision_loss,
//     clippy::cast_possible_wrap,
//     clippy::large_stack_frames
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
mod search_type;
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
