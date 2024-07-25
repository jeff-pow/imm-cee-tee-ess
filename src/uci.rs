use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io, time::Duration};

use crate::bench::bench;
use crate::chess_move::Move;
use crate::fen::{parse_fen_from_buffer, STARTING_FEN};
use crate::game_time::Clock;
use crate::perft::perft;
use crate::{board::Board, types::pieces::Color};

pub const ENGINE_NAME: &str = "IM CEE TEE ESS";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main loop that handles UCI communication with GUIs
pub fn main_loop() -> ! {
    let mut board = Board::from_fen(STARTING_FEN);
    let mut msg = None;
    let mut hash_history = Vec::new();
    let halt = AtomicBool::new(false);
    println!("{ENGINE_NAME} v{VERSION} by {}", env!("CARGO_PKG_AUTHORS"));

    loop {
        let input = msg.as_ref().map_or_else(
            || {
                let mut buffer = String::new();
                let len_read = io::stdin().read_line(&mut buffer).unwrap();
                if len_read == 0 {
                    // Stdin closed, exit for openbench
                    exit(0);
                }
                buffer
            },
            Clone::clone,
        );

        msg = None;
        let input = input.split_whitespace().collect::<Vec<_>>();

        match *input.first().unwrap_or(&"Invalid command") {
            "isready" => println!("readyok"),
            "ucinewgame" => {
                halt.store(false, Ordering::Relaxed);
            }
            "eval" => {
                todo!()
            }
            "position" => position_command(&input, &mut board, &mut hash_history),
            "d" => {
                dbg!(&board);
            }
            "dbg" => {
                dbg!(&board);
                board.debug_bitboards();
            }
            "bench" => bench(),
            "go" => {
                todo!()
            }
            "perft" => {
                perft(&board, input[1].parse().unwrap());
            }
            "quit" => {
                exit(0);
            }
            "uci" => {
                uci_opts();
            }
            "setoption" => match input[..] {
                ["setoption", "name", "Hash", "value", _x] => {
                    todo!()
                }
                ["setoption", "name", "Clear", "Hash"] => todo!(),
                ["setoption", "name", "Threads", "value", _x] => {
                    todo!()
                }
                _ => println!("Option not recognized"),
            },
            _ => (),
        };
    }
}

fn uci_opts() {
    println!("id name {ENGINE_NAME} {VERSION}");
    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
    println!("option name Threads type spin default 1 min 1 max 64");
    println!("option name Hash type spin default 16 min 1 max 8388608");
    println!("uciok");
}

fn position_command(input: &[&str], board: &mut Board, hash_history: &mut Vec<u64>) {
    hash_history.clear();

    if input.contains(&"fen") {
        *board = Board::from_fen(&parse_fen_from_buffer(input));

        if let Some(skip) = input.iter().position(|f| f == &"moves") {
            parse_moves(&input[skip + 1..], board, hash_history);
        }
    } else if input.contains(&"startpos") {
        *board = Board::from_fen(STARTING_FEN);

        if let Some(skip) = input.iter().position(|f| f == &"moves") {
            parse_moves(&input[skip + 1..], board, hash_history);
        }
    }
}

fn parse_moves(moves: &[&str], board: &mut Board, hash_history: &mut Vec<u64>) {
    for str in moves.iter() {
        let m = Move::from_san(str, board);
        board.make_move(m);
        hash_history.push(board.zobrist_hash);
    }
}

pub fn parse_time(buff: &[&str]) -> Clock {
    let mut game_time = Clock::default();
    let mut iter = buff.iter().skip(1);
    while let Some(uci_opt) = iter.next() {
        match *uci_opt {
            "wtime" => {
                let raw_time = iter
                    .next()
                    .unwrap()
                    .parse::<i64>()
                    .expect("Valid i64")
                    .max(1);
                game_time.time_remaining[Color::White] = Duration::from_millis(raw_time as u64);
            }
            "btime" => {
                let raw_time = iter
                    .next()
                    .unwrap()
                    .parse::<i64>()
                    .expect("Valid i64")
                    .max(1);
                game_time.time_remaining[Color::Black] = Duration::from_millis(raw_time as u64);
            }
            "winc" => {
                let raw_time = iter
                    .next()
                    .unwrap()
                    .parse::<i64>()
                    .expect("Valid i64")
                    .max(1);
                game_time.time_inc[Color::White] = Duration::from_millis(raw_time as u64);
            }
            "binc" => {
                let raw_time = iter
                    .next()
                    .unwrap()
                    .parse::<i64>()
                    .expect("Valid i64")
                    .max(1);
                game_time.time_inc[Color::Black] = Duration::from_millis(raw_time as u64);
            }
            "movestogo" => {
                game_time.movestogo = iter.next().unwrap().parse::<i32>().expect("Valid i32")
            }
            _ => return game_time,
        }
    }
    game_time
}
