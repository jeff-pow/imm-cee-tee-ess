use crate::{
    attack_boards::{valid_pinned_moves, BETWEEN_SQUARES},
    board::Board,
    chess_move::Direction::{self, North, NorthEast, NorthWest, South, SouthEast, SouthWest},
    types::{
        bitboard::Bitboard,
        pieces::{Color, PieceName},
        square::Square,
    },
};

use super::{
    attack_boards::{king_attacks, knight_attacks, RANKS},
    chess_move::{Castle, Move, MoveType},
    magics::{bishop_attacks, rook_attacks},
};
use arrayvec::ArrayVec;

pub type MoveList = ArrayVec<Move, 256>;

impl Board {
    /// Generates all legal moves
    pub fn legal_moves(&self) -> MoveList {
        let mut moves = MoveList::default();

        let mut dests = !self.color(self.stm);

        let kings = self.piece_color(self.stm, PieceName::King);
        let knights = self.piece_color(self.stm, PieceName::Knight);
        let diags = self.diags(self.stm);
        let orthos = self.orthos(self.stm);

        let (pinned, checkers) = self.pinned_and_checkers();
        let threats = self.threats();

        self.jumper_moves(kings, dests & !threats, &mut moves, pinned, king_attacks);

        if checkers.count_bits() > 1 {
            return moves;
        } else if checkers.count_bits() == 0 {
            self.castling_moves(threats, &mut moves);
        }

        if !checkers.is_empty() {
            dests &= BETWEEN_SQUARES[checkers.lsb()][self.king_square(self.stm)] | checkers;
        }

        self.jumper_moves(knights, dests, &mut moves, pinned, knight_attacks);
        self.magic_moves(orthos, dests, &mut moves, pinned, rook_attacks);
        self.magic_moves(diags, dests, &mut moves, pinned, bishop_attacks);
        self.pawn_moves(pinned, dests, &mut moves);

        moves
    }

    fn castling_moves(&self, threats: Bitboard, moves: &mut MoveList) {
        if self.stm == Color::White {
            if self.can_castle(Castle::WhiteKing)
                && !threats.intersects(Castle::WhiteKing.check_squares())
                && !self.occupancies().intersects(Castle::WhiteKing.empty_squares())
            {
                moves.push(Move::new(Square::E1, Square::G1, MoveType::KingCastle));
            }
            if self.can_castle(Castle::WhiteQueen)
                && !threats.intersects(Castle::WhiteQueen.check_squares())
                && !self.occupancies().intersects(Castle::WhiteQueen.empty_squares())
            {
                moves.push(Move::new(Square::E1, Square::C1, MoveType::QueenCastle));
            }
        } else {
            if self.can_castle(Castle::BlackKing)
                && !threats.intersects(Castle::BlackKing.check_squares())
                && !self.occupancies().intersects(Castle::BlackKing.empty_squares())
            {
                moves.push(Move::new(Square::E8, Square::G8, MoveType::KingCastle));
            }
            if self.can_castle(Castle::BlackQueen)
                && !threats.intersects(Castle::BlackQueen.check_squares())
                && !self.occupancies().intersects(Castle::BlackQueen.empty_squares())
            {
                moves.push(Move::new(Square::E8, Square::C8, MoveType::QueenCastle));
            }
        }
    }

    fn pawn_moves(&self, pinned: Bitboard, dests: Bitboard, moves: &mut MoveList) {
        let pawns = self.piece_color(self.stm, PieceName::Pawn);
        let vacancies = !self.occupancies();
        let enemies = self.color(!self.stm);

        let non_promotions = pawns & if self.stm == Color::White { !RANKS[6] } else { !RANKS[1] };
        let promotions = pawns & if self.stm == Color::White { RANKS[6] } else { RANKS[1] };

        let up = if self.stm == Color::White { North } else { South };
        let right = if self.stm == Color::White { NorthEast } else { SouthWest };
        let left = if self.stm == Color::White { NorthWest } else { SouthEast };

        let rank3 = if self.stm == Color::White { RANKS[2] } else { RANKS[5] };

        // Single and double pawn pushes w/o captures
        let push_one = vacancies & non_promotions.shift(up);
        let push_two = vacancies & (push_one & rank3).shift(up);
        for dest in push_one & dests {
            let src = dest.shift(up.opp());
            if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                moves.push(Move::new(src, dest, MoveType::Normal));
            }
        }
        for dest in push_two & dests {
            let src = dest.shift(up.opp()).shift(up.opp());
            if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                moves.push(Move::new(src, dest, MoveType::DoublePush));
            }
        }

        // Promotions - captures and straight pushes
        let no_capture_promotions = promotions.shift(up) & vacancies;
        let left_capture_promotions = promotions.shift(left) & enemies;
        let right_capture_promotions = promotions.shift(right) & enemies;
        for dest in no_capture_promotions & dests {
            let src = dest.shift(up.opp());
            if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                gen_promotions::<false>(src, dest, moves);
            }
        }
        for dest in left_capture_promotions & dests {
            let src = dest.shift(left.opp());
            if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                gen_promotions::<true>(src, dest, moves);
            }
        }
        for dest in right_capture_promotions & dests {
            let src = dest.shift(right.opp());
            if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                gen_promotions::<true>(src, dest, moves);
            }
        }

        // Captures that do not lead to promotions
        if !non_promotions.is_empty() {
            let left_captures = non_promotions.shift(left) & enemies;
            let right_captures = non_promotions.shift(right) & enemies;
            for dest in left_captures & dests {
                let src = dest.shift(left.opp());
                if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                    moves.push(Move::new(src, dest, MoveType::Capture));
                }
            }
            for dest in right_captures & dests {
                let src = dest.shift(right.opp());
                if !pinned.contains(src) || valid_pinned_moves(self.king_square(self.stm), src).contains(dest) {
                    moves.push(Move::new(src, dest, MoveType::Capture));
                }
            }
        }

        // En Passant
        if self.can_en_passant() {
            if let Some(x) = self.get_en_passant(left.opp()) {
                moves.push(x);
            }
            if let Some(x) = self.get_en_passant(right.opp()) {
                moves.push(x);
            }
        }
    }

    fn get_en_passant(&self, dir: Direction) -> Option<Move> {
        let sq = self.en_passant_square.checked_shift(dir)?;
        let pawn = sq.bitboard() & self.piece_color(self.stm, PieceName::Pawn);
        if pawn.is_empty() {
            return None;
        }
        let dest = self.en_passant_square;
        let src = dest.checked_shift(dir)?;
        let m = Move::new(src, dest, MoveType::EnPassant);
        let mut new_b = *self;
        new_b.make_move(m);
        if !new_b.square_under_attack(!self.stm, self.king_square(self.stm)) {
            return Some(m);
        }
        None
    }

    fn magic_moves<F: Fn(Square, Bitboard) -> Bitboard>(
        &self,
        pieces: Bitboard,
        destinations: Bitboard,
        moves: &mut MoveList,
        pinned: Bitboard,
        attack_fn: F,
    ) {
        for src in pieces {
            let dests = if pinned.contains(src) {
                destinations & valid_pinned_moves(self.king_square(self.stm), src)
            } else {
                destinations
            };
            for dest in attack_fn(src, self.occupancies()) & dests {
                moves.push(Move::new(src, dest, MoveType::Capture));
            }
        }
    }

    fn jumper_moves<F: Fn(Square) -> Bitboard>(
        &self,
        pieces: Bitboard,
        destinations: Bitboard,
        moves: &mut MoveList,
        pinned: Bitboard,
        attack_fn: F,
    ) {
        for src in pieces {
            let dests = if pinned.contains(src) {
                destinations & valid_pinned_moves(self.king_square(self.stm), src)
            } else {
                destinations
            };
            for dest in attack_fn(src) & dests {
                moves.push(Move::new(src, dest, MoveType::Normal));
            }
        }
    }
}

fn gen_promotions<const IS_CAP: bool>(src: Square, dest: Square, moves: &mut MoveList) {
    let promos = if IS_CAP {
        [
            MoveType::QueenCapturePromotion,
            MoveType::RookCapturePromotion,
            MoveType::BishopCapturePromotion,
            MoveType::KnightCapturePromotion,
        ]
    } else {
        [
            MoveType::QueenPromotion,
            MoveType::RookPromotion,
            MoveType::BishopPromotion,
            MoveType::KnightPromotion,
        ]
    };
    for promo in promos {
        moves.push(Move::new(src, dest, promo));
    }
}
