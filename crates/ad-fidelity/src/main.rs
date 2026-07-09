//! `ad-fidelity` — replay captured saves through `ad-core` and diff them against
//! the JS oracle fixtures.
//!
//! ```text
//! ad-fidelity [DIR] [--tests 1,3,12] [--ticks 1,10] [--tick-ms 50]
//!             [--epsilon 1e-4] [--roundtrip] [--verbose]
//! ad-fidelity trace <ID> [--tick X] [--epsilon 1e-4] [--tick-ms 50]
//! ```
//!
//! Default (no subcommand): compare every fixture in `DIR` at every horizon it
//! carries, printing a pass/fail table (rows = fixtures, columns = tick counts).
//! `--verbose` adds, per failing cell, the fields that diverged with their JS /
//! Rust values and the delta.
//!
//! `trace` takes one *dense* fixture (the oracle's `--trace` output, every tick
//! up to 1000) and reports the earliest tick that diverges; `--tick X` then dumps
//! the full field diff at that tick. `<ID>` resolves under `saves/traces` by the
//! shared id convention (see [`ad_fidelity::resolve`]).
//!
//! Note: pass CLI args after `--` so cargo forwards them, e.g.
//! `cargo run -p ad-fidelity -- trace 1`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use ad_fidelity::allowlist::allowlist;
use ad_fidelity::compare::Tolerance;
use ad_fidelity::fixture::{load_dir, load_fixture, Fixture};
use ad_fidelity::report::{table, trace_at, trace_scan, verbose};
use ad_fidelity::resolve::resolve;
use ad_fidelity::run::{run, RunConfig};
use ad_fidelity::trace::{compare_at, trace};

/// The oracle's default fixture output directory (`saves/fixtures`, relative to
/// this crate).
const DEFAULT_FIXTURES_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/saves/fixtures");

/// Where dense trace fixtures live (`saves/traces`), shared with the oracle's
/// `--trace` output. `trace <ID>` resolves ids here.
const DEFAULT_TRACES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/saves/traces");

#[derive(Parser, Debug)]
#[command(
    name = "ad-fidelity",
    about = "Replay saves through ad-core and diff against the JS oracle fixtures",
    long_about = None,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true,
)]
struct Cli {
    /// A subcommand (currently only `trace`); omit it for the grid comparison.
    #[command(subcommand)]
    command: Option<Command>,

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
    #[arg(long, default_value_t = 1e-4)]
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

#[derive(Subcommand, Debug)]
enum Command {
    /// Scan one dense trace fixture for its first divergent tick.
    Trace(TraceArgs),
}

#[derive(Args, Debug)]
struct TraceArgs {
    /// Trace fixture id (resolved under `saves/traces`) or a path.
    id: String,

    /// Inspect exactly this tick — dump the full field diff instead of scanning
    /// for the first divergence.
    #[arg(long)]
    tick: Option<u32>,

    /// Log-space (and relative) comparison epsilon.
    #[arg(long, default_value_t = 1e-4)]
    epsilon: f64,

    /// Override the fixture's `meta.tickMs`. Must match the oracle.
    #[arg(long)]
    tick_ms: Option<f64>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(Command::Trace(args)) = &cli.command {
        return run_trace(args);
    }

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

/// The `trace` subcommand: resolve the fixture, then either scan for the first
/// divergent tick or (with `--tick X`) dump the field diff at exactly tick `X`.
///
/// Exit codes mirror grid mode: `1` on divergence, `2` on a resolution/load
/// error, `0` when clean.
fn run_trace(args: &TraceArgs) -> ExitCode {
    let path = match resolve(&args.id, Path::new(DEFAULT_TRACES_DIR), "json") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let fixture = match load_fixture(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("could not load trace fixture {}: {e}", path.display());
            return ExitCode::from(2);
        }
    };

    let tolerance = Tolerance::with_log_eps(args.epsilon);
    let rules = allowlist();
    let tick_ms = args.tick_ms.unwrap_or(fixture.tick_ms);

    match args.tick {
        Some(tick) => {
            let diffs = match compare_at(&fixture, tick, tick_ms, &tolerance, &rules) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            print!("{}", trace_at(&fixture.name, tick, &diffs));
            if diffs.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        None => {
            let result = match trace(&fixture, tick_ms, &tolerance, &rules) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            print!("{}", trace_scan(&result));
            if result.first_divergence.is_some() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
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
        if i >= all.len() {
            return Err(format!(
                "--tests index {i} out of range (have {} fixtures, 0..{})",
                all.len(),
                all.len() - 1
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
