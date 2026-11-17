use crate::board::Board;
use crate::core::PieceType;
use crate::eval::MAX_PHASE;
use crate::search::types::is_mate_score;

/// Default `UCI_ShowWDL` setting (`false` for standard UCI compatibility).
pub const DEFAULT_SHOW_WDL: bool = false;

/// Win/Draw/Loss probabilities in permille (`win + draw + loss == 1000`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Wdl {
    /// Win probability in permille (`0..=1000`).
    pub win: u16,

    /// Draw probability in permille (`0..=1000`).
    pub draw: u16,

    /// Loss probability in permille (`0..=1000`).
    pub loss: u16,
}

impl Wdl {
    /// Computes WDL permille probabilities from a search `score` and game `phase` (`0..=MAX_PHASE`).
    #[must_use]
    pub fn from_score(score: i32, phase: i32) -> Self {
        if is_mate_score(score) {
            return if score > 0 {
                Self {
                    win: 1000,
                    draw: 0,
                    loss: 0,
                }
            } else {
                Self {
                    win: 0,
                    draw: 0,
                    loss: 1000,
                }
            };
        }

        let clamped_phase = phase.clamp(0, MAX_PHASE);
        let phase_ratio = f64::from(clamped_phase) / f64::from(MAX_PHASE);
        // Endgame positions have lower draw offset (95cp) than opening/middlegame (120cp).
        let a = 25.0_f64.mul_add(phase_ratio, 95.0);
        let b = 60.0;
        let x = f64::from(score);

        let win_prob = 1.0 / (1.0 + ((a - x) / b).exp());
        let loss_prob = 1.0 / (1.0 + ((a + x) / b).exp());

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut win = (win_prob * 1000.0).round().clamp(0.0, 1000.0) as u16;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut loss = (loss_prob * 1000.0).round().clamp(0.0, 1000.0) as u16;
        if win + loss > 1000 {
            if win >= loss {
                win = 1000 - loss;
            } else {
                loss = 1000 - win;
            }
        }
        let draw = 1000 - win - loss;
        Self { win, draw, loss }
    }

    /// Computes WDL permille probabilities directly from `board` and `score`.
    #[must_use]
    pub fn from_board_score(board: &Board, score: i32) -> Self {
        Self::from_score(score, board_phase(board))
    }

    /// Returns the expected score in `[0.0, 1.0]` (`(win + 0.5 * draw) / 1000.0`).
    #[must_use]
    pub fn expected_score(self) -> f64 {
        0.5_f64.mul_add(f64::from(self.draw), f64::from(self.win)) / 1000.0
    }
}

/// Computes the tapered-eval material phase (`0..=MAX_PHASE`) for `board`.
#[must_use]
pub const fn board_phase(board: &Board) -> i32 {
    let q = board.piece_type_bb(PieceType::Queen).count().cast_signed();
    let r = board.piece_type_bb(PieceType::Rook).count().cast_signed();
    let b = board.piece_type_bb(PieceType::Bishop).count().cast_signed();
    let n = board.piece_type_bb(PieceType::Knight).count().cast_signed();
    let raw = q * 4 + r * 2 + b + n;
    if raw > MAX_PHASE { MAX_PHASE } else { raw }
}
