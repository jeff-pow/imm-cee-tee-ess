//#![warn(clippy::all, clippy::pedantic/*, clippy::nursery*/)]
//#![allow(
//    clippy::cast_sign_loss,
//    clippy::module_name_repetitions,
//    clippy::cast_possible_truncation,
//    clippy::cast_precision_loss,
//    clippy::cast_possible_wrap,
//    clippy::large_stack_frames,
//)]

mod arena;
mod attack_boards;
mod bench;
pub mod board;
pub mod chess_move;
mod edge;
pub mod eval;
mod fen;
mod game_time;
mod hashtable;
mod historized_board;
mod magics;
pub mod movegen;
mod node;
mod perft;
mod search_type;
pub mod see;
pub mod types;
mod uci;
mod value;
mod zobrist;

pub use crate::bench::bench;
pub use std::env;
pub use uci::main_loop;
