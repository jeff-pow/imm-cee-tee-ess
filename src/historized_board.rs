use crate::{board::Board, chess_move::Move, movegen::MoveList, node::GameState, types::pieces::Color};

#[derive(Clone, Default, Debug)]
pub struct HistorizedBoard {
    board: Board,
    hashes: Vec<u64>,
}

impl HistorizedBoard {
    pub fn make_move(&mut self, m: Move) {
        self.board.make_move(m);
        self.hashes.push(self.board.zobrist_hash);
    }

    pub fn legal_moves(&self) -> MoveList {
        self.board.legal_moves()
    }

    pub fn game_state(&self) -> GameState {
        if self.board.half_moves >= 100 || self.is_3x_repetition() {
            return GameState::Draw;
        }

        if !self.legal_moves().is_empty() {
            return GameState::Ongoing;
        }

        if self.board.in_check() {
            GameState::Lost
        } else {
            GameState::Draw
        }
    }

    fn is_3x_repetition(&self) -> bool {
        if self.hashes.len() < 6 {
            return false;
        }

        let mut reps = 2;
        for &hash in self
            .hashes
            .iter()
            .rev()
            .take(self.board.half_moves as usize + 1)
            .skip(1)
            .step_by(2)
        {
            reps -= u32::from(hash == self.board.zobrist_hash);
            if reps == 0 {
                return true;
            }
        }
        false
    }

    pub const fn hash(&self) -> u64 {
        self.board.zobrist_hash
    }

    pub fn scaled_eval(&self) -> f32 {
        self.board.scaled_eval()
    }

    pub const fn stm(&self) -> Color {
        self.board.stm
    }

    pub const fn board(&self) -> Board {
        self.board
    }

    pub fn set_board(&mut self, board: Board) {
        self.board = board;
    }
}

impl From<&str> for HistorizedBoard {
    fn from(value: &str) -> Self {
        Self {
            board: Board::from_fen(value),
            hashes: vec![],
        }
    }
}
