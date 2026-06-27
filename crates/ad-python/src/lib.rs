use pyo3::prelude::*;

mod types;

use types::{PySimulationConfig, PySimulationResult};

/// Run a complete simulation from a fresh game until Big Crunch.
///
/// Args:
///     config: Simulation configuration (strategy + tick size +
///         snapshot count).
///
/// Returns:
///     SimulationResult with final stats and optional state
///     trace.
#[pyfunction]
fn simulate(config: PySimulationConfig) -> PySimulationResult {
    let core_config = config.to_core();
    let result = ad_sim::simulate(&core_config);
    PySimulationResult::from_core(result)
}

/// Antimatter Dimensions simulation engine.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(
        "BIG_CRUNCH_THRESHOLD",
        types::PyDecimal::from_big_crunch_threshold(),
    )?;
    m.add_function(wrap_pyfunction!(simulate, m)?)?;
    m.add_class::<types::PyDecimal>()?;
    m.add_class::<types::PyDecimalArray>()?;
    m.add_class::<types::PyDimensionTier>()?;
    m.add_class::<types::PyTickspeedState>()?;
    m.add_class::<types::PyGameState>()?;
    m.add_class::<types::PyStrategyConfig>()?;
    m.add_class::<types::PySimulationConfig>()?;
    m.add_class::<types::PySimulationResult>()?;
    m.add_class::<types::PySnapshot>()?;
    Ok(())
}
