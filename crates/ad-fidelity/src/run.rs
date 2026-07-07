//! Orchestration: replay each fixture at each horizon and classify the result.
//!
//! This ties [`crate::fixture`] (load + replay) to [`crate::compare`] (diff),
//! producing a grid of [`Outcome`]s — one per (fixture, horizon) — that
//! [`crate::report`] renders as a table or verbose field listing.

use crate::compare::{compare_trees, FieldDiff, FieldRule, Tolerance};
use crate::fixture::{decode_expected, replay_rust, Fixture};

/// How a single (fixture, horizon) comparison turned out.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// The engines agreed over the allowlist.
    Pass,
    /// They diverged on these fields.
    Fail(Vec<FieldDiff>),
    /// The comparison could not run (a save failed to decode, or the Rust
    /// replay errored). Holds a human-readable reason.
    Error(String),
    /// The fixture carries no expected save at this horizon.
    Missing,
}

impl Outcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, Outcome::Pass)
    }
    /// A cell counts against the run unless it passed or was simply absent.
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Fail(_) | Outcome::Error(_))
    }
}

/// One cell of the result grid.
#[derive(Clone, Debug)]
pub struct CellResult {
    pub horizon: u32,
    pub outcome: Outcome,
}

/// A fixture's row of the result grid.
#[derive(Clone, Debug)]
pub struct FixtureResult {
    pub name: String,
    pub cells: Vec<CellResult>,
}

/// Knobs for a comparison run.
#[derive(Clone, Debug, Default)]
pub struct RunConfig {
    /// Override the fixture's `meta.tickMs`. `None` uses each fixture's own.
    pub tick_ms: Option<f64>,
    /// Comparison tolerance.
    pub tolerance: Tolerance,
}

/// The result grid plus the horizon columns it was computed over.
#[derive(Clone, Debug)]
pub struct RunResult {
    /// The horizon columns, in the order the rows report them. Horizon `0` (if
    /// present) is the round-trip identity baseline.
    pub horizons: Vec<u32>,
    pub rows: Vec<FixtureResult>,
}

impl RunResult {
    /// `(passed, total)` over cells that actually ran (excludes `Missing`).
    pub fn tally(&self) -> (usize, usize) {
        let mut passed = 0;
        let mut total = 0;
        for row in &self.rows {
            for cell in &row.cells {
                match cell.outcome {
                    Outcome::Missing => {}
                    Outcome::Pass => {
                        passed += 1;
                        total += 1;
                    }
                    _ => total += 1,
                }
            }
        }
        (passed, total)
    }

    /// Whether any cell failed or errored (drives the process exit code).
    pub fn any_failure(&self) -> bool {
        self.rows
            .iter()
            .any(|r| r.cells.iter().any(|c| c.outcome.is_failure()))
    }
}

/// Compare `fixtures` over `horizons` under `config` and `rules`.
///
/// `horizons` is the shared column set; a fixture lacking one yields a `Missing`
/// cell so every row lines up. Horizon `0` runs the round-trip identity check
/// (design §6) against the fixture's own input.
pub fn run(
    fixtures: &[Fixture],
    horizons: &[u32],
    config: &RunConfig,
    rules: &[FieldRule],
) -> RunResult {
    let rows = fixtures
        .iter()
        .map(|f| run_fixture(f, horizons, config, rules))
        .collect();
    RunResult {
        horizons: horizons.to_vec(),
        rows,
    }
}

fn run_fixture(
    fixture: &Fixture,
    horizons: &[u32],
    config: &RunConfig,
    rules: &[FieldRule],
) -> FixtureResult {
    let tick_ms = config.tick_ms.unwrap_or(fixture.tick_ms);
    let cells = horizons
        .iter()
        .map(|&h| CellResult {
            horizon: h,
            outcome: run_cell(fixture, h, tick_ms, config, rules),
        })
        .collect();
    FixtureResult {
        name: fixture.name.clone(),
        cells,
    }
}

fn run_cell(
    fixture: &Fixture,
    horizon: u32,
    tick_ms: f64,
    config: &RunConfig,
    rules: &[FieldRule],
) -> Outcome {
    // Horizon 0 is the round-trip guard: compare the fixture's input as the JS
    // sees it against Rust's decode→re-encode of that same input.
    let expected_save = if horizon == 0 {
        &fixture.input
    } else {
        match fixture.expected.get(&horizon) {
            Some(s) => s,
            None => return Outcome::Missing,
        }
    };

    let expected = match decode_expected(expected_save) {
        Ok(v) => v,
        Err(e) => return Outcome::Error(format!("decode expected: {e}")),
    };
    let actual = match replay_rust(&fixture.input, horizon, tick_ms) {
        Ok(v) => v,
        Err(e) => return Outcome::Error(format!("replay: {e}")),
    };

    let diffs = compare_trees(&expected, &actual, rules, &config.tolerance, horizon);
    if diffs.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(diffs)
    }
}
