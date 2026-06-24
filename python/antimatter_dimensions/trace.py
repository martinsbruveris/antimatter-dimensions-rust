"""Vectorized trace types for simulation results."""

from __future__ import annotations

import numpy as np
from numpy.typing import NDArray


class DecimalSeries:
    """Series of Decimal values as parallel numpy arrays.

    Attributes:
        m: Mantissas (float64), each in [1, 10), (-10, -1] or 0.
        e: Exponents (int64).
    """

    __slots__ = ("m", "e")

    def __init__(self, m: NDArray[np.float64], e: NDArray[np.int64]) -> None:
        self.m = m
        self.e = e

    def __len__(self) -> int:
        return len(self.m)

    def __repr__(self) -> str:
        return f"DecimalSeries(len={len(self)})"


class DimensionsTrace:
    """All 8 dimension tiers vectorized across trace snapshots.

    Arrays have shape (N, 8) where N is the number of snapshots
    and columns are dimension tiers 1-8.

    Attributes:
        amount: Dimension amounts over time.
        bought: Number of purchases over time (uint64).
    """

    __slots__ = ("amount", "bought")

    def __init__(self, amount: DecimalSeries, bought: np.ndarray) -> None:
        self.amount = amount
        self.bought = bought


class TickspeedTrace:
    """Tickspeed state vectorized across trace snapshots.

    Attributes:
        bought: Number of upgrades over time (uint64).
        cost: Cost of next upgrade over time.
        cost_multiplier: Cost multiplier over time.
    """

    __slots__ = ("bought", "cost", "cost_multiplier")

    def __init__(
        self,
        bought: np.ndarray,
        cost: DecimalSeries,
        cost_multiplier: DecimalSeries,
    ) -> None:
        self.bought = bought
        self.cost = cost
        self.cost_multiplier = cost_multiplier


def _decimal_series(snapshots: list, accessor) -> DecimalSeries:
    """Extract a DecimalSeries from snapshots."""
    return DecimalSeries(
        m=np.array([accessor(s).m for s in snapshots]),
        e=np.array([accessor(s).e for s in snapshots], dtype=np.int64),
    )


class Trace:
    """Vectorized game state trace.

    Mirrors the GameState structure with each scalar field
    replaced by a numpy array across snapshots.

    Attributes:
        tick: Tick numbers (uint64).
        time_ms: Game time in milliseconds (float64).
        antimatter: Antimatter amounts.
        dimensions: All 8 dimension tiers (arrays shape (N, 8)).
        tickspeed: Tickspeed state.
        dim_boosts: Dimension boost counts (uint32).
        galaxies: Galaxy counts (uint32).
        sacrificed: Total sacrificed amounts.
        sacrifice_boost: Sacrifice boost multipliers.
        sacrifice_unlocked: Whether sacrifice is unlocked.
    """

    __slots__ = (
        "tick",
        "time_ms",
        "antimatter",
        "dimensions",
        "tickspeed",
        "dim_boosts",
        "galaxies",
        "sacrificed",
        "sacrifice_boost",
        "sacrifice_unlocked",
    )

    def __init__(self, snapshots: list) -> None:
        self.tick = np.array([s.tick for s in snapshots], dtype=np.uint64)
        self.time_ms = np.array([s.time_ms for s in snapshots], dtype=np.float64)
        self.antimatter = _decimal_series(snapshots, lambda s: s.state.antimatter)
        self.dimensions = DimensionsTrace(
            amount=DecimalSeries(
                m=np.array(
                    [[d.amount.m for d in s.state.dimensions] for s in snapshots]
                ),
                e=np.array(
                    [[d.amount.e for d in s.state.dimensions] for s in snapshots],
                    dtype=np.int64,
                ),
            ),
            bought=np.array(
                [[d.bought for d in s.state.dimensions] for s in snapshots],
                dtype=np.uint64,
            ),
        )
        self.tickspeed = TickspeedTrace(
            bought=np.array(
                [s.state.tickspeed.bought for s in snapshots],
                dtype=np.uint64,
            ),
            cost=_decimal_series(snapshots, lambda s: s.state.tickspeed.cost),
            cost_multiplier=_decimal_series(
                snapshots,
                lambda s: s.state.tickspeed.cost_multiplier,
            ),
        )
        self.dim_boosts = np.array(
            [s.state.dim_boosts for s in snapshots],
            dtype=np.uint32,
        )
        self.galaxies = np.array(
            [s.state.galaxies for s in snapshots],
            dtype=np.uint32,
        )
        self.sacrificed = _decimal_series(snapshots, lambda s: s.state.sacrificed)
        self.sacrifice_boost = _decimal_series(
            snapshots, lambda s: s.state.sacrifice_boost
        )
        self.sacrifice_unlocked = np.array(
            [s.state.sacrifice_unlocked for s in snapshots],
            dtype=np.bool_,
        )

    def __len__(self) -> int:
        return len(self.tick)

    def __repr__(self) -> str:
        return f"Trace(len={len(self)})"
