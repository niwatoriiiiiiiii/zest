//! Core library for the Zest Gardner minichess engine.

/// Core primitives and types for Gardner minichess.
pub mod core;

/// Board representation and game state for Gardner minichess.
pub mod board;

/// Move generation and attack tables for Gardner minichess.
pub mod movegen;

/// Static position evaluation for Gardner minichess.
pub mod eval;

/// Game state management, move history, and draw/termination rules.
pub mod game;

mod display;
