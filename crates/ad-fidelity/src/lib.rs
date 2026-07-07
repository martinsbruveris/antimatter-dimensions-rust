//! # ad-fidelity
//!
//! Fidelity test harness for verifying that the Rust `ad-core` implementation
//! matches the original JavaScript Antimatter Dimensions game.
//!
//! Fidelity is checked with a **save-replay** harness (design:
//! `docs/design/2026-07-06-fidelity-testing.md`): real savefiles are captured
//! from the JS game (`capture/`), replayed through the JS oracle to produce
//! reference fixtures (`oracle/`), then replayed through `ad-core` and diffed
//! against those fixtures.
//!
//! This crate provides that Rust replay/comparison side:
//!
//! - [`compare`] — the tolerant per-field diff walker and its [`Compare`] modes.
//! - [`allowlist`] — the set of `player`-tree fields that are compared (design
//!   §5).
//! - [`fixture`] — loading oracle fixtures and replaying saves through `ad-core`.
//! - [`run`] — orchestrating a (fixtures × horizons) comparison grid.
//! - [`trace`] — scanning one dense fixture for its first divergent tick.
//! - [`resolve`] — turning a short save/fixture id into a concrete file path.
//! - [`report`] — rendering the grid / trace as a table or field listing.
//! - [`tolerance`] — the underlying log-space comparison primitives.
//!
//! The `ad-fidelity` binary ([`main`](../main/index.html)) wires these into a CLI.

pub mod allowlist;
pub mod compare;
pub mod fixture;
pub mod report;
pub mod resolve;
pub mod run;
pub mod tolerance;
pub mod trace;

pub use allowlist::allowlist;
pub use compare::{compare_trees, Compare, FieldDiff, FieldRule, Tolerance};
pub use fixture::{
    decode_expected, load_dir, load_fixture, replay_rust, Fixture, LoadError,
};
pub use resolve::{resolve, ResolveError};
pub use run::{run, CellResult, FixtureResult, Outcome, RunConfig, RunResult};
pub use trace::{compare_at, trace, TraceResult};
