#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::large_stack_frames,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::missing_panics_doc
)]

mod arena;
mod attack_boards;
mod bench;
pub mod board;
pub mod chess_move;
mod edge;
pub mod eval;
mod game_time;
mod hashtable;
mod historized_board;
mod magics;
pub mod movegen;
mod node;
mod node_buffer;
mod perft;
pub mod policy;
mod search_type;
pub mod see;
pub mod types;
mod uci;
mod value;
mod zobrist;

pub use crate::bench::bench;
pub use uci::main_loop;
