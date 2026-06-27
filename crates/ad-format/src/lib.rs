//! `ad-format` — number formatting (notations) for Antimatter Dimensions.
//!
//! A pure, presentation-only port of the game's `format()` and the
//! `@antimatter-dimensions/notations` strategies. The single entry point is
//! [`format`]; the notation choice and digit/comma settings travel in
//! [`FormatOptions`]. Nothing here reads `GameState` — see
//! `design-docs/2026-06-25-number-formatting.md`.
//!
//! Milestone 1 implements the notation-independent routing ([`router`]) and the
//! first four notation strategies (Scientific, Engineering, Standard, Letters);
//! the remaining ~20 notations are ported in subsequent milestones.

mod exponent;
mod mantissa;
mod notations;
mod options;
mod router;

pub use options::{ExponentCommas, FormatOptions, Notation};
pub use router::format;
