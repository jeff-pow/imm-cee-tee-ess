use super::Board;
use crate::{
    chess_move::Castle,
    types::{
        pieces::{Color, Piece},
        square::{Square, SQUARE_NAMES},
    },
};

/// Fen string for the starting position of a board
pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Takes in a string in fen notation and returns a board state
impl Board {
    pub fn from_fen(fen_string: &str) -> Self {
        let mut board = Self::empty();
        let mut row = 7;
        let pieces = fen_string.split(['/', ' ']).collect::<Vec<_>>();
        // FEN strings have 13 entries (if each slash and each space delimit an entry)
        let mut iter = pieces.iter();
        let mut start = 7;
        let end = 0;
        let step: i32 = -1;
        while start >= end {
            // Loop handles reading board part of fen string
            let entry = iter.next().unwrap();
            let mut idx = 0;
            for c in entry.chars() {
                if c.is_ascii_digit() {
                    idx += c.to_digit(10).unwrap();
                    continue;
                }
                let square = row * 8 + idx;
                let square = Square(square as u8);
                let pieces = "PpNnBbRrQqKk";
                let Some(i) = pieces.chars().position(|x| x == c) else {
                    panic!("Unrecognized char {c}, board could not be made");
                };
                board.place_piece(i.into(), square);
                idx += 1;
            }
            start += step;
            row = row.saturating_sub(1);
        }
        // 9th element: find who's turn it is to move
        board.stm = match iter.next().unwrap().chars().next().unwrap() {
            'w' => Color::White,
            'b' => Color::Black,
            _ => panic!("Invalid turn"),
        };
        board.zobrist_hash = board.generate_hash();

        // 10th bucket find who can still castle
        // Order of array is white king castle, white queen castle, black king castle, black queen castle
        let Some(next) = iter.next() else {
            return board;
        };
        board.castling_rights = parse_castling(next);

        let Some(next) = iter.next() else {
            return board;
        };
        let en_passant_letters: Vec<char> = next.chars().collect();
        let en_passant_idx = find_en_passant_square(&en_passant_letters);
        if let Some(idx) = en_passant_idx {
            board.en_passant_square = Square(idx as u8);
        }
        board.zobrist_hash = board.generate_hash();

        let half_moves = iter.next();
        if let Some(half_moves) = half_moves {
            if let Ok(half_moves) = half_moves.parse() {
                board.half_moves = half_moves;
            }
        }

        // Full number of moves in the game: starts from 1 and incremented after black's first move
        let _full_moves = iter.next();

        assert_eq!(iter.next(), None);
        board
    }

    pub fn to_fen(self) -> String {
        let mut str = String::new();
        for r in (0..8).rev() {
            let mut gap = 0;
            for f in 0..8 {
                let sq = Square(r * 8 + f);
                let piece = self.piece_at(sq);

                if piece == Piece::None {
                    gap += 1;
                    continue;
                }

                if gap > 0 {
                    str += &gap.to_string();
                }
                str += &piece.char();
                gap = 0;
            }

            if gap > 0 {
                str += &gap.to_string();
            }

            if r != 0 {
                str += "/";
            }
        }

        str += " ";
        str += match self.stm {
            Color::White => "w",
            Color::Black => "b",
        };

        str += " ";
        if self.castling_rights == 0 {
            str += "-";
        } else {
            if self.can_castle(Castle::WhiteKing) {
                str += "K";
            }
            if self.can_castle(Castle::WhiteQueen) {
                str += "Q";
            }
            if self.can_castle(Castle::BlackKing) {
                str += "k";
            }
            if self.can_castle(Castle::BlackQueen) {
                str += "q";
            }
        }

        str += " ";
        if self.can_en_passant() {
            str += SQUARE_NAMES[self.en_passant_square];
        } else {
            str += "-";
        }

        str += " ";
        str += &self.half_moves.to_string();

        // We don't actually keep track of total number of moves so just throw a bogus number in
        // there
        str += " 1";

        str
    }
}

fn parse_castling(buf: &str) -> u8 {
    let rights = buf.chars().fold(0, |x, ch| {
        x | match ch {
            'K' => Castle::WhiteKing as u8,
            'Q' => Castle::WhiteQueen as u8,
            'k' => Castle::BlackKing as u8,
            'q' => Castle::BlackQueen as u8,
            _ => 0,
        }
    });
    rights
}

// Don't try to make this function const no matter what clippy says :')
fn find_en_passant_square(vec: &[char]) -> Option<u32> {
    if vec[0] == '-' {
        return None;
    }
    // Using base 20 allows program to convert letters directly to numbers instead of matching
    // against letters or some other workaround
    let column = vec[0].to_digit(20).unwrap() - 10;
    let row = (vec[1].to_digit(10).unwrap() - 1) * 8;
    Some(row + column)
}

pub fn parse_fen_from_buffer(buf: &[&str]) -> String {
    let mut vec = buf.to_owned();
    vec.remove(0);
    vec.remove(0);
    for _ in 6..vec.len() {
        vec.pop();
    }
    vec.join(" ")
}

#[cfg(test)]
mod fen_tests {
    use crate::{
        board::{
            fen::{find_en_passant_square, parse_castling},
            Board,
        },
        chess_move::Castle,
    };

    #[test]
    fn test_en_passant_square() {
        assert_eq!(Some(0), find_en_passant_square(&['a', '1']));
        assert_eq!(Some(9), find_en_passant_square(&['b', '2']));
        assert_eq!(Some(18), find_en_passant_square(&['c', '3']));
        assert_eq!(Some(27), find_en_passant_square(&['d', '4']));
        assert_eq!(Some(36), find_en_passant_square(&['e', '5']));
        assert_eq!(Some(45), find_en_passant_square(&['f', '6']));
        assert_eq!(Some(54), find_en_passant_square(&['g', '7']));
        assert_eq!(Some(63), find_en_passant_square(&['h', '8']));
        assert_eq!(Some(62), find_en_passant_square(&['g', '8']));
    }

    #[test]
    fn test_parse_castling_white_king() {
        let input = "K";
        let result = parse_castling(input);
        assert_eq!(result, Castle::WhiteKing as u8);
    }

    #[test]
    fn test_parse_castling_white_queen() {
        let input = "Q";
        let result = parse_castling(input);
        assert_eq!(result, Castle::WhiteQueen as u8);
    }

    #[test]
    fn test_parse_castling_black_king() {
        let input = "k";
        let result = parse_castling(input);
        assert_eq!(result, Castle::BlackKing as u8);
    }

    #[test]
    fn test_parse_castling_black_queen() {
        let input = "q";
        let result = parse_castling(input);
        assert_eq!(result, Castle::BlackQueen as u8);
    }

    #[test]
    fn test_parse_castling_invalid() {
        let input = "X";
        let result = parse_castling(input);
        assert_eq!(result, 0); // Expecting 0 for invalid input
    }

    #[test]
    fn test_parse_multiple_castlings() {
        let input = "KQkq";
        let result = parse_castling(input);
        // You need to define the expected result based on the combination of castling rights.
        // For example, if all castling rights are allowed (KQkq), you can set the expected result to a specific value.
        let expected_result =
            Castle::WhiteKing as u8 | Castle::WhiteQueen as u8 | Castle::BlackKing as u8 | Castle::BlackQueen as u8;
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_parse_partial_castlings() {
        let input = "Kk";
        let result = parse_castling(input);
        // Define the expected result for the combination of castling rights in the input.
        let expected_result = Castle::WhiteKing as u8 | Castle::BlackKing as u8;
        assert_eq!(result, expected_result);
    }

    #[test]
    fn fen() {
        // Suspend your disbelief for these castling availabilities...
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQ e3 0 1",
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w Kq c6 0 1",
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b - - 1 1",
        ] {
            assert_eq!(fen, Board::from_fen(fen).to_fen());
        }
    }
}
