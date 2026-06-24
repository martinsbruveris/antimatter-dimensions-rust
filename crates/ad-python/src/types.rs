use pyo3::prelude::*;

use ad_core::observed::{ObservedDimensionTier, ObservedState, ObservedTickspeedState};
use ad_core::simulator::{
    SimulationConfig, SimulationResult, StopCondition, StopReason,
};
use ad_core::strategy::{
    BuyPriority, DimensionOrder, PrestigeMode, PrestigeStep, PurchaseConfig,
    SacrificeConfig, StrategyConfig,
};
use break_infinity::Decimal;

// ============================================================
// Decimal types
// ============================================================

/// A single Decimal value exposed as mantissa + exponent.
///
/// Represents a number as m × 10^e where m is in [1, 10) or 0.
#[pyclass(name = "Decimal")]
#[derive(Debug, Clone)]
pub struct PyDecimal {
    /// Mantissa, normalized to [1, 10) or 0.
    #[pyo3(get)]
    pub m: f64,
    /// Integer exponent.
    #[pyo3(get)]
    pub e: i64,
}

#[pymethods]
impl PyDecimal {
    fn __repr__(&self) -> String {
        if self.m == 0.0 {
            "Decimal(0)".to_string()
        } else {
            format!("Decimal({}e{})", self.m, self.e)
        }
    }
}

impl PyDecimal {
    fn from_decimal(d: &Decimal) -> Self {
        Self {
            m: d.mantissa(),
            e: d.exponent(),
        }
    }

    pub fn from_big_crunch_threshold() -> Self {
        let d = ad_core::data::constants::big_crunch_threshold();
        Self::from_decimal(&d)
    }
}

/// A batch of Decimal values stored as parallel arrays.
///
/// Stores mantissas and exponents separately for efficient
/// numpy conversion on the Python side.
#[pyclass(name = "DecimalArray")]
#[derive(Debug, Clone)]
pub struct PyDecimalArray {
    /// Mantissas (each in [1, 10) or 0).
    #[pyo3(get)]
    pub m: Vec<f64>,
    /// Integer exponents.
    #[pyo3(get)]
    pub e: Vec<i64>,
}

#[pymethods]
impl PyDecimalArray {
    fn __len__(&self) -> usize {
        self.m.len()
    }

    fn __repr__(&self) -> String {
        format!("DecimalArray(len={})", self.m.len())
    }
}

impl PyDecimalArray {
    #[allow(dead_code)]
    fn from_decimals(decimals: &[Decimal]) -> Self {
        Self {
            m: decimals.iter().map(|d| d.mantissa()).collect(),
            e: decimals.iter().map(|d| d.exponent()).collect(),
        }
    }
}

// ============================================================
// Game state types
// ============================================================

/// A single antimatter dimension tier with computed fields.
#[pyclass(name = "DimensionTier")]
#[derive(Debug, Clone)]
pub struct PyDimensionTier {
    /// Current amount (can be fractional due to production).
    #[pyo3(get)]
    pub amount: PyDecimal,
    /// Number of individual purchases made.
    #[pyo3(get)]
    pub bought: u64,
    /// Current production multiplier for this tier.
    #[pyo3(get)]
    pub multiplier: PyDecimal,
    /// Production rate per second for this tier.
    #[pyo3(get)]
    pub production_per_second: PyDecimal,
}

impl PyDimensionTier {
    fn from_core(tier: &ObservedDimensionTier) -> Self {
        Self {
            amount: PyDecimal::from_decimal(&tier.amount),
            bought: tier.bought,
            multiplier: PyDecimal::from_decimal(&tier.multiplier),
            production_per_second: PyDecimal::from_decimal(&tier.production_per_second),
        }
    }
}

/// Tickspeed state with computed fields.
#[pyclass(name = "TickspeedState")]
#[derive(Debug, Clone)]
pub struct PyTickspeedState {
    /// Number of tickspeed upgrades purchased.
    #[pyo3(get)]
    pub bought: u64,
    /// Current cost to buy the next tickspeed upgrade.
    #[pyo3(get)]
    pub cost: PyDecimal,
    /// Cost multiplier per purchase.
    #[pyo3(get)]
    pub cost_multiplier: PyDecimal,
    /// Current tickspeed interval in milliseconds.
    #[pyo3(get)]
    pub tickspeed_ms: f64,
    /// Production multiplier from tickspeed.
    #[pyo3(get)]
    pub tickspeed_effect: PyDecimal,
}

impl PyTickspeedState {
    fn from_core(ts: &ObservedTickspeedState) -> Self {
        Self {
            bought: ts.bought,
            cost: PyDecimal::from_decimal(&ts.cost),
            cost_multiplier: PyDecimal::from_decimal(&ts.cost_multiplier),
            tickspeed_ms: ts.tickspeed_ms,
            tickspeed_effect: PyDecimal::from_decimal(&ts.tickspeed_effect),
        }
    }
}

/// Observed game state with computed fields.
///
/// Contains all game state fields plus materialised computed
/// values for analysis.
#[pyclass(name = "GameState")]
#[derive(Debug, Clone)]
pub struct PyGameState {
    /// Current antimatter amount.
    #[pyo3(get)]
    pub antimatter: PyDecimal,
    /// All 8 antimatter dimension tiers.
    #[pyo3(get)]
    pub dimensions: Vec<PyDimensionTier>,
    /// Tickspeed state with computed fields.
    #[pyo3(get)]
    pub tickspeed: PyTickspeedState,
    /// Number of dimension boosts performed.
    #[pyo3(get)]
    pub dim_boosts: u32,
    /// Number of antimatter galaxies purchased.
    #[pyo3(get)]
    pub galaxies: u32,
    /// Total antimatter sacrificed (cumulative).
    #[pyo3(get)]
    pub sacrificed: PyDecimal,
    /// Running product of all sacrifice boosts (applied to 8th
    /// dimension).
    #[pyo3(get)]
    pub sacrifice_boost: PyDecimal,
    /// Whether sacrifice is unlocked.
    #[pyo3(get)]
    pub sacrifice_unlocked: bool,
}

impl PyGameState {
    pub fn from_core(state: &ObservedState) -> Self {
        Self {
            antimatter: PyDecimal::from_decimal(&state.antimatter),
            dimensions: state
                .dimensions
                .iter()
                .map(PyDimensionTier::from_core)
                .collect(),
            tickspeed: PyTickspeedState::from_core(&state.tickspeed),
            dim_boosts: state.dim_boosts,
            galaxies: state.galaxies,
            sacrificed: PyDecimal::from_decimal(&state.sacrificed),
            sacrifice_boost: PyDecimal::from_decimal(&state.sacrifice_boost),
            sacrifice_unlocked: state.sacrifice_unlocked,
        }
    }
}

// ============================================================
// StrategyConfig
// ============================================================

/// Strategy configuration for the simulation.
///
/// Args:
///     sacrifice_enabled: Whether to auto-sacrifice.
///     sacrifice_threshold: Min gain ratio for sacrifice (e.g. 10.0).
///     tickspeed_weight: Weight for tickspeed vs dimension cost comparison.
///         >1 prefers tickspeed, <1 prefers dimensions, 1 is equal.
///     dimension_order: "highest_first", "lowest_first", or "cheapest_first".
///     prestige_mode: "auto" or a list of prestige steps like
///         ["boost:4", "galaxy", "boost:3", "galaxy"].
#[pyclass(name = "StrategyConfig")]
#[derive(Debug, Clone)]
pub struct PyStrategyConfig {
    #[pyo3(get, set)]
    pub sacrifice_enabled: bool,
    #[pyo3(get, set)]
    pub sacrifice_threshold: f64,
    #[pyo3(get, set)]
    pub tickspeed_weight: f64,
    #[pyo3(get, set)]
    pub dimension_order: String,
    /// Stored as parsed core type.
    prestige: PrestigeMode,
}

#[pymethods]
impl PyStrategyConfig {
    #[new]
    #[pyo3(signature = (
        sacrifice_enabled = true,
        sacrifice_threshold = 10.0,
        tickspeed_weight = 1.0,
        dimension_order = "highest_first".to_string(),
        prestige_mode = None,
    ))]
    fn new(
        sacrifice_enabled: bool,
        sacrifice_threshold: f64,
        tickspeed_weight: f64,
        dimension_order: String,
        prestige_mode: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let prestige = match prestige_mode {
            Some(obj) => parse_prestige_mode(obj)?,
            None => PrestigeMode::Auto,
        };
        Ok(Self {
            sacrifice_enabled,
            sacrifice_threshold,
            tickspeed_weight,
            dimension_order,
            prestige,
        })
    }

    /// Get prestige_mode as a Python-friendly value.
    #[getter]
    fn prestige_mode(&self, py: Python<'_>) -> PyObject {
        match &self.prestige {
            PrestigeMode::Auto => "auto".into_pyobject(py).unwrap().into_any().unbind(),
            PrestigeMode::Plan(steps) => {
                let strings: Vec<String> = steps
                    .iter()
                    .map(|s| match s {
                        PrestigeStep::DimBoost(n) => format!("boost:{}", n),
                        PrestigeStep::Galaxy => "galaxy".to_string(),
                    })
                    .collect();
                strings.into_pyobject(py).unwrap().into_any().unbind()
            }
        }
    }
}

impl PyStrategyConfig {
    pub fn to_core(&self) -> StrategyConfig {
        let dimension_order = match self.dimension_order.as_str() {
            "lowest_first" => DimensionOrder::LowestFirst,
            "cheapest_first" => DimensionOrder::CheapestFirst,
            _ => DimensionOrder::HighestFirst,
        };

        StrategyConfig {
            sacrifice: SacrificeConfig {
                enabled: self.sacrifice_enabled,
                min_gain_ratio: self.sacrifice_threshold,
            },
            purchase: PurchaseConfig {
                priority: BuyPriority::Weighted {
                    tickspeed_weight: self.tickspeed_weight,
                },
                dimension_order,
            },
            prestige: self.prestige.clone(),
        }
    }
}

/// Parse prestige_mode from Python: either "auto" or a list of step strings.
fn parse_prestige_mode(obj: &Bound<'_, PyAny>) -> PyResult<PrestigeMode> {
    // Try as string first
    if let Ok(s) = obj.extract::<String>() {
        if s == "auto" {
            return Ok(PrestigeMode::Auto);
        }
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown prestige mode: '{}'. Use 'auto' or a list of steps.",
            s
        )));
    }

    // Try as list of strings
    if let Ok(steps) = obj.extract::<Vec<String>>() {
        let parsed: Result<Vec<PrestigeStep>, _> =
            steps.iter().map(|s| parse_prestige_step(s)).collect();
        return Ok(PrestigeMode::Plan(parsed?));
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "prestige_mode must be 'auto' or a list of step strings",
    ))
}

/// Parse a single prestige step string like "boost:4" or "galaxy".
fn parse_prestige_step(s: &str) -> PyResult<PrestigeStep> {
    if s == "galaxy" {
        return Ok(PrestigeStep::Galaxy);
    }
    if let Some(n_str) = s.strip_prefix("boost:") {
        if let Ok(n) = n_str.parse::<u32>() {
            return Ok(PrestigeStep::DimBoost(n));
        }
    }
    Err(pyo3::exceptions::PyValueError::new_err(format!(
        "Invalid prestige step: '{}'. Use 'galaxy' or 'boost:N'.",
        s
    )))
}

// ============================================================
// SimulationConfig
// ============================================================

/// Configuration for a simulation run.
///
/// Args:
///     strategy: Strategy configuration.
///     tick_ms: Time step in milliseconds (default 50.0).
///     snapshot_count: Approximate number of trace snapshots
///         (0 to disable). Actual count is between this and 2x.
///     stop_score: Stop when antimatter reaches this Decimal
///         value. None = Big Crunch default.
///     stop_max_ticks: Stop after this many ticks. None = no
///         limit.
///     stop_max_game_time_s: Stop after this much game time in
///         seconds. None = no limit.
///     stop_max_wall_time_s: Stop after this much wall-clock
///         time in seconds. None = no limit.
#[pyclass(name = "SimulationConfig")]
#[derive(Debug, Clone)]
pub struct PySimulationConfig {
    #[pyo3(get, set)]
    pub strategy: PyStrategyConfig,
    #[pyo3(get, set)]
    pub tick_ms: f64,
    #[pyo3(get, set)]
    pub snapshot_count: usize,
    #[pyo3(get, set)]
    pub stop_score: Option<PyDecimal>,
    #[pyo3(get, set)]
    pub stop_max_ticks: Option<u64>,
    #[pyo3(get, set)]
    pub stop_max_game_time_s: Option<f64>,
    #[pyo3(get, set)]
    pub stop_max_wall_time_s: Option<f64>,
}

#[pymethods]
impl PySimulationConfig {
    #[new]
    #[pyo3(signature = (
        strategy,
        tick_ms = 50.0,
        snapshot_count = 500,
        stop_score = None,
        stop_max_ticks = None,
        stop_max_game_time_s = None,
        stop_max_wall_time_s = None,
    ))]
    fn new(
        strategy: PyStrategyConfig,
        tick_ms: f64,
        snapshot_count: usize,
        stop_score: Option<PyDecimal>,
        stop_max_ticks: Option<u64>,
        stop_max_game_time_s: Option<f64>,
        stop_max_wall_time_s: Option<f64>,
    ) -> Self {
        Self {
            strategy,
            tick_ms,
            snapshot_count,
            stop_score,
            stop_max_ticks,
            stop_max_game_time_s,
            stop_max_wall_time_s,
        }
    }
}

impl PySimulationConfig {
    pub fn to_core(&self) -> SimulationConfig {
        SimulationConfig {
            strategy: self.strategy.to_core(),
            tick_ms: self.tick_ms,
            snapshot_count: self.snapshot_count,
            stop: StopCondition {
                score: self
                    .stop_score
                    .as_ref()
                    .map(|d| break_infinity::Decimal::new(d.m, d.e)),
                max_ticks: self.stop_max_ticks,
                max_game_time_ms: self.stop_max_game_time_s.map(|s| s * 1000.0),
                max_wall_time_ms: self.stop_max_wall_time_s.map(|s| s * 1000.0),
            },
        }
    }
}

// ============================================================
// SimulationResult
// ============================================================

/// Result of a completed simulation.
#[pyclass(name = "SimulationResult")]
#[derive(Debug, Clone)]
pub struct PySimulationResult {
    /// Total game time in seconds.
    #[pyo3(get)]
    pub total_time_s: f64,
    /// Number of simulation ticks.
    #[pyo3(get)]
    pub total_ticks: u64,
    /// Which condition caused the simulation to stop.
    #[pyo3(get)]
    pub stop_reason: String,
    /// Full game state at end of simulation.
    #[pyo3(get)]
    pub final_state: PyGameState,
    /// State trace snapshots.
    #[pyo3(get)]
    pub trace: Vec<PySnapshot>,
}

impl PySimulationResult {
    pub fn from_core(result: SimulationResult) -> Self {
        let stop_reason = match result.stop_reason {
            StopReason::ScoreReached => "score_reached",
            StopReason::MaxTicks => "max_ticks",
            StopReason::MaxGameTime => "max_game_time",
            StopReason::MaxWallTime => "max_wall_time",
        }
        .to_string();
        Self {
            total_time_s: result.total_time_ms / 1000.0,
            total_ticks: result.total_ticks,
            stop_reason,
            final_state: PyGameState::from_core(&result.final_state),
            trace: result
                .trace
                .into_iter()
                .map(PySnapshot::from_core)
                .collect(),
        }
    }
}

// ============================================================
// Snapshot
// ============================================================

/// A single state snapshot from the simulation trace.
#[pyclass(name = "Snapshot")]
#[derive(Debug, Clone)]
pub struct PySnapshot {
    /// Tick number.
    #[pyo3(get)]
    pub tick: u64,
    /// Game time in milliseconds.
    #[pyo3(get)]
    pub time_ms: f64,
    /// Full game state at this snapshot.
    #[pyo3(get)]
    pub state: PyGameState,
}

impl PySnapshot {
    pub fn from_core(snap: ad_core::simulator::Snapshot) -> Self {
        Self {
            tick: snap.tick,
            time_ms: snap.time_ms,
            state: PyGameState::from_core(&snap.state),
        }
    }
}
