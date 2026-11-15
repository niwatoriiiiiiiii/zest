use crate::board::Board;
use crate::core::Move;
use crate::search::see::see_ge;
use crate::search::state::{HISTORY_MAX, KILLER_SECOND_SCORE};
use crate::search::types::{MAX_PLY, is_mate_score};

const MAX_RFP_DEPTH: u8 = 6;
const RFP_MARGIN_PER_PLY: i32 = 100;
const CAPTURE_SEE_MAX_DEPTH: u8 = 6;
const CAPTURE_SEE_MARGIN_PER_PLY: i32 = 90;
const LMP_MAX_DEPTH: u8 = 3;
const MAX_PRUNING_QUIET_DEPTH: u8 = 3;
const LMR_MIN_DEPTH: u8 = 3;

pub(super) struct PreparedNode {
    pub(super) depth: u8,
    pub(super) child_depth: u8,
    pub(super) rfp_cutoff: Option<i32>,
    pub(super) prune_ctx: MovePruneCtx,
}

#[derive(Clone, Copy)]
pub(super) struct MovePruneCtx {
    pub(super) depth: u8,
    pub(super) child_depth: u8,
    pub(super) is_pv: bool,
    pub(super) checked: bool,
    pub(super) alpha: i32,
    pub(super) futility_active: bool,
    pub(super) lmp_limit: usize,
}

impl MovePruneCtx {
    #[inline]
    pub(super) const fn prepare(
        raw_depth: u8,
        (alpha, beta): (i32, i32),
        is_pv: bool,
        static_eval: Option<i32>,
        has_tt_move: bool,
        ply: usize,
    ) -> PreparedNode {
        let checked = static_eval.is_none();
        let depth = if raw_depth >= 4 && !has_tt_move {
            raw_depth - 1
        } else {
            raw_depth
        };
        let eval = match static_eval {
            Some(v) => v,
            None => 0,
        };
        let rfp_cutoff = try_rfp(depth, beta, is_pv, checked, eval);
        let ext = if checked && ply < MAX_PLY / 2 { 1 } else { 0 };
        let child_depth = depth - 1 + ext;
        let prune_ctx = Self::new(depth, child_depth, is_pv, checked, alpha, eval);
        PreparedNode {
            depth,
            child_depth,
            rfp_cutoff,
            prune_ctx,
        }
    }

    #[inline]
    const fn new(
        depth: u8,
        child_depth: u8,
        is_pv: bool,
        checked: bool,
        alpha: i32,
        static_eval: i32,
    ) -> Self {
        let futility_active = !is_pv
            && !checked
            && depth <= MAX_PRUNING_QUIET_DEPTH
            && !is_mate_score(alpha)
            && static_eval + futility_margin(depth) <= alpha;
        Self {
            depth,
            child_depth,
            is_pv,
            checked,
            alpha,
            futility_active,
            lmp_limit: lmp_quiet_limit(depth),
        }
    }

    #[inline]
    pub(super) fn classify(
        self,
        board: &Board,
        mv: Move,
        idx: usize,
        quiet: bool,
        quiet_count: usize,
        move_score: i32,
    ) -> (bool, bool) {
        let can_prune_quiet = !self.is_pv
            && !self.checked
            && self.depth <= MAX_PRUNING_QUIET_DEPTH
            && !is_mate_score(self.alpha);
        let prunable_quiet = can_prune_quiet
            && quiet
            && idx > 0
            && move_score < KILLER_SECOND_SCORE
            && (self.futility_active || quiet_count > self.lmp_limit);
        let see_prunable = !quiet
            && !self.is_pv
            && !self.checked
            && idx > 0
            && mv.promotion().is_none()
            && !is_mate_score(self.alpha)
            && self.depth <= CAPTURE_SEE_MAX_DEPTH
            && move_score < 0
            && !see_ge(
                board,
                mv,
                -CAPTURE_SEE_MARGIN_PER_PLY * i32::from(self.depth),
            );
        let lmr_candidate =
            quiet && self.depth >= LMR_MIN_DEPTH && self.child_depth >= 2 && idx >= 2;
        (prunable_quiet || see_prunable, lmr_candidate)
    }
}

#[inline]
const fn try_rfp(
    depth: u8,
    beta: i32,
    is_pv: bool,
    checked: bool,
    static_eval: i32,
) -> Option<i32> {
    if can_rfp(depth, is_pv, checked, beta) && static_eval - rfp_margin(depth) >= beta {
        return Some(static_eval - rfp_margin(depth));
    }
    None
}

#[inline]
const fn can_rfp(depth: u8, is_pv: bool, checked: bool, beta: i32) -> bool {
    !is_pv && !checked && depth <= MAX_RFP_DEPTH && !is_mate_score(beta)
}

#[inline]
const fn rfp_margin(depth: u8) -> i32 {
    RFP_MARGIN_PER_PLY * (depth as i32)
}

#[inline]
const fn futility_margin(depth: u8) -> i32 {
    90 * (depth as i32)
}

#[inline]
const fn lmp_quiet_limit(depth: u8) -> usize {
    if depth > LMP_MAX_DEPTH {
        usize::MAX
    } else {
        3 + (depth as usize) * (depth as usize)
    }
}

#[inline]
pub(super) const fn lmr_reduction(
    depth: u8,
    child_depth: u8,
    move_idx: usize,
    is_pv: bool,
    move_score: i32,
) -> u8 {
    if depth < LMR_MIN_DEPTH || child_depth < 2 || move_idx < 2 {
        return 0;
    }

    #[allow(clippy::cast_possible_truncation)]
    let capped_idx = if move_idx < 31 { move_idx } else { 31 };
    #[allow(clippy::cast_possible_truncation)]
    let log_prod = (depth.ilog2() as u8) * (capped_idx.ilog2() as u8);
    let mut r = 1 + log_prod / 3;

    if is_pv {
        r = r.saturating_sub(1);
    }
    if move_score >= KILLER_SECOND_SCORE || move_score > HISTORY_MAX + 200 {
        r = r.saturating_sub(1);
    } else if move_score >= 0 && move_score < HISTORY_MAX - 200 {
        r += 1;
    }

    let max_red = child_depth - 1;
    if r > max_red { max_red } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfp_and_lmp_margins_respect_bounds() {
        assert!(!can_rfp(1, true, false, 100));
        assert!(!can_rfp(1, false, true, 100));
        assert!(can_rfp(6, false, false, 100));
        assert!(!can_rfp(7, false, false, 100));
        assert_eq!(rfp_margin(3), 300);
        assert_eq!(futility_margin(2), 180);
        assert_eq!(lmp_quiet_limit(1), 4);
        assert_eq!(lmp_quiet_limit(4), usize::MAX);
    }

    #[test]
    fn lmr_reduction_stays_bounded_and_respects_pv_and_history() {
        assert_eq!(lmr_reduction(2, 1, 4, false, 0), 0);
        assert_eq!(lmr_reduction(6, 5, 1, false, 0), 0);

        let non_pv = lmr_reduction(10, 9, 8, false, 0);
        let pv = lmr_reduction(10, 9, 8, true, 0);
        let killer = lmr_reduction(10, 9, 8, false, KILLER_SECOND_SCORE);
        assert!(non_pv >= pv);
        assert!(non_pv >= killer);
        assert!(non_pv < 9);
    }
}
