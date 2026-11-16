use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::core::{Color, PieceType, Square};

/// Tapered evaluation score separating middlegame (`mg`) and endgame (`eg`) terms in centipawns.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Score {
    /// Middlegame centipawn value.
    pub mg: i32,

    /// Endgame centipawn value.
    pub eg: i32,
}

impl Score {
    /// Zero evaluation score.
    pub const ZERO: Self = Self::new(0, 0);

    /// Creates a new tapered [`Score`].
    #[must_use]
    #[inline]
    pub const fn new(mg: i32, eg: i32) -> Self {
        Self { mg, eg }
    }

    /// Adds two scores in a `const` context.
    #[must_use]
    #[inline]
    pub const fn plus(self, rhs: Self) -> Self {
        Self::new(self.mg + rhs.mg, self.eg + rhs.eg)
    }

    /// Subtracts `rhs` from `self` in a `const` context.
    #[must_use]
    #[inline]
    pub const fn minus(self, rhs: Self) -> Self {
        Self::new(self.mg - rhs.mg, self.eg - rhs.eg)
    }
}

impl Add for Score {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.plus(rhs)
    }
}

impl AddAssign for Score {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl Sub for Score {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.minus(rhs)
    }
}

impl SubAssign for Score {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

/// Total game phase weight at the starting Gardner minichess position.
///
/// Computed from both sides: `2 * (Knight(1) + Bishop(1) + Rook(2) + Queen(4)) = 16`.
pub const MAX_PHASE: i32 = 16;

/// Game phase contribution for each [`PieceType`] indexed by [`PieceType::index`].
pub const PHASE_WEIGHTS: [i32; 6] = [
    0, // Pawn
    1, // Knight
    1, // Bishop
    2, // Rook
    4, // Queen
    0, // King
];

/// Material values in centipawns indexed by [`PieceType::index`].
///
/// Calibrated from 3,000 Gardner piece-odds games (`P=100, N=237.5, B=231.1, R=315.2, Q=552.9`),
/// split across middlegame and endgame phases.
pub const PIECE_VALUES: [Score; 6] = [
    Score::new(111, 17),  // Pawn
    Score::new(326, 151), // Knight
    Score::new(345, 95),  // Bishop
    Score::new(403, 201), // Rook
    Score::new(563, 535), // Queen
    Score::new(0, 0),     // King
];

const fn s(mg: i32, eg: i32) -> Score {
    Score::new(mg, eg)
}

/// Converts a visual top-down 5x5 matrix (rank 5 down to rank 1, files A..E) into board index order.
const fn from_top_down(rows: [[Score; 5]; 5]) -> [Score; 25] {
    let mut out = [Score::ZERO; 25];
    let mut visual_r = 0;
    while visual_r < 5 {
        let rank = 4 - visual_r;
        let mut file = 0;
        while file < 5 {
            out[rank * 5 + file] = rows[visual_r][file];
            file += 1;
        }
        visual_r += 1;
    }
    out
}

/// Returns the White-perspective square index (`0..=24`) for a piece of `color` on `sq`.
///
/// Because Gardner minichess is horizontally aligned between sides (`R N B Q K` vs `r n b q k`),
/// Black's perspective only mirrors the rank (`4 - rank`) while preserving the file.
#[must_use]
pub const fn relative_square_index(color: Color, sq: Square) -> usize {
    let rank = match color {
        Color::White => sq.rank() as usize,
        Color::Black => 4 - sq.rank() as usize,
    };
    rank * 5 + sq.file() as usize
}

/// Returns the tapered piece-square bonus for `piece_type` belonging to `color` on `sq`.
#[must_use]
pub const fn pst_score(piece_type: PieceType, color: Color, sq: Square) -> Score {
    let idx = relative_square_index(color, sq);
    match piece_type {
        PieceType::Pawn => PAWN_PST[idx],
        PieceType::Knight => KNIGHT_PST[idx],
        PieceType::Bishop => BISHOP_PST[idx],
        PieceType::Rook => ROOK_PST[idx],
        PieceType::Queen => QUEEN_PST[idx],
        PieceType::King => KING_PST[idx],
    }
}

/// White-perspective piece-square table for Pawns.
pub const PAWN_PST: [Score; 25] = from_top_down([
    [s(0, 0), s(0, 0), s(0, 0), s(0, 0), s(0, 0)], // Rank 5 (promotion squares)
    [s(45, 30), s(50, 64), s(70, 47), s(36, 27), s(7, 21)], // Rank 4 (one step to promotion)
    [s(42, -8), s(58, -15), s(46, 6), s(69, -19), s(21, -11)], // Rank 3 (central push)
    [s(-31, 50), s(-42, 26), s(-33, 17), s(-32, 31), s(-15, 47)], // Rank 2 (starting rank)
    [s(0, 0), s(0, 0), s(0, 0), s(0, 0), s(0, 0)], // Rank 1
]);

/// White-perspective piece-square table for Knights.
pub const KNIGHT_PST: [Score; 25] = from_top_down([
    [s(-26, -14), s(-14, -18), s(-3, 0), s(-15, -22), s(-30, -35)], // Rank 5
    [s(5, 2), s(2, -3), s(28, 4), s(4, -3), s(-27, -18)],           // Rank 4
    [s(15, 24), s(29, 2), s(10, 0), s(13, 13), s(12, -10)],         // Rank 3
    [s(-6, -24), s(16, -20), s(24, 24), s(27, 0), s(-23, -15)],     // Rank 2
    [s(-27, -22), s(-16, -19), s(-1, 0), s(-8, -40), s(-28, -26)],  // Rank 1
]);

/// White-perspective piece-square table for Bishops.
pub const BISHOP_PST: [Score; 25] = from_top_down([
    [s(-23, -26), s(-4, -2), s(-7, -7), s(-4, -2), s(-8, -32)], // Rank 5
    [s(-2, 0), s(37, -12), s(7, 9), s(17, -12), s(-2, 0)],      // Rank 4
    [s(9, 3), s(8, 8), s(65, -2), s(8, 8), s(6, 6)],            // Rank 3
    [s(-2, 0), s(51, 0), s(6, 6), s(13, -4), s(-2, 0)],         // Rank 2
    [s(19, -17), s(-4, -4), s(-30, 4), s(-4, -4), s(-6, -22)],  // Rank 1
]);

/// White-perspective piece-square table for Rooks.
pub const ROOK_PST: [Score; 25] = from_top_down([
    [s(27, 30), s(16, 2), s(12, 12), s(8, 5), s(12, 18)], // Rank 5
    [s(34, 21), s(26, 8), s(27, 29), s(19, 14), s(12, 10)], // Rank 4 (4th rank penetration)
    [s(-21, 28), s(39, -2), s(9, 12), s(2, 3), s(6, 7)],  // Rank 3
    [s(-19, -5), s(-20, -19), s(34, -26), s(-4, -12), s(-5, -13)], // Rank 2
    [s(-15, -6), s(-7, -13), s(7, -12), s(9, -19), s(25, -9)], // Rank 1
]);

/// White-perspective piece-square table for Queens.
pub const QUEEN_PST: [Score; 25] = from_top_down([
    [s(-15, 3), s(12, 10), s(-2, 10), s(-7, 4), s(-12, 0)], // Rank 5
    [s(16, 14), s(24, 15), s(18, 18), s(4, 10), s(-6, 4)],  // Rank 4
    [s(-5, 4), s(50, 31), s(10, 24), s(27, 18), s(-17, 0)], // Rank 3
    [s(-19, -10), s(3, -5), s(11, 19), s(-3, -9), s(-2, 6)], // Rank 2
    [
        s(-24, -17),
        s(-6, -18),
        s(-34, -26),
        s(-23, -11),
        s(-9, -15),
    ], // Rank 1
]);

/// White-perspective piece-square table for Kings.
pub const KING_PST: [Score; 25] = from_top_down([
    [
        s(-40, -30),
        s(-35, -24),
        s(-35, -2),
        s(-35, -24),
        s(-40, -18),
    ], // Rank 5
    [s(-35, -28), s(-25, 1), s(-25, 39), s(-25, -1), s(-35, -3)], // Rank 4
    [s(-25, -12), s(-18, 3), s(-20, 5), s(-18, 7), s(-25, 8)],    // Rank 3
    [s(-6, 5), s(-10, -9), s(-12, 1), s(-6, 0), s(-4, 2)],        // Rank 2
    [s(10, 13), s(12, 3), s(4, -9), s(12, -17), s(18, -21)],      // Rank 1 (e1 safest in MG)
]);
