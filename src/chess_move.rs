use core::fmt;
use std::{
    fmt::Display,
    num::{NonZero, NonZeroU16},
};

use crate::{
    board::Board,
    chess_move::Direction::{East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West},
    types::{
        bitboard::Bitboard,
        pieces::{Piece, PieceName},
        square::Square,
    },
};

use MoveType::{
    BishopCapturePromotion, BishopPromotion, Capture, DoublePush, EnPassant, KingCastle, KnightCapturePromotion,
    KnightPromotion, Normal, QueenCapturePromotion, QueenCastle, QueenPromotion, RookCapturePromotion, RookPromotion,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveType {
    Normal = 0,

    QueenPromotion = 1,
    RookPromotion = 2,
    BishopPromotion = 3,
    KnightPromotion = 4,

    DoublePush = 5,

    KingCastle = 6,
    QueenCastle = 7,

    EnPassant = 8,

    Capture = 9,

    QueenCapturePromotion = 10,
    RookCapturePromotion = 11,
    BishopCapturePromotion = 12,
    KnightCapturePromotion = 13,
}

const _: () = assert!(std::mem::size_of::<Move>() == std::mem::size_of::<Option<Move>>());
const _: () = assert!(2 == std::mem::size_of::<Move>(), "Move should be 2 bytes");

/// A move needs 16 bits to be stored
///
/// bit  0-5: origin square (from 0 to 63)
/// bit  6-11: destination square (from 0 to 63)
/// bit 12-15: special move flag
/// NOTE: en passant bit is set only when a pawn can be captured
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(pub NonZeroU16);

impl Move {
    pub const NULL: Option<Self> = None;
    pub const INVALID: Self = Self(unsafe { NonZeroU16::new_unchecked(65) });

    pub const fn new(origin: Square, destination: Square, move_type: MoveType) -> Self {
        let m = origin.0 as u16 | ((destination.0 as u16) << 6) | ((move_type as u16) << 12);
        unsafe { Self(NonZero::new_unchecked(m)) }
    }

    pub fn is_capture(self, board: &Board) -> bool {
        let c = matches!(
            self.flag(),
            Capture | QueenCapturePromotion | RookCapturePromotion | BishopCapturePromotion | KnightCapturePromotion
        );
        if c {
            assert_ne!(Piece::None, board.piece_at(self.to()));
        } else {
            assert_eq!(Piece::None, board.piece_at(self.to()));
        }
        c
    }

    pub fn is_castle(self) -> bool {
        self.flag() == KingCastle || self.flag() == QueenCastle
    }

    pub fn piece_moving(self, board: &Board) -> Piece {
        board.piece_at(self.from())
    }

    pub fn flag(self) -> MoveType {
        let f = (self.0.get() >> 12) as u8 & 0b1111;
        match f {
            0 => Normal,
            1 => QueenPromotion,
            2 => RookPromotion,
            3 => BishopPromotion,
            4 => KnightPromotion,
            5 => DoublePush,
            6 => KingCastle,
            7 => QueenCastle,
            8 => EnPassant,
            9 => Capture,
            10 => QueenCapturePromotion,
            11 => RookCapturePromotion,
            12 => BishopCapturePromotion,
            13 => KnightCapturePromotion,
            _ => unreachable!(),
        }
    }

    pub fn is_en_passant(self) -> bool {
        self.flag() == EnPassant
    }

    pub fn promotion(self) -> Option<PieceName> {
        match self.flag() {
            QueenPromotion | QueenCapturePromotion => Some(PieceName::Queen),
            RookPromotion | RookCapturePromotion => Some(PieceName::Rook),
            BishopPromotion | BishopCapturePromotion => Some(PieceName::Bishop),
            KnightPromotion | KnightCapturePromotion => Some(PieceName::Knight),
            _ => None,
        }
    }

    pub const fn from(self) -> Square {
        Square((self.0.get() & 0b11_1111) as u8)
    }

    pub const fn to(self) -> Square {
        Square((self.0.get() >> 6 & 0b11_1111) as u8)
    }

    pub fn is_tactical(self, board: &Board) -> bool {
        self.promotion().is_some() || self.is_en_passant() || board.occupancies().occupied(self.to())
    }

    /// To Short Algebraic Notation
    #[expect(huh_theres_no_used_lint)]
    pub fn to_san_refact(self) -> String {
        format!(
            "{}{}{}",
            self.from(),
            self.to(),
            match self.promotion() {
                Some(PieceName::Queen) => "q",
                Some(PieceName::Rook) => "r",
                Some(PieceName::Bishop) => "b",
                Some(PieceName::Knight) => "n",
                Some(_) => unreachable!(),
                None => "",
            }
        )
    }

    /// To Short Algebraic Notation
    pub fn to_san(self) -> String {
        let mut str = String::new();
        let arr = ["a", "b", "c", "d", "e", "f", "g", "h"];
        let origin_number = self.from().rank() + 1;
        let origin_letter = self.from().file();
        let end_number = self.to().rank() + 1;
        let end_letter = self.to().file();
        str += arr[origin_letter as usize];
        str += &origin_number.to_string();
        str += arr[end_letter as usize];
        str += &end_number.to_string();
        if let Some(p) = self.promotion() {
            match p {
                PieceName::Queen => str += "q",
                PieceName::Rook => str += "r",
                PieceName::Bishop => str += "b",
                PieceName::Knight => str += "n",
                _ => (),
            }
        }
        str
    }

    pub fn castle_type(self) -> Castle {
        debug_assert!(self.is_castle());
        if self.to().dist(self.from()) != 2 {
            Castle::None
        } else if self.to() == Square::C1 {
            Castle::WhiteQueen
        } else if self.to() == Square::G1 {
            Castle::WhiteKing
        } else if self.to() == Square::C8 {
            Castle::BlackQueen
        } else if self.to() == Square::G8 {
            Castle::BlackKing
        } else {
            unreachable!()
        }
    }

    /// Method converts a san move provided by UCI framework into a Move struct
    pub fn from_san(str: &str, board: &Board) -> Self {
        let vec: Vec<char> = str.chars().collect();

        // Using base 20 allows program to convert letters directly to numbers instead of matching
        // against letters or some other workaround
        let start_column = vec[0].to_digit(20).unwrap() - 10;
        let start_row = (vec[1].to_digit(10).unwrap() - 1) * 8;
        let origin_sq = Square((start_row + start_column) as u8);

        let end_column = vec[2].to_digit(20).unwrap() - 10;
        let end_row = (vec[3].to_digit(10).unwrap() - 1) * 8;
        let dest_sq = Square((end_row + end_column) as u8);

        let promotion = match vec.get(4) {
            Some('q') => Some(PieceName::Queen),
            Some('r') => Some(PieceName::Rook),
            Some('b') => Some(PieceName::Bishop),
            Some('n') => Some(PieceName::Knight),
            None => None,
            x => panic!("Invalid letter in promotion spot of move: {x:?}"),
        };

        let piece_moving = board.piece_at(origin_sq);
        assert!(piece_moving != Piece::None);
        let captured = board.piece_at(dest_sq);
        let is_capture = captured != Piece::None;
        let castle = match piece_moving.name() {
            PieceName::King => {
                if origin_sq.dist(dest_sq) != 2 {
                    None
                } else if dest_sq == Square::C1 {
                    Some(QueenCastle)
                } else if dest_sq == Square::G1 {
                    Some(KingCastle)
                } else if dest_sq == Square::C8 {
                    Some(QueenCastle)
                } else if dest_sq == Square::G8 {
                    Some(KingCastle)
                } else {
                    unreachable!()
                }
            }
            _ => None,
        };
        let en_passant = { piece_moving.name() == PieceName::Pawn && !is_capture && start_column != end_column };
        let double_push = { piece_moving.name() == PieceName::Pawn && origin_sq.dist(dest_sq) == 2 };
        let move_type = {
            if en_passant {
                EnPassant
            } else if let Some(c) = castle {
                c
            } else if let Some(promotion) = promotion {
                match promotion {
                    PieceName::Knight => {
                        if is_capture {
                            KnightCapturePromotion
                        } else {
                            KnightPromotion
                        }
                    }
                    PieceName::Bishop => {
                        if is_capture {
                            BishopCapturePromotion
                        } else {
                            BishopPromotion
                        }
                    }
                    PieceName::Rook => {
                        if is_capture {
                            RookCapturePromotion
                        } else {
                            RookPromotion
                        }
                    }
                    PieceName::Queen => {
                        if is_capture {
                            QueenCapturePromotion
                        } else {
                            QueenPromotion
                        }
                    }
                    _ => unreachable!(),
                }
            } else if double_push {
                DoublePush
            } else if is_capture {
                Capture
            } else {
                Normal
            }
        };
        Self::new(origin_sq, dest_sq, move_type)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = String::new();
        str += &self.to_san();
        write!(f, "{str}")
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut str = String::new();
        str += &self.to_san();
        write!(f, "{str}")
    }
}

impl From<u16> for Move {
    fn from(value: u16) -> Self {
        Self(NonZeroU16::new(value).unwrap())
    }
}

impl From<Move> for u16 {
    fn from(value: Move) -> Self {
        value.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Castle {
    WhiteKing = 1,
    WhiteQueen = 2,
    BlackKing = 4,
    BlackQueen = 8,
    None,
}

impl Castle {
    /// These squares may not be under attack for a castle to be valid
    pub(crate) const fn check_squares(self) -> Bitboard {
        match self {
            Self::WhiteKing => Bitboard(112),
            Self::WhiteQueen => Bitboard(28),
            Self::BlackKing => Bitboard(0x7000_0000_0000_0000),
            Self::BlackQueen => Bitboard(0x1C00_0000_0000_0000),
            Self::None => panic!("Invalid castle"),
        }
    }

    /// These squares must be unoccupied for a castle to be valid
    pub(crate) const fn empty_squares(self) -> Bitboard {
        match self {
            Self::WhiteKing => Bitboard(96),
            Self::WhiteQueen => Bitboard(14),
            Self::BlackKing => Bitboard(0x6000_0000_0000_0000),
            Self::BlackQueen => Bitboard(0xE00_0000_0000_0000),
            Self::None => panic!("Invalid castle"),
        }
    }

    pub(crate) const fn rook_to(self) -> Square {
        match self {
            Self::WhiteKing => Square::F1,
            Self::WhiteQueen => Square::D1,
            Self::BlackKing => Square::F8,
            Self::BlackQueen => Square::D8,
            Self::None => panic!("Invalid castle"),
        }
    }

    pub(crate) const fn rook_from(self) -> Square {
        match self {
            Self::WhiteKing => Square::H1,
            Self::WhiteQueen => Square::A1,
            Self::BlackKing => Square::H8,
            Self::BlackQueen => Square::A8,
            Self::None => panic!("Invalid castle"),
        }
    }
}

#[rustfmt::skip]
pub const CASTLING_RIGHTS: [u8; 64] = [
    13, 15, 15, 15, 12, 15, 15, 14,
    15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15,
    7,  15, 15, 15,  3, 15, 15, 11,
];

/// Cardinal directions from the point of view of white side
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    North = 8,
    NorthWest = 7,
    West = -1,
    SouthWest = -9,
    South = -8,
    SouthEast = -7,
    East = 1,
    NorthEast = 9,
}

impl Direction {
    /// Returns the opposite direction of the given direction
    pub(crate) const fn opp(self) -> Self {
        match self {
            North => South,
            NorthWest => SouthEast,
            West => East,
            SouthWest => NorthEast,
            South => North,
            SouthEast => NorthWest,
            East => West,
            NorthEast => SouthWest,
        }
    }
}

#[cfg(test)]
mod move_test {
    use super::*;

    #[test]
    fn test_move_creation() {
        let normal_move = Move::new(Square(10), Square(20), Normal);
        assert_eq!(normal_move.from(), Square(10));
        assert_eq!(normal_move.to(), Square(20));
        assert!(!normal_move.is_castle());
        assert!(!normal_move.is_en_passant());
        assert_eq!(normal_move.promotion(), None);

        let promotion_move = Move::new(Square(15), Square(25), QueenPromotion);
        assert_eq!(promotion_move.from(), Square(15));
        assert_eq!(promotion_move.to(), Square(25));
        assert!(!promotion_move.is_castle());
        assert!(!promotion_move.is_en_passant());
        assert_eq!(promotion_move.promotion(), Some(PieceName::Queen));

        let castle_move = Move::new(Square(4), Square(2), QueenCastle);
        assert_eq!(castle_move.from(), Square(4));
        assert_eq!(castle_move.to(), Square(2));
        assert!(castle_move.is_castle());
        assert!(!castle_move.is_en_passant());
        assert_eq!(castle_move.promotion(), None);

        let en_passant_move = Move::new(Square(7), Square(5), EnPassant);
        assert_eq!(en_passant_move.from(), Square(7));
        assert_eq!(en_passant_move.to(), Square(5));
        assert!(!en_passant_move.is_castle());
        assert!(en_passant_move.is_en_passant());
        assert_eq!(en_passant_move.promotion(), None);
    }

    #[test]
    fn test_promotion_conversion() {
        let knight_promotion = Move::new(Square(0), Square(7), KnightPromotion);
        assert_eq!(knight_promotion.promotion(), Some(PieceName::Knight));

        let bishop_promotion = Move::new(Square(15), Square(23), BishopPromotion);
        assert_eq!(bishop_promotion.promotion(), Some(PieceName::Bishop));

        let rook_promotion = Move::new(Square(28), Square(31), RookPromotion);
        assert_eq!(rook_promotion.promotion(), Some(PieceName::Rook));

        let queen_promotion = Move::new(Square(62), Square(61), QueenPromotion);
        assert_eq!(queen_promotion.promotion(), Some(PieceName::Queen));
    }
}
