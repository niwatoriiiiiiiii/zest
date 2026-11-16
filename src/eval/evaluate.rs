use crate::board::{Bitboard, Board};
use crate::core::{Color, PieceType, Square};
use crate::eval::tables::{
    MAX_PHASE, PHASE_WEIGHTS, PIECE_VALUES, Score, pst_score, relative_square_index,
};

/// Tempo bonus awarded to the side to move (in centipawns).
pub const TEMPO_BONUS: i32 = 10;

const PASSED_PAWN_BONUS: [Score; 5] = [
    Score::ZERO,
    Score::new(4, 4),
    Score::new(21, 56),
    Score::new(53, 66),
    Score::ZERO,
];

const PASSED_FREE_ADVANCE_BONUS: [Score; 5] = [
    Score::ZERO,
    Score::new(17, 0),
    Score::new(18, 4),
    Score::new(21, 0),
    Score::ZERO,
];

const KING_PASSER_ENEMY_BONUS: Score = Score::new(10, 19);
const KING_PASSER_OWN_BONUS: Score = Score::new(6, 13);
const PAWN_PHALANX_BONUS: Score = Score::new(47, 38);
const MINOR_PAWN_SUPPORT_BONUS: Score = Score::new(39, 0);
const PAWN_THREAT_NON_PAWN_BONUS: Score = Score::new(70, 13);
const QUEEN_TROPISM_BONUS: Score = Score::new(33, 14);

const ISOLATED_PAWN_PENALTY: Score = Score::new(-18, -10);
const DOUBLED_PAWN_PENALTY: Score = Score::new(-4, -10);
const BISHOP_PAIR_BONUS: Score = Score::new(15, 25);
const ROOK_OPEN_FILE_BONUS: Score = Score::new(64, 24);
const ROOK_SEMI_OPEN_FILE_BONUS: Score = Score::new(32, 11);

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

const PASSED_FREE_ADVANCE_SCORES: [[Score; 25]; 2] = {
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
            table[color.index()][sq_i] = PASSED_FREE_ADVANCE_BONUS[rel_rank];
            sq_i += 1;
        }
        c_i += 1;
    }
    table
};

const PAWN_ADVANCE_MASK: [[Bitboard; 25]; 2] = {
    let mut table = [[Bitboard::EMPTY; 25]; 2];
    let mut sq_i = 0;
    while sq_i < 25 {
        if sq_i + 5 < 25 {
            table[Color::White.index()][sq_i] = Bitboard::from_raw(1u32 << (sq_i + 5));
        }
        if sq_i >= 5 {
            table[Color::Black.index()][sq_i] = Bitboard::from_raw(1u32 << (sq_i - 5));
        }
        sq_i += 1;
    }
    table
};

const CHEBYSHEV_DIST: [[i32; 25]; 25] = {
    let mut table = [[0i32; 25]; 25];
    let mut a = 0;
    while a < 25 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let af = (a % 5) as i32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let ar = (a / 5) as i32;
        let mut b = 0;
        while b < 25 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let bf = (b % 5) as i32;
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let br = (b / 5) as i32;
            let df = (af - bf).abs();
            let dr = (ar - br).abs();
            table[a][b] = if df > dr { df } else { dr };
            b += 1;
        }
        a += 1;
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
const fn scale_signed(unit: Score, factor: i32) -> Score {
    Score::new(unit.mg * factor, unit.eg * factor)
}

#[inline]
const fn pawn_attacks_raw(pawns_raw: u32, color: Color) -> u32 {
    const NOT_FILE_A: u32 = !0x0010_8421;
    const NOT_FILE_E: u32 = !0x0108_4210;
    match color {
        Color::White => {
            (((pawns_raw & NOT_FILE_A) << 4) | ((pawns_raw & NOT_FILE_E) << 6)) & 0x01FF_FFFF
        }
        Color::Black => ((pawns_raw & NOT_FILE_E) >> 4) | ((pawns_raw & NOT_FILE_A) >> 6),
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn evaluate_pawns(
    board: &Board,
    color: Color,
    own_files: u32,
    own_pawns: Bitboard,
    enemy_pawns: Bitboard,
    own_king_sq: Option<Square>,
    enemy_king_sq: Option<Square>,
    own_pawn_atk: u32,
) -> Score {
    const NOT_FILE_E: u32 = !0x0108_4210;
    let c_idx = color.index();
    let enemy = color.opposite();
    let occupied = board.occupied();
    let mut score = Score::ZERO;
    let mut pawn_bb = own_pawns;
    let passed_masks = &PASSED_PAWN_MASKS[c_idx];
    let passed_scores = &PASSED_PAWN_SCORES[c_idx];
    let passed_free_scores = &PASSED_FREE_ADVANCE_SCORES[c_idx];
    let advance_masks = &PAWN_ADVANCE_MASK[c_idx];
    let pawn_psqt = &BAKED_PSQT[PieceType::Pawn.index()][c_idx];
    while let Some(sq) = pawn_bb.pop_lsb() {
        let idx = sq.index();
        score += pawn_psqt[idx];
        if !enemy_pawns.intersects(passed_masks[idx]) {
            score += passed_scores[idx];
            if !occupied.intersects(advance_masks[idx]) {
                score += passed_free_scores[idx];
            }
            let rel_rank = relative_square_index(color, sq) / 5;
            if rel_rank >= 2 {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let r_scale = (rel_rank - 1) as i32;
                let promo_idx = match color {
                    Color::White => 20 + (idx % 5),
                    Color::Black => idx % 5,
                };
                if let Some(ek) = enemy_king_sq {
                    score += scale_signed(
                        KING_PASSER_ENEMY_BONUS,
                        r_scale * (CHEBYSHEV_DIST[ek.index()][promo_idx] - 2),
                    );
                }
                if let Some(ok) = own_king_sq {
                    score += scale_signed(
                        KING_PASSER_OWN_BONUS,
                        r_scale * (2 - CHEBYSHEV_DIST[ok.index()][idx]),
                    );
                }
            }
        }
    }

    let p_raw = own_pawns.as_raw();
    let phalanx_count = (p_raw & ((p_raw & NOT_FILE_E) << 1)).count_ones();
    if phalanx_count > 0 {
        score += scale_score(PAWN_PHALANX_BONUS, phalanx_count);
    }

    let enemy_non_pawns = board.color_bb(enemy).as_raw() & !enemy_pawns.as_raw();
    let threat_count = (own_pawn_atk & enemy_non_pawns).count_ones();
    if threat_count > 0 {
        score += scale_score(PAWN_THREAT_NON_PAWN_BONUS, threat_count);
    }

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

    score
}

#[derive(Clone, Copy)]
struct SideEvalCtx {
    color: Color,
    own_pawns: Bitboard,
    enemy_pawns: Bitboard,
    own_files: u32,
    enemy_files: u32,
    own_pawn_atk: u32,
    own_king_sq: Option<Square>,
    enemy_king_sq: Option<Square>,
}

#[inline]
fn evaluate_side(board: &Board, ctx: SideEvalCtx) -> (Score, i32) {
    let color = ctx.color;
    let c_idx = color.index();

    let mut score = evaluate_pawns(
        board,
        color,
        ctx.own_files,
        ctx.own_pawns,
        ctx.enemy_pawns,
        ctx.own_king_sq,
        ctx.enemy_king_sq,
        ctx.own_pawn_atk,
    );

    // Knights & Bishops (+ pair + pawn outpost support + bishop mobility)
    let knights = board.pieces_bb(color, PieceType::Knight);
    score += sum_baked_psqt(knights, &BAKED_PSQT[PieceType::Knight.index()][c_idx]);

    let bishops = board.pieces_bb(color, PieceType::Bishop);
    let bishop_psqt = &BAKED_PSQT[PieceType::Bishop.index()][c_idx];
    let mut bishop_bb = bishops;
    while let Some(sq) = bishop_bb.pop_lsb() {
        score += bishop_psqt[sq.index()];
    }
    if bishops.count() >= 2 {
        score += BISHOP_PAIR_BONUS;
    }

    let defended_minors = ((knights.as_raw() | bishops.as_raw()) & ctx.own_pawn_atk).count_ones();
    if defended_minors > 0 {
        score += scale_score(MINOR_PAWN_SUPPORT_BONUS, defended_minors);
    }

    // Rooks (+ open / semi-open file bonuses via 5-bit file masks + mobility)
    let rooks = board.pieces_bb(color, PieceType::Rook);
    let rook_psqt = &BAKED_PSQT[PieceType::Rook.index()][c_idx];
    let mut rook_bb = rooks;
    while let Some(sq) = rook_bb.pop_lsb() {
        let idx = sq.index();
        score += rook_psqt[idx];
        let file_bit = FILE_BIT_BY_SQ[idx];
        if (ctx.own_files & file_bit) == 0 {
            if (ctx.enemy_files & file_bit) == 0 {
                score += ROOK_OPEN_FILE_BONUS;
            } else {
                score += ROOK_SEMI_OPEN_FILE_BONUS;
            }
        }
    }

    // Queens (+ king tropism + mobility) & King
    let queens = board.pieces_bb(color, PieceType::Queen);
    let queen_psqt = &BAKED_PSQT[PieceType::Queen.index()][c_idx];
    let mut queen_bb = queens;
    while let Some(sq) = queen_bb.pop_lsb() {
        let idx = sq.index();
        score += queen_psqt[idx];
        if let Some(ek) = ctx.enemy_king_sq {
            let trop = 3 - CHEBYSHEV_DIST[idx][ek.index()];
            if trop > 0 {
                score += scale_signed(QUEEN_TROPISM_BONUS, trop);
            }
        }
    }

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

#[inline]
fn nnue_scale(board: &Board) -> i32 {
    (board.occupied().as_raw() & !board.piece_type_bb(PieceType::King).as_raw())
        .count_ones()
        .saturating_sub(1)
        .min(4)
        .cast_signed()
}

#[inline]
fn evaluate_breakdown_core(board: &Board, scale: i32, (nn_w, nn_b): (i32, i32)) -> EvalBreakdown {
    let white_pawns = board.pieces_bb(Color::White, PieceType::Pawn);
    let black_pawns = board.pieces_bb(Color::Black, PieceType::Pawn);
    let white_files = pawn_files_mask(white_pawns.as_raw());
    let black_files = pawn_files_mask(black_pawns.as_raw());
    let white_pawn_atk = pawn_attacks_raw(white_pawns.as_raw(), Color::White);
    let black_pawn_atk = pawn_attacks_raw(black_pawns.as_raw(), Color::Black);
    let white_king_sq = board.pieces_bb(Color::White, PieceType::King).lsb();
    let black_king_sq = board.pieces_bb(Color::Black, PieceType::King).lsb();

    let (white, white_phase) = evaluate_side(
        board,
        SideEvalCtx {
            color: Color::White,
            own_pawns: white_pawns,
            enemy_pawns: black_pawns,
            own_files: white_files,
            enemy_files: black_files,
            own_pawn_atk: white_pawn_atk,
            own_king_sq: white_king_sq,
            enemy_king_sq: black_king_sq,
        },
    );
    let (black, black_phase) = evaluate_side(
        board,
        SideEvalCtx {
            color: Color::Black,
            own_pawns: black_pawns,
            enemy_pawns: white_pawns,
            own_files: black_files,
            enemy_files: white_files,
            own_pawn_atk: black_pawn_atk,
            own_king_sq: black_king_sq,
            enemy_king_sq: white_king_sq,
        },
    );
    let white = white + Score::new((nn_w * scale) / 4, (nn_w * scale) / 4);
    let black = black + Score::new((nn_b * scale) / 4, (nn_b * scale) / 4);
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

/// Evaluates the given `board` and returns a full [`EvalBreakdown`].
#[must_use]
#[inline]
pub fn evaluate_breakdown(board: &Board) -> EvalBreakdown {
    let scale = nnue_scale(board);
    let nn_res = if scale == 0 {
        (0, 0)
    } else {
        crate::eval::nnue::evaluate_nnue_residual(board)
    };
    evaluate_breakdown_core(board, scale, nn_res)
}

/// Evaluates `board` in centipawns from the side-to-move perspective using an incremental [`AccStack`](crate::eval::AccStack).
#[must_use]
#[inline]
pub fn evaluate_with_acc(board: &Board, acc: &mut crate::eval::nnue::AccStack, ply: usize) -> i32 {
    let scale = nnue_scale(board);
    let nn_res = if scale == 0 {
        (0, 0)
    } else {
        let res = acc.evaluate_residual(board, ply);
        debug_assert_eq!(res, crate::eval::nnue::evaluate_nnue_residual(board));
        res
    };
    evaluate_breakdown_core(board, scale, nn_res).total
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
