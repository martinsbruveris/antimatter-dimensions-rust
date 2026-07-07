//! `ad-fidelity` — replay captured saves through `ad-core` and diff them against
//! the JS oracle fixtures.
//!
//! ```text
//! ad-fidelity [DIR] [--tests 1,3,12] [--ticks 1,10] [--tick-ms 50]
//!             [--epsilon 1e-6] [--roundtrip] [--verbose]
//! ```
//!
//! Default (no flags): compare every fixture in `DIR` at every horizon it
//! carries, printing a pass/fail table (rows = fixtures, columns = tick counts).
//! `--verbose` adds, per failing cell, the fields that diverged with their JS /
//! Rust values and the delta.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use ad_fidelity::allowlist::allowlist;
use ad_fidelity::compare::Tolerance;
use ad_fidelity::fixture::{load_dir, Fixture};
use ad_fidelity::report::{table, verbose};
use ad_fidelity::run::{run, RunConfig};

/// The oracle's default fixture output directory (`oracle/fixtures`, relative to
/// this crate).
const DEFAULT_FIXTURES_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/oracle/fixtures");

#[derive(Parser, Debug)]
#[command(
    name = "ad-fidelity",
    about = "Replay saves through ad-core and diff against the JS oracle fixtures",
    long_about = None,
)]
struct Cli {
    /// Directory of oracle fixture `*.json` files.
    #[arg(default_value = DEFAULT_FIXTURES_DIR)]
    dir: PathBuf,

    /// Only these fixtures, by 0-based row index (as shown in the table),
    /// comma-separated: --tests 1,3,12
    #[arg(long, value_delimiter = ',')]
    tests: Option<Vec<usize>>,

    /// Only these horizons (tick counts), comma-separated: --ticks 1,10.
    /// Defaults to every horizon present across the selected fixtures.
    #[arg(long, value_delimiter = ',')]
    ticks: Option<Vec<u32>>,

    /// Override the tick granularity (ms). Defaults to each fixture's
    /// `meta.tickMs`. Must match the oracle or the engines diverge by design.
    #[arg(long)]
    tick_ms: Option<f64>,

    /// Log-space (and relative) comparison epsilon.
    #[arg(long, default_value_t = 1e-6)]
    epsilon: f64,

    /// Also run the round-trip identity check (a `rt` column: Rust
    /// decode→encode of the input vs the input itself). Isolates encode/decode
    /// bugs from tick bugs.
    #[arg(long)]
    roundtrip: bool,

    /// Show, per failing cell, the fields that diverged and their deltas.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let all = match load_dir(&cli.dir) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("could not read fixtures from {}: {e}", cli.dir.display());
            eprintln!(
                "hint: generate fixtures first — see crates/ad-fidelity/oracle/README.md"
            );
            return ExitCode::from(2);
        }
    };
    if all.is_empty() {
        eprintln!("no fixtures (*.json) found in {}", cli.dir.display());
        return ExitCode::from(2);
    }

    let fixtures = match select_fixtures(all, cli.tests.as_deref()) {
        Ok(f) => f,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };

    let horizons = select_horizons(&fixtures, cli.ticks.as_deref(), cli.roundtrip);
    if horizons.is_empty() {
        eprintln!(
            "no horizons to compare (none of --ticks are present in the fixtures)"
        );
        return ExitCode::from(2);
    }

    let config = RunConfig {
        tick_ms: cli.tick_ms,
        tolerance: Tolerance::with_log_eps(cli.epsilon),
    };
    let rules = allowlist();
    let result = run(&fixtures, &horizons, &config, &rules);

    if cli.verbose {
        let detail = verbose(&result);
        if !detail.is_empty() {
            print!("{detail}");
        }
    }
    print!("{}", table(&result));

    if result.any_failure() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Keep only the fixtures at the given 0-based indices (or all, if `None`).
fn select_fixtures(
    all: Vec<Fixture>,
    tests: Option<&[usize]>,
) -> Result<Vec<Fixture>, String> {
    let Some(indices) = tests else {
        return Ok(all);
    };
    let mut selected = Vec::new();
    for &i in indices {
        if i == 0 || i > all.len() {
            return Err(format!(
                "--tests index {i} out of range (have {} fixtures)",
                all.len()
            ));
        }
        selected.push(all[i].clone());
    }
    Ok(selected)
}

/// The horizon columns: the requested `--ticks` (intersected with what the
/// fixtures carry) or, by default, every horizon present. Horizon `0` (the
/// round-trip baseline) is prepended when `--roundtrip` is set.
fn select_horizons(
    fixtures: &[Fixture],
    ticks: Option<&[u32]>,
    roundtrip: bool,
) -> Vec<u32> {
    let present: BTreeSet<u32> = fixtures.iter().flat_map(Fixture::horizons).collect();
    let mut horizons: Vec<u32> = match ticks {
        // 0 is the synthetic round-trip baseline — always available, never in
        // the fixtures' own horizon set.
        Some(req) => req
            .iter()
            .copied()
            .filter(|h| *h == 0 || present.contains(h))
            .collect(),
        None => present.into_iter().collect(),
    };
    horizons.sort_unstable();
    horizons.dedup();
    if roundtrip && !horizons.contains(&0) {
        horizons.insert(0, 0);
    }
    horizons
}
