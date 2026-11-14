use core::sync::atomic::{AtomicBool, Ordering};

use crate::board::Board;
use crate::core::{Color, Move, PieceType, Square};
use crate::search::types::MAX_PLY;

const DUMMY_MOVE: Move = Move::new(Square::A1, Square::A1, None);
const NUM_SQUARES: usize = 25;

/// Maximum magnitude stored per `(color, from, to)` entry in the history table.
pub(super) const HISTORY_MAX: i32 = 800;

/// Ordering score added to quiet killer moves at the current ply.
pub(super) const KILLER_FIRST_SCORE: i32 = 1_900;
pub(super) const KILLER_SECOND_SCORE: i32 = 1_800;

/// Per-node ply, repetition/fifty-move state, and cancellation flag passed down recursion.
#[derive(Clone, Copy)]
pub(super) struct NodeCtx<'a> {
    pub(super) ply: usize,
    pub(super) halfmove: u16,
    pub(super) reversible_start: usize,
    pub(super) stop: Option<&'a AtomicBool>,
}

impl NodeCtx<'_> {
    #[inline]
    pub(super) fn advance(self, board: &mut Board, mv: Move, path_len: usize) -> Self {
        let (halfmove, reversible_start) =
            advance_board_and_clock(board, mv, self.halfmove, self.reversible_start, path_len);
        Self {
            ply: self.ply + 1,
            halfmove,
            reversible_start,
            stop: self.stop,
        }
    }
}

/// Mutable per-search buffers for node accounting, repetition tracking, heuristics, and PV.
#[derive(Debug)]
pub(super) struct SearchState {
    pub(super) nodes: u64,
    pub(super) max_nodes: Option<u64>,
    pub(super) aborted: bool,
    pub(super) path_keys: Vec<u64>,
    pub(super) pv_table: Vec<[Move; MAX_PLY]>,
    pub(super) pv_len: [usize; MAX_PLY + 1],
    killers: [[Option<Move>; 2]; MAX_PLY],
    history: [[[i32; NUM_SQUARES]; NUM_SQUARES]; 2],
}

impl SearchState {
    pub(super) fn new() -> Self {
        Self {
            nodes: 0,
            max_nodes: None,
            aborted: false,
            path_keys: Vec::with_capacity(256),
            pv_table: vec![[DUMMY_MOVE; MAX_PLY]; MAX_PLY + 1],
            pv_len: [0; MAX_PLY + 1],
            killers: [[None; 2]; MAX_PLY],
            history: [[[0; NUM_SQUARES]; NUM_SQUARES]; 2],
        }
    }

    pub(super) fn clear_heuristics(&mut self) {
        self.killers.fill([None; 2]);
        self.history.fill([[0; NUM_SQUARES]; NUM_SQUARES]);
    }

    pub(super) fn reset(&mut self, root_key: u64, history: &[u64], max_nodes: Option<u64>) {
        self.nodes = 0;
        self.max_nodes = max_nodes;
        self.aborted = false;
        self.path_keys.clear();
        self.path_keys.extend_from_slice(history);
        if self.path_keys.last().copied() != Some(root_key) {
            self.path_keys.push(root_key);
        }
        self.killers.fill([None; 2]);
        for side in &mut self.history {
            for row in side {
                for slot in row {
                    *slot /= 2;
                }
            }
        }
    }

    pub(super) fn should_stop(&self, stop: Option<&AtomicBool>) -> bool {
        stop.is_some_and(|flag| flag.load(Ordering::Relaxed))
            || self.max_nodes.is_some_and(|limit| self.nodes >= limit)
    }

    #[inline]
    pub(super) fn check_limits(&mut self, stop: Option<&AtomicBool>) -> bool {
        if self.aborted {
            return true;
        }
        if self.max_nodes.is_some_and(|limit| self.nodes >= limit) {
            self.aborted = true;
            return true;
        }
        if self.nodes.trailing_zeros() >= 10
            && stop.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            self.aborted = true;
            return true;
        }
        false
    }

    #[inline]
    pub(super) fn is_repetition(&self, current_key: u64, reversible_start: usize) -> bool {
        // Twofold repetition inside the search tree detects cycles one turn earlier.
        if self.path_keys.len() < reversible_start + 3 {
            return false;
        }
        self.path_keys[reversible_start..]
            .iter()
            .rev()
            .skip(2)
            .step_by(2)
            .any(|&key| key == current_key)
    }

    #[inline]
    pub(super) fn update_pv(&mut self, ply: usize, mv: Move) {
        let child_end = self.pv_len[ply + 1].min(MAX_PLY);
        let copy_len = child_end.saturating_sub(ply + 1);
        self.pv_table[ply][ply] = mv;
        let (upper, lower) = self.pv_table.split_at_mut(ply + 1);
        upper[ply][(ply + 1)..(ply + 1 + copy_len)]
            .copy_from_slice(&lower[0][(ply + 1)..(ply + 1 + copy_len)]);
        self.pv_len[ply] = ply + 1 + copy_len;
    }

    #[inline]
    pub(super) fn quiet_move_score(&self, color: Color, ply: usize, mv: Move) -> i32 {
        if ply < MAX_PLY {
            let [first, second] = self.killers[ply];
            if first == Some(mv) {
                return KILLER_FIRST_SCORE;
            }
            if second == Some(mv) {
                return KILLER_SECOND_SCORE;
            }
        }
        let hist = self.history[color.index()][usize::from(mv.from().as_u8())]
            [usize::from(mv.to().as_u8())];
        HISTORY_MAX + hist
    }

    #[inline]
    pub(super) fn record_killer(&mut self, ply: usize, mv: Move) {
        if ply < MAX_PLY && self.killers[ply][0] != Some(mv) {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = Some(mv);
        }
    }

    #[inline]
    pub(super) fn update_history(&mut self, color: Color, mv: Move, delta: i32) {
        let clamped = delta.clamp(-HISTORY_MAX, HISTORY_MAX);
        let slot = &mut self.history[color.index()][usize::from(mv.from().as_u8())]
            [usize::from(mv.to().as_u8())];
        *slot += clamped - (*slot * clamped.abs()) / HISTORY_MAX;
    }
}

#[inline]
pub(super) const fn is_quiet_move(board: &Board, mv: Move) -> bool {
    board.piece_at(mv.to()).is_none() && mv.promotion().is_none()
}

#[inline]
pub(super) fn advance_board_and_clock(
    board: &mut Board,
    mv: Move,
    halfmove_clock: u16,
    reversible_start: usize,
    next_rev_index: usize,
) -> (u16, usize) {
    let is_pawn_move = board
        .piece_at(mv.from())
        .is_some_and(|p| p.piece_type == PieceType::Pawn);
    let is_capture = board.piece_at(mv.to()).is_some();

    board.make_move(mv);

    if is_pawn_move || is_capture {
        (0, next_rev_index)
    } else {
        (halfmove_clock.saturating_add(1), reversible_start)
    }
}
