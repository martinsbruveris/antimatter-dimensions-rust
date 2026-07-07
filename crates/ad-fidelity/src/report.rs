//! Rendering a [`RunResult`] for the terminal: a pass/fail table (default) and a
//! per-field verbose listing.

use std::fmt::Write as _;

use crate::compare::FieldDiff;
use crate::run::{Outcome, RunResult};
use crate::trace::TraceResult;

/// The maximum number of field diffs printed per failing cell in verbose mode
/// (a single off-by-one discrete event can diverge the whole state vector —
/// design §7 — so cap the spew).
const MAX_DIFFS_PER_CELL: usize = 25;

/// Render the pass/fail grid: one row per fixture, one column per horizon.
pub fn table(result: &RunResult) -> String {
    let mut out = String::new();

    // Column widths.
    let idx_w = result.rows.len().to_string().len().max(1);
    let name_w = result
        .rows
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max("fixture".len());
    let col_w = |h: u32| header_label(h).len().max("FAIL".len());

    // Header.
    let _ = write!(out, "{:>idx_w$}  {:<name_w$}", "#", "fixture");
    for &h in &result.horizons {
        let _ = write!(out, "  {:>w$}", header_label(h), w = col_w(h));
    }
    out.push('\n');

    // Rows.
    for (i, row) in result.rows.iter().enumerate() {
        let _ = write!(out, "{:>idx_w$}  {:<name_w$}", i, row.name);
        for cell in &row.cells {
            let w = col_w(cell.horizon);
            let _ = write!(out, "  {:>w$}", cell_label(&cell.outcome));
        }
        out.push('\n');
    }

    let (passed, total) = result.tally();
    let _ = write!(out, "\n{passed}/{total} cells passed");
    if total > passed {
        let _ = write!(out, " ({} diverged)", total - passed);
    }
    out.push('\n');
    out
}

/// Render per-field detail for every failing (and errored) cell.
pub fn verbose(result: &RunResult) -> String {
    let mut out = String::new();
    for row in &result.rows {
        for cell in &row.cells {
            match &cell.outcome {
                Outcome::Fail(diffs) => {
                    let _ = writeln!(
                        out,
                        "■ {} @ {} — {} field(s) diverged",
                        row.name,
                        header_label(cell.horizon),
                        diffs.len()
                    );
                    for d in diffs.iter().take(MAX_DIFFS_PER_CELL) {
                        diff_line(&mut out, d);
                    }
                    if diffs.len() > MAX_DIFFS_PER_CELL {
                        let _ = writeln!(
                            out,
                            "    … {} more",
                            diffs.len() - MAX_DIFFS_PER_CELL
                        );
                    }
                    out.push('\n');
                }
                Outcome::Error(msg) => {
                    let _ = writeln!(
                        out,
                        "■ {} @ {} — error: {}",
                        row.name,
                        header_label(cell.horizon),
                        msg
                    );
                    out.push('\n');
                }
                Outcome::Pass | Outcome::Missing => {}
            }
        }
    }
    out
}

/// Render the first-divergence scan (`ad-fidelity trace <id>`): the earliest
/// tick that diverged and the fields that broke, or a clean-run line.
pub fn trace_scan(result: &TraceResult) -> String {
    let mut out = String::new();
    match &result.first_divergence {
        None => {
            let _ = writeln!(
                out,
                "{}: no divergence over {} tick(s)",
                result.name, result.max_horizon
            );
        }
        Some((horizon, diffs)) => {
            let _ = writeln!(
                out,
                "{}: first divergence at tick {} — {} field(s) diverged",
                result.name,
                horizon,
                diffs.len()
            );
            for d in diffs.iter().take(MAX_DIFFS_PER_CELL) {
                diff_line(&mut out, d);
            }
            if diffs.len() > MAX_DIFFS_PER_CELL {
                let _ = writeln!(out, "    … {} more", diffs.len() - MAX_DIFFS_PER_CELL);
            }
            let _ = writeln!(out, "\ninspect this tick:  … trace --tick {horizon} <id>");
        }
    }
    out
}

/// Render the field diffs at one specific tick (`ad-fidelity trace --tick X`).
pub fn trace_at(name: &str, horizon: u32, diffs: &[FieldDiff]) -> String {
    let mut out = String::new();
    if diffs.is_empty() {
        let _ = writeln!(out, "{name}: no divergence at tick {horizon}");
        return out;
    }
    let _ = writeln!(
        out,
        "{name}: {} field(s) diverged at tick {horizon}",
        diffs.len()
    );
    for d in diffs {
        diff_line(&mut out, d);
    }
    out
}

/// One field-diff line, shared by the verbose grid and the trace renderers.
fn diff_line(out: &mut String, d: &FieldDiff) {
    let _ = writeln!(
        out,
        "    {:<40}  JS={:<24}  Rust={:<24}  {}",
        d.path, d.expected, d.actual, d.detail
    );
}

/// Column header for a horizon: `rt` for the round-trip baseline, else the tick
/// count.
fn header_label(horizon: u32) -> String {
    if horizon == 0 {
        "rt".to_string()
    } else {
        horizon.to_string()
    }
}

fn cell_label(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Pass => "ok",
        Outcome::Fail(_) => "FAIL",
        Outcome::Error(_) => "err",
        Outcome::Missing => "—",
    }
}
