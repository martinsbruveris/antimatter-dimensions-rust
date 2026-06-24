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
    let result = ad_core::simulator::simulate(&core_config);
    PySimulationResult::from_core(result)
}

/// Antimatter Dimensions simulation engine.
#[pymodule]
fn ad_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(simulate, m)?)?;
    m.add_class::<types::PyStrategyConfig>()?;
    m.add_class::<types::PySimulationConfig>()?;
    m.add_class::<types::PySimulationResult>()?;
    m.add_class::<types::PySnapshot>()?;
    Ok(())
}
