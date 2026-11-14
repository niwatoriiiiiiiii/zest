//! Core library for the Zest Gardner minichess engine.

/// Core primitives and types for Gardner minichess.
pub mod core;

/// Board representation and game state for Gardner minichess.
pub mod board;

/// Move generation and attack tables for Gardner minichess.
pub mod movegen;

/// Static position evaluation for Gardner minichess.
pub mod eval;

/// Game-tree search engine for Gardner minichess.
pub mod search;

/// Game state management, move history, and draw/termination rules.
pub mod game;

/// Universal Chess Interface (UCI) protocol support.
pub mod uci;

mod display;
