//! `ad-format` — number formatting (notations) for Antimatter Dimensions.
//!
//! A pure, presentation-only reproduction of the game's number formatting. The
//! single entry point is [`format`]; the notation choice and digit/comma settings
//! travel in [`FormatOptions`]. Nothing here reads `GameState` — see
//! `design-docs/2026-06-25-number-formatting.md`.
//!
//! The original game supports ~20 notations; we implement a subset (Scientific,
//! Engineering, Standard, Letters). The routing is general, so additional notations
//! can be added in future.

mod exponent;
mod mantissa;
mod notations;
mod options;
mod router;
#[cfg(feature = "wasm")]
mod wasm;

pub use options::{ExponentDisplay, FormatOptions, Notation};
pub use router::format;
