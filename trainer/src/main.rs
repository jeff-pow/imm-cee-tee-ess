use bullet::{
    format::ChessBoard,
    inputs::{Chess768, InputType},
};
use std::str::FromStr;
use threat_inputs::ThreatInput;
use trainer::train;

mod advanced;
mod threat_inputs;
mod trainer;

fn main() {
    let board = ChessBoard::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
    let t = ThreatInput::feature_iter(&self, &board);
    let feats = t.into_iter().collect::<Vec<_>>();
    dbg!(feats);
    train();
}
