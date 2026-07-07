//! Rendering a [`RunResult`] for the terminal: a pass/fail table (default) and a
//! per-field verbose listing.

use std::fmt::Write as _;

use crate::run::{Outcome, RunResult};

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
                        let _ = writeln!(
                            out,
                            "    {:<40}  JS={:<24}  Rust={:<24}  {}",
                            d.path, d.expected, d.actual, d.detail
                        );
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
