use std::collections::HashMap;

use zest::core::{Color, Move};
use zest::game::{DrawReason, Game, GameStatus};

use crate::uci_player::{SearchSpec, UciPlayer};

#[derive(Clone, Debug, Default)]
pub struct EngineStats {
    pub moves: u64,
    pub sum_depth: u64,
    pub depth_samples: u64,
    pub sum_nodes: u64,
    pub node_samples: u64,
    pub sum_nps: u64,
    pub nps_samples: u64,
}

impl EngineStats {
    pub const fn merge(&mut self, other: &Self) {
        self.moves += other.moves;
        self.sum_depth += other.sum_depth;
        self.depth_samples += other.depth_samples;
        self.sum_nodes += other.sum_nodes;
        self.node_samples += other.node_samples;
        self.sum_nps += other.sum_nps;
        self.nps_samples += other.nps_samples;
    }

    pub const fn record(&mut self, depth: Option<u64>, nodes: Option<u64>, nps: Option<u64>) {
        self.moves += 1;
        if let Some(d) = depth {
            self.sum_depth += d;
            self.depth_samples += 1;
        }
        if let Some(n) = nodes {
            self.sum_nodes += n;
            self.node_samples += 1;
        }
        if let Some(v) = nps {
            self.sum_nps += v;
            self.nps_samples += 1;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GameOutcome {
    WinA,
    Draw,
    WinB,
}

#[derive(Clone, Debug)]
pub struct GameRecord {
    pub outcome: GameOutcome,
    pub plies: u32,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct MatchRules {
    pub spec_a: SearchSpec,
    pub spec_b: SearchSpec,
    pub max_plies: u32,
    pub resign_cp: i32,
    pub resign_plies: u32,
}

pub fn play_pair(
    opening: &[String],
    rules: MatchRules,
    player_a: &mut UciPlayer,
    player_b: &mut UciPlayer,
    stats_a: &mut EngineStats,
    stats_b: &mut EngineStats,
) -> Result<[GameRecord; 2], String> {
    let g1 = play_single(opening, true, rules, player_a, player_b, stats_a, stats_b)?;
    let g2 = play_single(opening, false, rules, player_a, player_b, stats_a, stats_b)?;
    Ok([g1, g2])
}

#[allow(clippy::too_many_arguments)]
fn play_single(
    opening: &[String],
    a_is_white: bool,
    rules: MatchRules,
    player_a: &mut UciPlayer,
    player_b: &mut UciPlayer,
    stats_a: &mut EngineStats,
    stats_b: &mut EngineStats,
) -> Result<GameRecord, String> {
    player_a.new_game()?;
    player_b.new_game()?;

    let mut game = Game::new();
    let mut moves = Vec::with_capacity(opening.len() + 64);
    for raw in opening {
        let mv: Move = raw
            .parse()
            .map_err(|e| format!("invalid opening move {raw}: {e}"))?;
        game.try_move(mv)
            .map_err(|e| format!("illegal opening move {raw}: {e}"))?;
        moves.push(raw.clone());
    }

    let mut plies = 0u32;
    let mut resign_streak = [0u32; 2];

    loop {
        let status = game.status();
        if status != GameStatus::Ongoing {
            return Ok(GameRecord {
                outcome: status_to_outcome(status, a_is_white),
                plies,
                reason: status_label(status),
            });
        }
        if plies >= rules.max_plies {
            return Ok(GameRecord {
                outcome: GameOutcome::Draw,
                plies,
                reason: "max_plies",
            });
        }

        let side = game.board().side_to_move();
        let side_is_a = (side == Color::White) == a_is_white;
        let (player, spec, engine_stats) = if side_is_a {
            (&mut *player_a, rules.spec_a, &mut *stats_a)
        } else {
            (&mut *player_b, rules.spec_b, &mut *stats_b)
        };

        let res = player.go(&moves, spec)?;
        engine_stats.record(res.depth, res.nodes, res.nps);

        let Some(best_str) = res.bestmove else {
            let outcome = if side_is_a {
                GameOutcome::WinB
            } else {
                GameOutcome::WinA
            };
            return Ok(GameRecord {
                outcome,
                plies,
                reason: "no_bestmove",
            });
        };

        let Ok(mv) = best_str.parse::<Move>() else {
            let outcome = if side_is_a {
                GameOutcome::WinB
            } else {
                GameOutcome::WinA
            };
            return Ok(GameRecord {
                outcome,
                plies,
                reason: "illegal_move",
            });
        };

        if game.try_move(mv).is_err() {
            let outcome = if side_is_a {
                GameOutcome::WinB
            } else {
                GameOutcome::WinA
            };
            return Ok(GameRecord {
                outcome,
                plies,
                reason: "illegal_move",
            });
        }

        moves.push(best_str);
        plies += 1;

        if rules.resign_cp > 0 {
            let slot = usize::from(!side_is_a);
            if res.score_cp.is_some_and(|cp| cp <= -rules.resign_cp) {
                resign_streak[slot] += 1;
            } else {
                resign_streak[slot] = 0;
            }
            if resign_streak[slot] >= rules.resign_plies {
                let outcome = if side_is_a {
                    GameOutcome::WinB
                } else {
                    GameOutcome::WinA
                };
                return Ok(GameRecord {
                    outcome,
                    plies,
                    reason: "resign_adj",
                });
            }
        }
    }
}

const fn status_to_outcome(status: GameStatus, a_is_white: bool) -> GameOutcome {
    match status {
        GameStatus::Checkmate(winner) => {
            if matches!(winner, Color::White) == a_is_white {
                GameOutcome::WinA
            } else {
                GameOutcome::WinB
            }
        }
        GameStatus::Stalemate | GameStatus::Draw(_) | GameStatus::Ongoing => GameOutcome::Draw,
    }
}

const fn status_label(status: GameStatus) -> &'static str {
    match status {
        GameStatus::Checkmate(_) => "checkmate",
        GameStatus::Stalemate => "stalemate",
        GameStatus::Draw(DrawReason::InsufficientMaterial) => "insufficient_material",
        GameStatus::Draw(DrawReason::ThreefoldRepetition) => "threefold",
        GameStatus::Draw(DrawReason::FiftyMoveRule) => "fifty_move",
        GameStatus::Ongoing => "ongoing",
    }
}

pub fn summarize_reasons(records: &[GameRecord]) -> Vec<(&'static str, usize)> {
    let mut map: HashMap<&'static str, usize> = HashMap::new();
    for r in records {
        *map.entry(r.reason).or_insert(0) += 1;
    }
    let mut items: Vec<_> = map.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    items
}
