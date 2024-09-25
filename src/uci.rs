use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io, time::Duration};

use crate::arena::Arena;
use crate::bench::bench;
use crate::chess_move::Move;
use crate::fen::{parse_fen_from_buffer, STARTING_FEN};
use crate::game_time::Clock;
use crate::historized_board::HistorizedBoard;
use crate::perft::perft;
use crate::search_type::SearchType;
use crate::{board::Board, types::pieces::Color};
use std::thread;

pub const ENGINE_NAME: &str = "IM CEE TEE ESS";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main loop that handles UCI communication with GUIs
pub fn main_loop() -> ! {
    let mut msg = None;
    let mut board = HistorizedBoard::default();
    let halt = AtomicBool::new(false);
    let mut arena = Arena::default();
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
            "position" => board = position_command(&input),
            "d" => {
                dbg!(&board.board());
            }
            "dbg" => {
                dbg!(&board.board());
                board.board().debug_bitboards();
            }
            "bench" => bench(),
            "go" => handle_go(&mut arena, &input, &board, &mut msg, &halt),
            "perft" => {
                perft(&board.board(), input[1].parse().unwrap());
            }
            "quit" => {
                exit(0);
            }
            "uci" => {
                uci_opts();
            }
            "setoption" => match input[..] {
                ["setoption", "name", "Hash", "value", _x] => (),
                ["setoption", "name", "Clear", "Hash"] => (),
                ["setoption", "name", "Threads", "value", _x] => (),
                _ => println!("Option not recognized"),
            },
            _ => (),
        };
    }
}

fn uci_opts() {
    println!("id name {ENGINE_NAME} {VERSION}");
    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
    println!("option name Threads type spin default 1 min 1 max 1");
    println!("option name Hash type spin default 32 min 32 max 32");
    println!("uciok");
}

fn position_command(input: &[&str]) -> HistorizedBoard {
    let mut board = HistorizedBoard::default();

    if input.contains(&"fen") {
        board.set_board(Board::from_fen(&parse_fen_from_buffer(input)));

        if let Some(skip) = input.iter().position(|f| f == &"moves") {
            parse_moves(&input[skip + 1..], &mut board);
        }
    } else if input.contains(&"startpos") {
        board.set_board(Board::from_fen(STARTING_FEN));

        if let Some(skip) = input.iter().position(|f| f == &"moves") {
            parse_moves(&input[skip + 1..], &mut board);
        }
    }

    board
}

fn parse_moves(moves: &[&str], board: &mut HistorizedBoard) {
    for str in moves {
        let m = Move::from_san(str, &board.board());
        board.make_move(m);
    }
}

pub fn parse_time(buff: &[&str]) -> Clock {
    let mut game_time = Clock::default();
    let mut iter = buff.iter().skip(1);
    while let Some(uci_opt) = iter.next() {
        match *uci_opt {
            "wtime" => {
                let raw_time = iter.next().unwrap().parse::<i64>().expect("Valid i64").max(1);
                game_time.time_remaining[Color::White] = Duration::from_millis(raw_time as u64);
            }
            "btime" => {
                let raw_time = iter.next().unwrap().parse::<i64>().expect("Valid i64").max(1);
                game_time.time_remaining[Color::Black] = Duration::from_millis(raw_time as u64);
            }
            "winc" => {
                let raw_time = iter.next().unwrap().parse::<i64>().expect("Valid i64").max(1);
                game_time.time_inc[Color::White] = Duration::from_millis(raw_time as u64);
            }
            "binc" => {
                let raw_time = iter.next().unwrap().parse::<i64>().expect("Valid i64").max(1);
                game_time.time_inc[Color::Black] = Duration::from_millis(raw_time as u64);
            }
            "movestogo" => {
                game_time.movestogo = iter.next().unwrap().parse::<i32>().expect("Valid i32");
            }
            _ => return game_time,
        }
    }
    game_time
}

pub fn handle_go(
    arena: &mut Arena,
    buffer: &[&str],
    board: &HistorizedBoard,
    msg: &mut Option<String>,
    halt: &AtomicBool,
) {
    let search_type = if buffer.contains(&"depth") {
        let mut iter = buffer.iter().skip(2);
        let depth = iter.next().unwrap().parse::<i32>().unwrap();
        SearchType::Depth(depth as u64, u64::MAX)
    } else if buffer.contains(&"nodes") {
        let mut iter = buffer.iter().skip(2);
        let nodes = iter.next().unwrap().parse::<u64>().unwrap();
        SearchType::Nodes(nodes)
    } else if buffer.contains(&"wtime") {
        let mut clock = parse_time(buffer);
        clock.recommended_time(board.stm());
        SearchType::Time(clock)
    } else if buffer.contains(&"mate") {
        let mut iter = buffer.iter().skip(2);
        let ply = iter.next().unwrap().parse::<i32>().unwrap();
        SearchType::Mate(ply)
    } else {
        SearchType::Infinite
    };

    arena.reset();
    thread::scope(|s| {
        s.spawn(|| {
            let m = arena.start_search(board, halt, search_type, true);
            println!("bestmove {m}");
        });

        let mut s = String::new();
        let len_read = io::stdin().read_line(&mut s).unwrap();
        if len_read == 0 {
            // Stdin closed, exit for openbench
            exit(0);
        }
        match s.as_str().trim() {
            "isready" => println!("readyok"),
            "quit" => exit(0),
            "stop" => halt.store(true, Ordering::Relaxed),
            _ => {
                *msg = Some(s);
            }
        }
    });
}
