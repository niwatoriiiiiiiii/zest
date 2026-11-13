use std::fmt::{self, Write as _};

use crate::board::Board;
use crate::core::{Color, Piece, Square};

/// Buffer capacity covering a 5x5 placement and typical clock fields (`<placement> <side> - - <hm> <fm>`).
const GARDNER_FEN_CAPACITY: usize = 48;

/// Errors that can occur when parsing a Gardner 5x5 FEN string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenError {
    /// The FEN string has missing or extra fields.
    InvalidFormat,

    /// The piece placement field does not contain exactly 5 ranks.
    InvalidRankCount,

    /// A rank in the piece placement field does not span exactly 5 files.
    InvalidFileCount,

    /// An unrecognized character appeared in the piece placement field.
    InvalidPiece(char),

    /// The active color field is neither `'w'` nor `'b'`.
    InvalidSideToMove,

    /// The halfmove clock field is not a valid non-negative integer.
    InvalidHalfmoveClock,

    /// The fullmove number field is not a valid positive integer.
    InvalidFullmoveNumber,
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid FEN field structure"),
            Self::InvalidRankCount => write!(f, "FEN placement must have exactly 5 ranks"),
            Self::InvalidFileCount => write!(f, "each FEN rank must span exactly 5 files"),
            Self::InvalidPiece(ch) => write!(f, "invalid piece character in FEN: '{ch}'"),
            Self::InvalidSideToMove => write!(f, "side to move must be 'w' or 'b'"),
            Self::InvalidHalfmoveClock => {
                write!(f, "halfmove clock must be a non-negative integer")
            }
            Self::InvalidFullmoveNumber => write!(f, "fullmove number must be a positive integer"),
        }
    }
}

impl std::error::Error for FenError {}

impl Board {
    /// Serializes the board state into a Gardner 5x5 FEN string.
    ///
    /// Castling rights, en passant square, halfmove clock, and fullmove number
    /// are emitted as the fixed suffix `"- - 0 1"`.
    #[must_use]
    pub fn to_fen(&self) -> String {
        self.to_fen_with_clocks(0, 1)
    }

    /// Serializes the board state into a Gardner 5x5 FEN string with the given move clocks.
    #[must_use]
    pub fn to_fen_with_clocks(&self, halfmove_clock: u16, fullmove_number: u16) -> String {
        let mut fen_str = String::with_capacity(GARDNER_FEN_CAPACITY);

        for rank in (0..5_u8).rev() {
            let mut empty_count = 0_u8;

            for file in 0..5_u8 {
                if let Some(sq) = Square::new(file, rank) {
                    if let Some(piece) = self.piece_at(sq) {
                        if empty_count > 0 {
                            fen_str.push(char::from(b'0' + empty_count));
                            empty_count = 0;
                        }
                        fen_str.push(piece.to_char());
                    } else {
                        empty_count += 1;
                    }
                }
            }

            if empty_count > 0 {
                fen_str.push(char::from(b'0' + empty_count));
            }
            if rank > 0 {
                fen_str.push('/');
            }
        }

        let side_char = match self.side_to_move() {
            Color::White => 'w',
            Color::Black => 'b',
        };
        let _ = write!(
            fen_str,
            " {side_char} - - {halfmove_clock} {fullmove_number}"
        );

        fen_str
    }

    /// Parses a Gardner 5x5 FEN string into a [`Board`].
    ///
    /// Accepts either the 2-field form (`"<placement> <side>"`) or the standard
    /// 6-field form (`"<placement> <side> <castling> <ep> <halfmove> <fullmove>"`).
    ///
    /// # Errors
    ///
    /// Returns a [`FenError`] if the field count is invalid, the board placement does not
    /// have 5 ranks of 5 files each, an unknown piece character is encountered, the
    /// side to move is neither `'w'` nor `'b'`, or the move clocks in a 6-field FEN are invalid.
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        Self::from_fen_with_clocks(fen).map(|(board, _, _)| board)
    }

    /// Parses a Gardner 5x5 FEN string into `(Board, halfmove_clock, fullmove_number)`.
    ///
    /// For the 2-field short form, the clocks default to `(0, 1)`.
    ///
    /// # Errors
    ///
    /// Returns a [`FenError`] if the FEN format, piece placement, side to move, or move clocks
    /// are invalid.
    pub fn from_fen_with_clocks(fen: &str) -> Result<(Self, u16, u16), FenError> {
        let mut fields = fen.split_ascii_whitespace();
        let placement = fields.next().ok_or(FenError::InvalidFormat)?;
        let side_str = fields.next().ok_or(FenError::InvalidFormat)?;

        let (halfmove_clock, fullmove_number) = match fields.next() {
            None => (0, 1),
            Some(_castling) => {
                let _ep = fields.next().ok_or(FenError::InvalidFormat)?;
                let halfmove_str = fields.next().ok_or(FenError::InvalidFormat)?;
                let fullmove_str = fields.next().ok_or(FenError::InvalidFormat)?;
                if fields.next().is_some() {
                    return Err(FenError::InvalidFormat);
                }

                let halfmove = halfmove_str
                    .parse::<u16>()
                    .map_err(|_| FenError::InvalidHalfmoveClock)?;
                let fullmove = fullmove_str
                    .parse::<u16>()
                    .ok()
                    .filter(|&n| n >= 1)
                    .ok_or(FenError::InvalidFullmoveNumber)?;

                (halfmove, fullmove)
            }
        };

        let side_to_move = match side_str {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::InvalidSideToMove),
        };

        let mut board = Self::empty();
        let mut rank_count = 0_u8;

        for rank_str in placement.split('/') {
            if rank_count >= 5 {
                return Err(FenError::InvalidRankCount);
            }
            parse_rank(&mut board, rank_str, 4 - rank_count)?;
            rank_count += 1;
        }

        if rank_count != 5 {
            return Err(FenError::InvalidRankCount);
        }

        board.set_side_to_move(side_to_move);
        Ok((board, halfmove_clock, fullmove_number))
    }
}

/// Parses a single FEN rank string (`a`..`e` files) onto `board` at `rank` (`0..=4`).
fn parse_rank(board: &mut Board, rank_str: &str, rank: u8) -> Result<(), FenError> {
    let mut file = 0_u8;

    for ch in rank_str.chars() {
        match ch {
            '1'..='9' => {
                let empty = (ch as u8) - b'0';
                file += empty;
                if file > 5 {
                    return Err(FenError::InvalidFileCount);
                }
            }
            '0' => return Err(FenError::InvalidFileCount),
            _ => {
                let piece = Piece::from_char(ch).ok_or(FenError::InvalidPiece(ch))?;
                let sq = Square::new(file, rank).ok_or(FenError::InvalidFileCount)?;
                board.set_piece(sq, Some(piece));
                file += 1;
            }
        }
    }

    if file != 5 {
        return Err(FenError::InvalidFileCount);
    }

    Ok(())
}
