"""Antimatter Dimensions simulation engine (Python bindings)."""

from ._native import (
    BIG_CRUNCH_THRESHOLD,
    Decimal,
    DecimalArray,
    DimensionTier,
    GameState,
    SimulationConfig,
    Snapshot,
    StrategyConfig,
    TickspeedState,
)
from ._native import simulate as _simulate_native
from .trace import (
    DecimalSeries,
    DimensionsTrace,
    TickspeedTrace,
    Trace,
)

__all__ = [
    "BIG_CRUNCH_THRESHOLD",
    "Decimal",
    "DecimalArray",
    "DecimalSeries",
    "DimensionTier",
    "DimensionsTrace",
    "GameState",
    "SimulationConfig",
    "SimulationResult",
    "Snapshot",
    "StrategyConfig",
    "TickspeedState",
    "TickspeedTrace",
    "Trace",
    "simulate",
]


class SimulationResult:
    """Result of a completed simulation.

    Attributes:
        total_time_s: Total game time in seconds.
        total_ticks: Number of simulation ticks.
        stop_reason: Which condition stopped the simulation.
            One of "score_reached", "max_ticks",
            "max_game_time", "max_wall_time".
        final_state: Full game state at end of simulation.
        trace: Vectorized state trace as numpy arrays.
    """

    __slots__ = (
        "total_time_s",
        "total_ticks",
        "stop_reason",
        "final_state",
        "trace",
    )

    def __init__(
        self,
        total_time_s: float,
        total_ticks: int,
        stop_reason: str,
        final_state: GameState,
        trace: Trace,
    ) -> None:
        self.total_time_s = total_time_s
        self.total_ticks = total_ticks
        self.stop_reason = stop_reason
        self.final_state = final_state
        self.trace = trace

    def __repr__(self) -> str:
        return (
            f"SimulationResult(ticks={self.total_ticks}, "
            f"time={self.total_time_s:.1f}s, "
            f"stop={self.stop_reason}, "
            f"trace_len={len(self.trace)})"
        )


def simulate(config: SimulationConfig) -> SimulationResult:
    """Run a simulation from fresh game until Big Crunch.

    Args:
        config: Simulation configuration.

    Returns:
        SimulationResult with final state and vectorized trace.
    """
    native = _simulate_native(config)
    return SimulationResult(
        total_time_s=native.total_time_s,
        total_ticks=native.total_ticks,
        stop_reason=native.stop_reason,
        final_state=native.final_state,
        trace=Trace(native.trace),
    )
