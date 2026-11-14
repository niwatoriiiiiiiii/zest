use core::sync::atomic::{AtomicBool, Ordering};

use crate::board::Board;
use crate::core::{Move, PieceType, Square};
use crate::search::types::MAX_PLY;

const DUMMY_MOVE: Move = Move::new(Square::A1, Square::A1, None);

/// Mutable per-search buffers for node accounting, repetition tracking, and PV collection.
#[derive(Debug)]
pub(super) struct SearchState {
    pub(super) nodes: u64,
    pub(super) max_nodes: Option<u64>,
    pub(super) aborted: bool,
    pub(super) path_keys: Vec<u64>,
    pub(super) pv_table: Vec<[Move; MAX_PLY]>,
    pub(super) pv_len: [usize; MAX_PLY + 1],
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
        }
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
