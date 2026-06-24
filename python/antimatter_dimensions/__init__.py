"""Antimatter Dimensions simulation engine (Python bindings)."""

from ._native import *  # noqa: F401, F403
from ._native import __doc__

__all__ = [
    "Decimal",
    "DecimalArray",
    "StrategyConfig",
    "SimulationConfig",
    "SimulationResult",
    "Snapshot",
    "simulate",
]
