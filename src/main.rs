mod attack_boards;
mod bench;
mod board;
mod chess_move;
mod fen;
mod game_time;
mod magics;
mod movegen;
mod perft;
mod see;
mod types;
mod uci;
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
