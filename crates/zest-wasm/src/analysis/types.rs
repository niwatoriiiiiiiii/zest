use zest::core::{Color, Move};

/// Move classification tags tailored for 5x5 Gardner minichess.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveTag {
    Book,
    Forced,
    Blunder,
    Mistake,
    Inaccuracy,
    Brilliant,
    Critical,
    Best,
    Good,
}

impl MoveTag {
    /// Returns the display name of the classification tag.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Book => "Book",
            Self::Forced => "Forced",
            Self::Blunder => "Blunder",
            Self::Mistake => "Mistake",
            Self::Inaccuracy => "Inaccuracy",
            Self::Brilliant => "Brilliant",
            Self::Critical => "Critical",
            Self::Best => "Best",
            Self::Good => "Good",
        }
    }

    /// Returns the glyph symbol for UI annotation.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Book => "📖",
            Self::Forced => "□",
            Self::Blunder => "??",
            Self::Mistake => "?",
            Self::Inaccuracy => "?!",
            Self::Brilliant => "!!",
            Self::Critical => "!",
            Self::Best => "✦",
            Self::Good => "○",
        }
    }

    /// Returns the standard PGN Numeric Annotation Glyph (NAG), if defined.
    #[must_use]
    pub const fn nag(self) -> Option<u8> {
        match self {
            Self::Critical => Some(1),
            Self::Mistake => Some(2),
            Self::Brilliant => Some(3),
            Self::Blunder => Some(4),
            Self::Inaccuracy => Some(6),
            Self::Forced => Some(7),
            Self::Book | Self::Best | Self::Good => None,
        }
    }

    /// Returns `true` if this move should be excluded from Accuracy and ACPL aggregation.
    #[must_use]
    pub const fn is_excluded_from_stats(self) -> bool {
        matches!(self, Self::Book | Self::Forced)
    }
}

/// Evaluation summary for a single candidate PV line.
#[derive(Clone, Copy, Debug)]
pub struct PvCandidate {
    pub mv: Move,
    pub raw_score: i32,
    pub cp: i32,
    pub expected: f64,
}

/// Classification and metrics for a single played move.
#[derive(Clone, Debug)]
pub struct PlyAnalysis {
    pub ply: usize,
    pub side: Color,
    pub uci: String,
    pub score: i32,
    pub is_mate: bool,
    pub mate_in: Option<i32>,
    pub expected: f64,
    pub d_exp: f64,
    pub cp_loss: i32,
    pub own_safe_count: usize,
    pub own_legal_count: usize,
    pub opp_safe_count: usize,
    pub opp_legal_count: usize,
    pub tag: MoveTag,
}

/// Aggregated accuracy and classification counts for one side.
#[derive(Clone, Debug)]
pub struct SideSummary {
    pub accuracy: f64,
    pub acpl: f64,
    pub rms_d_exp: f64,
    pub counted_moves: usize,
    pub book: usize,
    pub forced: usize,
    pub brilliant: usize,
    pub critical: usize,
    pub best: usize,
    pub good: usize,
    pub inaccuracy: usize,
    pub mistake: usize,
    pub blunder: usize,
}

/// Full post-game analysis result for both sides.
#[derive(Clone, Debug)]
pub struct GameAnalysis {
    pub plies: Vec<PlyAnalysis>,
    pub white: SideSummary,
    pub black: SideSummary,
}
