//! # ad-fidelity
//!
//! Fidelity test harness for verifying that the Rust `ad-core`
//! implementation matches the original JavaScript Antimatter
//! Dimensions game.
//!
//! Fidelity is checked with a save-replay harness: real savefiles are
//! captured from the JS game (`capture/`), replayed through the JS oracle
//! to produce reference fixtures (`oracle/`), and — once built — replayed
//! through `ad-core` and diffed against those fixtures.
//!
//! This crate currently provides the tolerance-based comparison utilities
//! ([`tolerance`]) used by that diff.

pub mod tolerance;
