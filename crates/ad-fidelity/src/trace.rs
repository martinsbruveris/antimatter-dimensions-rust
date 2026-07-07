//! First-divergence scan of a dense fixture (the `trace` subcommand).
//!
//! Where [`crate::run`] compares a grid of fixtures at a handful of horizons,
//! `trace` takes a *single* fixture carrying every tick (produced by the oracle's
//! `--trace` mode) and walks its horizons in ascending order to find the earliest
//! tick at which the Rust replay diverges from the JS oracle. That earliest tick,
//! plus the fields that broke, is the starting point for a debugging session:
//! fix the first divergence, regenerate the trace, repeat.
//!
//! Divergence means exactly what it means for the grid — the same allowlist and
//! the same [`Tolerance`] — so a tick that passes here would pass a grid cell at
//! the same horizon.

use crate::compare::{compare_trees, FieldDiff, FieldRule, Tolerance};
use crate::fixture::{decode_expected, replay_rust, Fixture};

/// The outcome of scanning a fixture for its first divergent tick.
#[derive(Clone, Debug)]
pub struct TraceResult {
    /// The fixture's name (file stem).
    pub name: String,
    /// The largest horizon scanned — reported when nothing diverges.
    pub max_horizon: u32,
    /// The first divergent horizon and the fields that diverged there, or `None`
    /// if the two engines agreed at every horizon the fixture carries.
    pub first_divergence: Option<(u32, Vec<FieldDiff>)>,
}

/// Compare Rust's replay against the fixture's expected save at `horizon`.
///
/// Returns the field diffs (empty means agreement). Errs if the fixture carries
/// no expected save at `horizon`, or if decoding/replay fails.
pub fn compare_at(
    fixture: &Fixture,
    horizon: u32,
    tick_ms: f64,
    tol: &Tolerance,
    rules: &[FieldRule],
) -> Result<Vec<FieldDiff>, String> {
    let expected_save = fixture
        .expected
        .get(&horizon)
        .ok_or_else(|| format!("fixture has no expected save at tick {horizon}"))?;
    let expected = decode_expected(expected_save)
        .map_err(|e| format!("decode expected @ tick {horizon}: {e}"))?;
    let actual = replay_rust(&fixture.input, horizon, tick_ms)
        .map_err(|e| format!("replay @ tick {horizon}: {e}"))?;
    Ok(compare_trees(&expected, &actual, rules, tol, horizon))
}

/// Scan the fixture's horizons in ascending order, stopping at the first that
/// diverges.
///
/// This replays from scratch at each horizon (via [`compare_at`]) — O(n²) in the
/// number of ticks. That is deliberately naive for now; the single-pass
/// optimization (replay once, snapshot per tick) is a later step.
pub fn trace(
    fixture: &Fixture,
    tick_ms: f64,
    tol: &Tolerance,
    rules: &[FieldRule],
) -> Result<TraceResult, String> {
    let mut horizons: Vec<u32> = fixture.horizons().collect();
    horizons.sort_unstable();
    let max_horizon = horizons.last().copied().unwrap_or(0);

    for h in horizons {
        let diffs = compare_at(fixture, h, tick_ms, tol, rules)?;
        if !diffs.is_empty() {
            return Ok(TraceResult {
                name: fixture.name.clone(),
                max_horizon,
                first_divergence: Some((h, diffs)),
            });
        }
    }

    Ok(TraceResult {
        name: fixture.name.clone(),
        max_horizon,
        first_divergence: None,
    })
}
