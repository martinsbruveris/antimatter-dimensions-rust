use pyo3::prelude::*;

use ad_core::simulator::{SimulationConfig, SimulationResult};
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

    /// Return log10 of this value.
    fn log10(&self) -> f64 {
        if self.m <= 0.0 {
            f64::NEG_INFINITY
        } else {
            self.m.log10() + self.e as f64
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
    /// Return log10 of each value as a list.
    fn log10(&self) -> Vec<f64> {
        self.m
            .iter()
            .zip(self.e.iter())
            .map(|(m, e)| {
                if *m <= 0.0 {
                    f64::NEG_INFINITY
                } else {
                    m.log10() + *e as f64
                }
            })
            .collect()
    }

    fn __len__(&self) -> usize {
        self.m.len()
    }

    fn __repr__(&self) -> String {
        format!("DecimalArray(len={})", self.m.len())
    }
}

impl PyDecimalArray {
    fn from_decimals(decimals: &[Decimal]) -> Self {
        Self {
            m: decimals.iter().map(|d| d.mantissa()).collect(),
            e: decimals.iter().map(|d| d.exponent()).collect(),
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
#[pyclass(name = "SimulationConfig")]
#[derive(Debug, Clone)]
pub struct PySimulationConfig {
    #[pyo3(get, set)]
    pub strategy: PyStrategyConfig,
    #[pyo3(get, set)]
    pub tick_ms: f64,
    #[pyo3(get, set)]
    pub snapshot_count: usize,
}

#[pymethods]
impl PySimulationConfig {
    #[new]
    #[pyo3(signature = (strategy, tick_ms = 50.0, snapshot_count = 500))]
    fn new(strategy: PyStrategyConfig, tick_ms: f64, snapshot_count: usize) -> Self {
        Self {
            strategy,
            tick_ms,
            snapshot_count,
        }
    }
}

impl PySimulationConfig {
    pub fn to_core(&self) -> SimulationConfig {
        SimulationConfig {
            strategy: self.strategy.to_core(),
            tick_ms: self.tick_ms,
            snapshot_count: self.snapshot_count,
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
    /// Final galaxy count.
    #[pyo3(get)]
    pub galaxies: u32,
    /// Final dimension boost count.
    #[pyo3(get)]
    pub dim_boosts: u32,
    /// Final antimatter amount.
    #[pyo3(get)]
    pub final_antimatter: PyDecimal,
    /// State trace snapshots.
    #[pyo3(get)]
    pub trace: Vec<PySnapshot>,
}

impl PySimulationResult {
    pub fn from_core(result: SimulationResult) -> Self {
        Self {
            total_time_s: result.total_time_ms / 1000.0,
            total_ticks: result.total_ticks,
            galaxies: result.final_galaxies,
            dim_boosts: result.final_dim_boosts,
            final_antimatter: PyDecimal::from_decimal(
                &result.final_antimatter,
            ),
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
    /// Current antimatter.
    #[pyo3(get)]
    pub antimatter: PyDecimal,
    /// Number of dimension boosts.
    #[pyo3(get)]
    pub dim_boosts: u32,
    /// Number of galaxies.
    #[pyo3(get)]
    pub galaxies: u32,
    /// Dimension amounts (8 tiers).
    #[pyo3(get)]
    pub dimension_amounts: PyDecimalArray,
    /// Number of purchases for each dimension (8 tiers).
    #[pyo3(get)]
    pub dimension_bought: Vec<u64>,
}

impl PySnapshot {
    pub fn from_core(snap: ad_core::simulator::Snapshot) -> Self {
        let amounts: Vec<Decimal> =
            snap.state.dimensions.iter().map(|d| d.amount).collect();
        let dimension_bought =
            snap.state.dimensions.iter().map(|d| d.bought).collect();
        Self {
            tick: snap.tick,
            time_ms: snap.time_ms,
            antimatter: PyDecimal::from_decimal(&snap.state.antimatter),
            dim_boosts: snap.state.dim_boosts,
            galaxies: snap.state.galaxies,
            dimension_amounts: PyDecimalArray::from_decimals(&amounts),
            dimension_bought,
        }
    }
}
