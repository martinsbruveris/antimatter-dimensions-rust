"""Type stubs for ad_python native module."""

class Decimal:
    """A single Decimal value: m × 10^e."""

    @property
    def m(self) -> float: ...
    @property
    def e(self) -> int: ...
    def __repr__(self) -> str: ...

class DecimalArray:
    """Batch of Decimal values as parallel arrays."""

    @property
    def m(self) -> list[float]: ...
    @property
    def e(self) -> list[int]: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class DimensionTier:
    """A single antimatter dimension tier."""

    @property
    def amount(self) -> Decimal: ...
    @property
    def bought(self) -> int: ...

class TickspeedState:
    """Tickspeed upgrade state."""

    @property
    def bought(self) -> int: ...
    @property
    def cost(self) -> Decimal: ...
    @property
    def cost_multiplier(self) -> Decimal: ...

class GameState:
    """Full game state for pre-infinity gameplay."""

    @property
    def antimatter(self) -> Decimal: ...
    @property
    def dimensions(self) -> list[DimensionTier]: ...
    @property
    def tickspeed(self) -> TickspeedState: ...
    @property
    def dim_boosts(self) -> int: ...
    @property
    def galaxies(self) -> int: ...
    @property
    def sacrificed(self) -> Decimal: ...
    @property
    def sacrifice_boost(self) -> Decimal: ...
    @property
    def sacrifice_unlocked(self) -> bool: ...

class StrategyConfig:
    """Strategy configuration for the simulation."""

    sacrifice_enabled: bool
    sacrifice_threshold: float
    tickspeed_weight: float
    dimension_order: str

    @property
    def prestige_mode(self) -> str | list[str]: ...
    def __init__(
        self,
        sacrifice_enabled: bool = True,
        sacrifice_threshold: float = 10.0,
        tickspeed_weight: float = 1.0,
        dimension_order: str = "highest_first",
        prestige_mode: str | list[str] | None = None,
    ) -> None: ...

class SimulationConfig:
    """Configuration for a simulation run."""

    strategy: StrategyConfig
    tick_ms: float
    snapshot_count: int

    def __init__(
        self,
        strategy: StrategyConfig,
        tick_ms: float = 33.0,
        snapshot_count: int = 1_000,
    ) -> None: ...

class Snapshot:
    """A single state snapshot from the simulation trace."""

    @property
    def tick(self) -> int: ...
    @property
    def time_ms(self) -> float: ...
    @property
    def state(self) -> GameState: ...

class SimulationResult:
    """Result of a completed simulation."""

    @property
    def total_time_s(self) -> float: ...
    @property
    def total_ticks(self) -> int: ...
    @property
    def final_state(self) -> GameState: ...
    @property
    def trace(self) -> list[Snapshot]: ...

def simulate(config: SimulationConfig) -> SimulationResult:
    """Run a simulation from fresh game until Big Crunch."""
    ...
