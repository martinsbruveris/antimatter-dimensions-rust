//! # ad-fidelity
//!
//! Fidelity test harness for verifying that the Rust `ad-core`
//! implementation matches the original JavaScript Antimatter
//! Dimensions game.
//!
//! This crate provides:
//! - Tolerance-based comparison utilities for Decimal values
//! - Pre-computed reference values from the JS game
//! - Test scenarios covering the pre-infinity phase

pub mod tolerance;
