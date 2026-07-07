//! Loading oracle fixtures and replaying saves through `ad-core`.
//!
//! A fixture (produced by `oracle/generate-replay-fixtures.js`) is a JSON file
//! pairing an input save with the JS engine's expected save after each horizon:
//!
//! ```json
//! { "meta": { "sourceSave": "…", "tickMs": 50, "horizons": [1,10,100,1000] },
//!   "input": "<savefile>",
//!   "expected": { "1": "<savefile>", "10": "…", … } }
//! ```
//!
//! The Rust side loads `input` into a [`GameState`], ticks it to the same
//! horizon at the same granularity, and re-encodes it to a `player` tree — the
//! `actual` half of the diff. Both halves are `serde_json` [`Value`]s so the
//! comparator ([`crate::compare`]) can walk them by JS/save key.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ad_core::save::{decode_pipeline, decode_save, to_player_value, SaveError};
use serde::Deserialize;
use serde_json::Value;

/// `player.lastUpdate` stamped on the Rust re-encode. Time fields are on the
/// ignore list, so the exact value is immaterial — a fixed constant keeps the
/// replay deterministic and free of the wall clock.
const REPLAY_NOW_MS: i64 = 0;

/// Default tick granularity if a fixture's `meta.tickMs` is absent (design §10).
pub const DEFAULT_TICK_MS: f64 = 50.0;

/// A loaded oracle fixture.
#[derive(Clone, Debug)]
pub struct Fixture {
    /// The fixture file stem (e.g. `01_pre_big_crunch`), used as the test name.
    pub name: String,
    /// The file it was loaded from.
    pub path: PathBuf,
    /// Tick granularity the oracle used (`meta.tickMs`), which the Rust replay
    /// must match or the two engines diverge by construction.
    pub tick_ms: f64,
    /// The input save string.
    pub input: String,
    /// Horizon (tick count) → the JS engine's expected save at that horizon.
    pub expected: BTreeMap<u32, String>,
}

impl Fixture {
    /// The horizons this fixture carries, ascending.
    pub fn horizons(&self) -> impl Iterator<Item = u32> + '_ {
        self.expected.keys().copied()
    }
}

/// The on-disk fixture schema (only the fields the Rust side reads).
#[derive(Deserialize)]
struct FixtureFile {
    #[serde(default)]
    meta: Meta,
    input: String,
    expected: BTreeMap<String, String>,
}

#[derive(Deserialize, Default)]
struct Meta {
    #[serde(rename = "tickMs")]
    tick_ms: Option<f64>,
}

/// An error loading a fixture file.
#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Json(serde_json::Error),
    /// A horizon key in `expected` was not a non-negative integer.
    BadHorizon(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "{e}"),
            LoadError::Json(e) => write!(f, "invalid fixture JSON: {e}"),
            LoadError::BadHorizon(k) => write!(f, "invalid horizon key {k:?}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}
impl From<serde_json::Error> for LoadError {
    fn from(e: serde_json::Error) -> Self {
        LoadError::Json(e)
    }
}

/// Load a single fixture file.
pub fn load_fixture(path: &Path) -> Result<Fixture, LoadError> {
    let text = fs::read_to_string(path)?;
    let file: FixtureFile = serde_json::from_str(&text)?;
    let mut expected = BTreeMap::new();
    for (k, v) in file.expected {
        let h: u32 = k.parse().map_err(|_| LoadError::BadHorizon(k.clone()))?;
        expected.insert(h, v);
    }
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    Ok(Fixture {
        name,
        path: path.to_path_buf(),
        tick_ms: file.meta.tick_ms.unwrap_or(DEFAULT_TICK_MS),
        input: file.input,
        expected,
    })
}

/// Load every `*.json` fixture in `dir`, sorted by file name.
pub fn load_dir(dir: &Path) -> Result<Vec<Fixture>, LoadError> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    paths.iter().map(|p| load_fixture(p)).collect()
}

/// Decode a JS save string into its `player` tree (the `expected` side).
pub fn decode_expected(save: &str) -> Result<Value, SaveError> {
    let json = decode_pipeline(save)?;
    Ok(serde_json::from_str(&json)?)
}

/// Replay `input` through `ad-core`: decode, tick `horizon` steps of `tick_ms`,
/// and re-encode to a `player` tree (the `actual` side). `horizon == 0` yields
/// the input decoded-then-re-encoded — the round-trip identity baseline that
/// isolates an encode/decode bug from a tick bug (design §6).
pub fn replay_rust(input: &str, horizon: u32, tick_ms: f64) -> Result<Value, SaveError> {
    let mut state = decode_save(input)?;
    state.ticks(tick_ms, horizon);
    Ok(to_player_value(&state, REPLAY_NOW_MS))
}
