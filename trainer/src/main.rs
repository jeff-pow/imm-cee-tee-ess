use bullet::{
    format::ChessBoard,
    inputs::{Chess768, InputType},
};
use imm_cee_tee_ess::types::pieces::Color;
use std::str::FromStr;
use threat_inputs::ThreatInput;
use trainer::train;

mod advanced;
mod threat_inputs;
mod trainer;

fn main() {
    for fen in [
        //"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        //"r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        //"r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        //"rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 b - - 0 1",
    ] {
        let pos = ChessBoard::from_str(format!("{} | 0 | 0.0", fen).as_str()).unwrap();
        let board = imm_cee_tee_ess::board::Board::from_fen(fen);
        let t = ThreatInput.feature_iter(&pos);

        dbg!(t.threats);
        if board.stm == Color::Black {
            dbg!(board.threats(!board.stm).flip_vertical());
        } else {
            dbg!(board.threats(!board.stm));
        }
        println!();
        println!();

        dbg!(t.defenders);
        if board.stm == Color::Black {
            dbg!(board.threats(board.stm).flip_vertical());
        } else {
            dbg!(board.threats(board.stm));
        }
        println!();
        println!();

        dbg!(t.defenders.flip_vertical());
        if !board.stm == Color::Black {
            dbg!(board.threats(board.stm).flip_vertical());
        } else {
            dbg!(board.threats(board.stm));
        }
        println!();
        println!();

        dbg!(t.threats.flip_vertical());
        if !board.stm == Color::Black {
            dbg!(board.threats(!board.stm).flip_vertical());
        } else {
            dbg!(board.threats(!board.stm));
        }

        let mut stm = vec![];
        let mut xstm = vec![];
        for (s, x) in t.into_iter() {
            stm.push(s);
            xstm.push(x);
        }
        stm.sort();
        xstm.sort();
        let (board_stm, board_xstm) = board.calculate_features();
        assert_eq!(stm, board_stm);
        assert_eq!(xstm, board_xstm);
    }

    train();
}
