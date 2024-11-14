pub mod fen;

use core::fmt;

use crate::{
    attack_boards::{king_attacks, knight_attacks, pawn_attacks, pawn_set_attacks, BETWEEN_SQUARES},
    chess_move::{
        Castle,
        Direction::{North, South},
        Move, MoveType, CASTLING_RIGHTS,
    },
    magics::{bishop_attacks, rook_attacks},
    types::{
        bitboard::Bitboard,
        pieces::{Color, Piece, PieceName, NUM_PIECES},
        square::Square,
    },
    zobrist::ZOBRIST,
};
use fen::STARTING_FEN;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Board {
    bitboards: [Bitboard; NUM_PIECES],
    color_occupancies: [Bitboard; 2],
    mailbox: [Piece; 64],
    /// Side to move
    stm: Color,
    castling_rights: u8,
    en_passant_square: Square,
    half_moves: u8,
    zobrist_hash: u64,
    pawn_hash: u64,
}

impl Default for Board {
    fn default() -> Self {
        Self::from_fen(STARTING_FEN)
    }
}

impl Board {
    pub const fn hash(&self) -> u64 {
        self.zobrist_hash
    }

    pub const fn pawn_hash(&self) -> u64 {
        self.pawn_hash
    }

    pub fn half_moves(&self) -> usize {
        usize::from(self.half_moves)
    }

    pub fn castling_rights(&self) -> usize {
        usize::from(self.castling_rights)
    }

    pub const fn en_passant_square(&self) -> Square {
        self.en_passant_square
    }

    pub const fn stm(&self) -> Color {
        self.stm
    }

    pub const fn piece_bbs(&self) -> [Bitboard; 6] {
        self.bitboards
    }

    pub const fn color_bbs(&self) -> [Bitboard; 2] {
        self.color_occupancies
    }

    pub fn piece_color(&self, side: Color, piece: PieceName) -> Bitboard {
        self.piece(piece) & self.color(side)
    }

    pub fn piece(&self, piece: PieceName) -> Bitboard {
        self.bitboards[piece]
    }

    pub fn color(&self, color: Color) -> Bitboard {
        self.color_occupancies[color]
    }

    pub fn occupancies(&self) -> Bitboard {
        self.color(Color::White) | self.color(Color::Black)
    }

    pub fn piece_at(&self, sq: Square) -> Piece {
        self.mailbox[sq]
    }

    /// Returns the type of piece captured by a move, if any
    pub fn capture(&self, m: Move) -> Piece {
        if m.is_en_passant() {
            Piece::new(PieceName::Pawn, !self.stm)
        } else {
            self.piece_at(m.to())
        }
    }

    pub fn has_non_pawns(&self, side: Color) -> bool {
        !(self.occupancies() ^ self.piece_color(side, PieceName::King) ^ self.piece_color(side, PieceName::Pawn))
            .is_empty()
    }

    pub fn can_en_passant(&self) -> bool {
        self.en_passant_square != Square::NONE
    }

    pub const fn can_castle(&self, c: Castle) -> bool {
        match c {
            Castle::WhiteKing => self.castling_rights & Castle::WhiteKing as u8 != 0,
            Castle::WhiteQueen => self.castling_rights & Castle::WhiteQueen as u8 != 0,
            Castle::BlackKing => self.castling_rights & Castle::BlackKing as u8 != 0,
            Castle::BlackQueen => self.castling_rights & Castle::BlackQueen as u8 != 0,
            Castle::None => unreachable!(),
        }
    }

    pub fn place_piece(&mut self, piece: Piece, sq: Square) {
        self.mailbox[sq] = piece;
        self.bitboards[piece.name()] ^= sq.bitboard();
        self.color_occupancies[piece.color()] ^= sq.bitboard();
        self.zobrist_hash ^= ZOBRIST.piece[piece][sq];
        if piece.name() == PieceName::Pawn {
            self.pawn_hash ^= ZOBRIST.piece[piece][sq];
        }
    }

    fn remove_piece(&mut self, sq: Square) {
        let piece = self.piece_at(sq);
        self.mailbox[sq] = Piece::None;
        if piece != Piece::None {
            self.bitboards[piece.name()] ^= sq.bitboard();
            self.color_occupancies[piece.color()] ^= sq.bitboard();
            self.zobrist_hash ^= ZOBRIST.piece[piece][sq];
            if piece.name() == PieceName::Pawn {
                self.pawn_hash ^= ZOBRIST.piece[piece][sq];
            }
        }
    }

    pub fn king_square(&self, color: Color) -> Square {
        self.piece_color(color, PieceName::King).lsb()
    }

    pub fn attackers(&self, sq: Square, occupancy: Bitboard) -> Bitboard {
        self.attackers_for_side(Color::White, sq, occupancy) | self.attackers_for_side(Color::Black, sq, occupancy)
    }

    pub fn attackers_for_side(&self, attacker: Color, sq: Square, occupancy: Bitboard) -> Bitboard {
        let bishops = self.piece(PieceName::Queen) | self.piece(PieceName::Bishop);
        let rooks = self.piece(PieceName::Queen) | self.piece(PieceName::Rook);
        let pawn_attacks = pawn_attacks(sq, !attacker) & self.piece(PieceName::Pawn);
        let knight_attacks = knight_attacks(sq) & self.piece(PieceName::Knight);
        let bishop_attacks = bishop_attacks(sq, occupancy) & bishops;
        let rook_attacks = rook_attacks(sq, occupancy) & rooks;
        let king_attacks = king_attacks(sq) & self.piece(PieceName::King);
        (pawn_attacks | knight_attacks | bishop_attacks | rook_attacks | king_attacks) & self.color(attacker)
    }

    pub fn square_under_attack(&self, attacker: Color, sq: Square) -> bool {
        !self.attackers_for_side(attacker, sq, self.occupancies()).is_empty()
    }

    pub fn in_check(&self) -> bool {
        self.square_under_attack(!self.stm, self.king_square(self.stm))
    }

    pub(super) fn pinned_and_checkers(&self) -> (Bitboard, Bitboard) {
        let mut pinned = Bitboard::EMPTY;
        let attacker = !self.stm;
        let king_sq = self.king_square(self.stm);

        let mut checkers = knight_attacks(king_sq) & self.piece_color(attacker, PieceName::Knight)
            | pawn_attacks(king_sq, self.stm) & self.piece_color(attacker, PieceName::Pawn);

        let sliders_attacks = self.diags(attacker) & bishop_attacks(king_sq, Bitboard::EMPTY)
            | self.orthos(attacker) & rook_attacks(king_sq, Bitboard::EMPTY);
        for sq in sliders_attacks {
            let blockers = BETWEEN_SQUARES[sq][king_sq] & self.occupancies();
            if blockers.is_empty() {
                // No pieces between attacker and king
                checkers |= sq.bitboard();
            } else if blockers.count_bits() == 1 {
                // One piece between attacker and king
                pinned |= blockers & self.color(self.stm);
            }
            // Multiple pieces between attacker and king, we don't really care
        }
        (pinned, checkers)
    }

    pub(crate) fn diags(&self, side: Color) -> Bitboard {
        (self.piece(PieceName::Bishop) | self.piece(PieceName::Queen)) & self.color(side)
    }

    pub(crate) fn orthos(&self, side: Color) -> Bitboard {
        (self.piece(PieceName::Rook) | self.piece(PieceName::Queen)) & self.color(side)
    }

    pub fn threats(&self, attacker: Color) -> Bitboard {
        let mut threats = Bitboard::EMPTY;
        let occ = self.occupancies() ^ self.king_square(self.stm).bitboard();

        threats |= pawn_set_attacks(self.piece_color(attacker, PieceName::Pawn), attacker);

        self.orthos(attacker)
            .into_iter()
            .for_each(|sq| threats |= rook_attacks(sq, occ));

        self.diags(attacker)
            .into_iter()
            .for_each(|sq| threats |= bishop_attacks(sq, occ));

        self.piece_color(attacker, PieceName::Knight)
            .into_iter()
            .for_each(|sq| threats |= knight_attacks(sq));

        threats |= king_attacks(self.king_square(attacker));

        threats
    }

    /// Function makes a move and modifies board state to reflect the move that just happened.
    /// Assumes move is legal. Does *no* error checking whatsoever to ensure legality.
    pub fn make_move(&mut self, m: Move) {
        let piece_moving = m.piece_moving(self);
        assert_ne!(piece_moving, Piece::None, "{m:?}\n{self}");
        let capture = self.capture(m);
        self.remove_piece(m.to());

        if m.promotion().is_none() {
            self.place_piece(piece_moving, m.to());
        }

        self.remove_piece(m.from());

        // Move rooks if a castle move is applied
        if m.is_castle() {
            let rook = Piece::new(PieceName::Rook, self.stm);
            self.place_piece(rook, m.castle_type().rook_to());
            self.remove_piece(m.castle_type().rook_from());
        } else if let Some(p) = m.promotion() {
            self.place_piece(Piece::new(p, self.stm), m.to());
        } else if m.is_en_passant() {
            match self.stm {
                Color::White => {
                    self.remove_piece(m.to().shift(South));
                }
                Color::Black => {
                    self.remove_piece(m.to().shift(North));
                }
            }
        }

        // Xor out the old en passant square hash
        if self.can_en_passant() {
            self.zobrist_hash ^= ZOBRIST.en_passant[self.en_passant_square];
        }
        // If the end index of a move is 16 squares from the start (and a pawn moved), an en passant is possible
        self.en_passant_square = Square::NONE;
        if m.flag() == MoveType::DoublePush {
            match self.stm {
                Color::White => {
                    self.en_passant_square = m.to().shift(South);
                }
                Color::Black => {
                    self.en_passant_square = m.to().shift(North);
                }
            }
        }
        // Xor in the new en passant square hash
        if self.can_en_passant() {
            self.zobrist_hash ^= ZOBRIST.en_passant[self.en_passant_square];
        }

        // If a piece isn't captured and a pawn isn't moved, increment the half move clock.
        // Otherwise set it to zero

        if capture == Piece::None && piece_moving.name() != PieceName::Pawn {
            self.half_moves += 1;
        } else {
            self.half_moves = 0;
        }

        self.zobrist_hash ^= ZOBRIST.castling[self.castling_rights as usize];
        self.castling_rights &= CASTLING_RIGHTS[m.from()] & CASTLING_RIGHTS[m.to()];
        self.zobrist_hash ^= ZOBRIST.castling[self.castling_rights as usize];

        self.stm = !self.stm;
        self.zobrist_hash ^= ZOBRIST.turn;
    }

    /// Will have unexpected behavior if either of the arrays aren't completely disjoint sets with themselves
    pub fn from_bbs(piece_bbs: [Bitboard; 6], color_bbs: [Bitboard; 2], stm: Color) -> Self {
        let mut mailbox = [Piece::None; 64];
        for (piece_idx, &piece_bb) in piece_bbs.iter().enumerate() {
            for sq in piece_bb {
                for (color_idx, &color_bb) in color_bbs.iter().enumerate() {
                    if color_bb.contains(sq) {
                        mailbox[sq] = Piece::new(piece_idx.into(), color_idx.into());
                    }
                }
            }
        }
        Self {
            mailbox,
            bitboards: piece_bbs,
            color_occupancies: color_bbs,
            stm,
            ..Default::default()
        }
    }

    pub const fn empty() -> Self {
        Self {
            bitboards: [Bitboard::EMPTY; 6],
            color_occupancies: [Bitboard::EMPTY; 2],
            mailbox: [Piece::None; 64],
            castling_rights: 0,
            stm: Color::White,
            en_passant_square: Square::NONE,
            half_moves: 0,
            zobrist_hash: 0,
            pawn_hash: 0,
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = String::new();

        for row in (0..8).rev() {
            str.push_str(&(row + 1).to_string());
            str.push_str(" | ");

            for col in 0..8 {
                let idx = row * 8 + col;

                let piece = self.piece_at(Square(idx));
                str += &piece.char();

                str.push_str(" | ");
            }

            str.push('\n');
        }

        str.push_str("    a   b   c   d   e   f   g   h\n");

        str.push('\n');
        str.push_str(&self.to_fen());
        str.push('\n');

        write!(f, "{str}")
    }
}

#[cfg(test)]
mod board_tests {
    use super::*;
    #[test]
    fn test_place_piece() {
        let mut board = Board::empty();
        board.place_piece(Piece::WhiteRook, Square(0));
        assert!(board.piece_color(Color::White, PieceName::Rook).occupied(Square(0)));
    }

    #[test]
    fn test_remove_piece() {
        let board = Board::from_fen(STARTING_FEN);

        let mut c = board;
        c.remove_piece(Square(0));
        assert!(c.piece_color(Color::White, PieceName::Rook).empty(Square(0)));
        assert!(c.occupancies().empty(Square(0)));
        assert_ne!(c, board);

        let mut c = board;
        c.remove_piece(Square(27));
        assert_eq!(board, c);
    }
}
