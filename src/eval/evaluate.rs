use crate::board::{Bitboard, Board};
use crate::core::{Color, PieceType, Square};
use crate::eval::tables::{
    MAX_PHASE, PHASE_WEIGHTS, PIECE_VALUES, Score, pst_score, relative_square_index,
};

/// Tempo bonus awarded to the side to move (in centipawns).
pub const TEMPO_BONUS: i32 = 10;

const PASSED_PAWN_BONUS: [Score; 5] = [
    Score::ZERO,
    Score::new(6, 10),
    Score::new(18, 28),
    Score::new(40, 62),
    Score::ZERO,
];

const ISOLATED_PAWN_PENALTY: Score = Score::new(-8, -12);
const DOUBLED_PAWN_PENALTY: Score = Score::new(-8, -10);
const BISHOP_PAIR_BONUS: Score = Score::new(15, 25);
const ROOK_OPEN_FILE_BONUS: Score = Score::new(16, 12);
const ROOK_SEMI_OPEN_FILE_BONUS: Score = Score::new(8, 6);

/// Broadcasts a 5-bit file mask across all 5 ranks without carry overlap (`bit r*5 + f`).
const FILE_BROADCAST_MULTIPLIER: u32 = (1 << 20) | (1 << 15) | (1 << 10) | (1 << 5) | 1;
const FIVE_FILES_MASK: u32 = 0b1_1111;

const PIECE_TYPES: [PieceType; 6] = [
    PieceType::Pawn,
    PieceType::Knight,
    PieceType::Bishop,
    PieceType::Rook,
    PieceType::Queen,
    PieceType::King,
];

/// Pre-baked `PIECE_VALUES + PST` indexed by `[PieceType::index][Color::index][Square::index]`.
///
/// Eliminates runtime `/ 5` and `% 5` coordinate conversions as well as per-piece material adds.
const BAKED_PSQT: [[[Score; 25]; 2]; 6] = {
    let mut table = [[[Score::ZERO; 25]; 2]; 6];
    let colors = [Color::White, Color::Black];
    let mut pt_i = 0;
    while pt_i < 6 {
        let pt = PIECE_TYPES[pt_i];
        let base = PIECE_VALUES[pt.index()];
        let mut c_i = 0;
        while c_i < 2 {
            let color = colors[c_i];
            let mut sq_i = 0;
            while sq_i < 25 {
                #[allow(clippy::cast_possible_truncation)]
                let sq = Square::from_index(sq_i as u8).expect("valid square index");
                table[pt.index()][color.index()][sq_i] = base.plus(pst_score(pt, color, sq));
                sq_i += 1;
            }
            c_i += 1;
        }
        pt_i += 1;
    }
    table
};

const PASSED_PAWN_SCORES: [[Score; 25]; 2] = {
    let mut table = [[Score::ZERO; 25]; 2];
    let colors = [Color::White, Color::Black];
    let mut c_i = 0;
    while c_i < 2 {
        let color = colors[c_i];
        let mut sq_i = 0;
        while sq_i < 25 {
            #[allow(clippy::cast_possible_truncation)]
            let sq = Square::from_index(sq_i as u8).expect("valid square index");
            let rel_rank = relative_square_index(color, sq) / 5;
            table[color.index()][sq_i] = PASSED_PAWN_BONUS[rel_rank];
            sq_i += 1;
        }
        c_i += 1;
    }
    table
};

const FILE_BIT_BY_SQ: [u32; 25] = {
    let mut bits = [0u32; 25];
    let mut sq_i = 0;
    while sq_i < 25 {
        bits[sq_i] = 1u32 << (sq_i % 5);
        sq_i += 1;
    }
    bits
};

const PASSED_PAWN_MASKS: [[Bitboard; 25]; 2] = {
    let mut file_masks = [0u32; 5];
    let mut f = 0;
    while f < 5 {
        file_masks[f] = (1u32 << f) * FILE_BROADCAST_MULTIPLIER;
        f += 1;
    }

    let mut masks = [[Bitboard::EMPTY; 25]; 2];
    let mut sq_idx = 0;
    while sq_idx < 25 {
        let rank = sq_idx / 5;
        let file = sq_idx % 5;
        let mut span_files = file_masks[file];
        if file > 0 {
            span_files |= file_masks[file - 1];
        }
        if file + 1 < 5 {
            span_files |= file_masks[file + 1];
        }

        let mut white_ranks = 0u32;
        let mut r = rank + 1;
        while r < 5 {
            white_ranks |= FIVE_FILES_MASK << (r * 5);
            r += 1;
        }

        let mut black_ranks = 0u32;
        let mut br = 0;
        while br < rank {
            black_ranks |= FIVE_FILES_MASK << (br * 5);
            br += 1;
        }

        masks[Color::White.index()][sq_idx] = Bitboard::from_raw(span_files & white_ranks);
        masks[Color::Black.index()][sq_idx] = Bitboard::from_raw(span_files & black_ranks);
        sq_idx += 1;
    }
    masks
};

/// Detailed evaluation output for diagnostics and testing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EvalBreakdown {
    /// White's raw tapered score (`mg`, `eg`).
    pub white: Score,

    /// Black's raw tapered score (`mg`, `eg`).
    pub black: Score,

    /// Clamped game phase weight in `0..=MAX_PHASE`.
    pub phase: i32,

    /// Signed tempo contribution applied for the side to move.
    pub tempo: i32,

    /// Final static evaluation from the side-to-move perspective (in centipawns).
    pub total: i32,
}

#[inline]
const fn pawn_below_mask(pawns_raw: u32) -> u32 {
    (pawns_raw >> 5) | (pawns_raw >> 10) | (pawns_raw >> 15) | (pawns_raw >> 20)
}

#[inline]
const fn pawn_files_mask(pawns_raw: u32) -> u32 {
    (pawns_raw | pawn_below_mask(pawns_raw)) & FIVE_FILES_MASK
}

#[inline]
const fn scale_score(unit: Score, count: u32) -> Score {
    // Counts on a 5x5 board are at most 25, so casting to `i32` never wraps.
    #[allow(clippy::cast_possible_wrap)]
    let n = count as i32;
    Score::new(unit.mg * n, unit.eg * n)
}

#[inline]
fn sum_baked_psqt(mut bb: Bitboard, table: &[Score; 25]) -> Score {
    let mut score = Score::ZERO;
    while let Some(sq) = bb.pop_lsb() {
        score += table[sq.index()];
    }
    score
}

#[inline]
fn evaluate_side(board: &Board, color: Color, own_files: u32, enemy_files: u32) -> (Score, i32) {
    let c_idx = color.index();
    let own_pawns = board.pieces_bb(color, PieceType::Pawn);
    let enemy_pawns = board.pieces_bb(color.opposite(), PieceType::Pawn);

    // Pawns: baked PSQT + passed pawn bonus + SWAR doubled/isolated penalties
    let mut score = Score::ZERO;
    let mut pawn_bb = own_pawns;
    let passed_masks = &PASSED_PAWN_MASKS[c_idx];
    let passed_scores = &PASSED_PAWN_SCORES[c_idx];
    let pawn_psqt = &BAKED_PSQT[PieceType::Pawn.index()][c_idx];
    while let Some(sq) = pawn_bb.pop_lsb() {
        let idx = sq.index();
        score += pawn_psqt[idx];
        if !enemy_pawns.intersects(passed_masks[idx]) {
            score += passed_scores[idx];
        }
    }

    let p_raw = own_pawns.as_raw();
    let doubled_count = (p_raw & pawn_below_mask(p_raw)).count_ones();
    if doubled_count > 0 {
        score += scale_score(DOUBLED_PAWN_PENALTY, doubled_count);
    }

    let neighbor_files = ((own_files << 1) | (own_files >> 1)) & FIVE_FILES_MASK;
    let isolated_files = own_files & !neighbor_files;
    let isolated_count = (p_raw & (isolated_files * FILE_BROADCAST_MULTIPLIER)).count_ones();
    if isolated_count > 0 {
        score += scale_score(ISOLATED_PAWN_PENALTY, isolated_count);
    }

    // Knights
    let knights = board.pieces_bb(color, PieceType::Knight);
    score += sum_baked_psqt(knights, &BAKED_PSQT[PieceType::Knight.index()][c_idx]);

    // Bishops (+ pair bonus)
    let bishops = board.pieces_bb(color, PieceType::Bishop);
    score += sum_baked_psqt(bishops, &BAKED_PSQT[PieceType::Bishop.index()][c_idx]);
    if bishops.count() >= 2 {
        score += BISHOP_PAIR_BONUS;
    }

    // Rooks (+ open / semi-open file bonuses via 5-bit file masks)
    let rooks = board.pieces_bb(color, PieceType::Rook);
    let rook_psqt = &BAKED_PSQT[PieceType::Rook.index()][c_idx];
    let mut rook_bb = rooks;
    while let Some(sq) = rook_bb.pop_lsb() {
        let idx = sq.index();
        score += rook_psqt[idx];
        let file_bit = FILE_BIT_BY_SQ[idx];
        if (own_files & file_bit) == 0 {
            if (enemy_files & file_bit) == 0 {
                score += ROOK_OPEN_FILE_BONUS;
            } else {
                score += ROOK_SEMI_OPEN_FILE_BONUS;
            }
        }
    }

    // Queens & King
    let queens = board.pieces_bb(color, PieceType::Queen);
    score += sum_baked_psqt(queens, &BAKED_PSQT[PieceType::Queen.index()][c_idx]);

    let kings = board.pieces_bb(color, PieceType::King);
    score += sum_baked_psqt(kings, &BAKED_PSQT[PieceType::King.index()][c_idx]);

    // Phase counts on a 5x5 board are bounded by 25; casting to `i32` never wraps.
    #[allow(clippy::cast_possible_wrap)]
    let phase = (knights.count() as i32) * PHASE_WEIGHTS[PieceType::Knight.index()]
        + (bishops.count() as i32) * PHASE_WEIGHTS[PieceType::Bishop.index()]
        + (rooks.count() as i32) * PHASE_WEIGHTS[PieceType::Rook.index()]
        + (queens.count() as i32) * PHASE_WEIGHTS[PieceType::Queen.index()];

    (score, phase)
}

/// Evaluates the given `board` and returns a full [`EvalBreakdown`].
#[must_use]
#[inline]
pub fn evaluate_breakdown(board: &Board) -> EvalBreakdown {
    let white_files = pawn_files_mask(board.pieces_bb(Color::White, PieceType::Pawn).as_raw());
    let black_files = pawn_files_mask(board.pieces_bb(Color::Black, PieceType::Pawn).as_raw());

    let (white, white_phase) = evaluate_side(board, Color::White, white_files, black_files);
    let (black, black_phase) = evaluate_side(board, Color::Black, black_files, white_files);
    let phase = (white_phase + black_phase).min(MAX_PHASE);

    let net = white - black;
    let tapered_white = (net.mg * phase + net.eg * (MAX_PHASE - phase)) / MAX_PHASE;
    let positional = match board.side_to_move() {
        Color::White => tapered_white,
        Color::Black => -tapered_white,
    };

    EvalBreakdown {
        white,
        black,
        phase,
        tempo: TEMPO_BONUS,
        total: positional + TEMPO_BONUS,
    }
}

/// Returns the static evaluation of `board` in centipawns from the side-to-move perspective.
#[must_use]
#[inline]
pub fn evaluate(board: &Board) -> i32 {
    evaluate_breakdown(board).total
}

const _: () = {
    assert!(Square::ALL.len() == 25);
};
