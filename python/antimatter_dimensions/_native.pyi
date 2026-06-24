"""Type stubs for ad_python native module."""

class Decimal:
    """A single Decimal value: m × 10^e."""

    @property
    def m(self) -> float: ...
    @property
    def e(self) -> int: ...
    def log10(self) -> float: ...
    def __repr__(self) -> str: ...

class DecimalArray:
    """Batch of Decimal values as parallel arrays."""

    @property
    def m(self) -> list[float]: ...
    @property
    def e(self) -> list[int]: ...
    def log10(self) -> list[float]: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

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
        tick_ms: float = 50.0,
        snapshot_count: int = 500,
    ) -> None: ...

class Snapshot:
    """A single state snapshot from the simulation trace."""

    @property
    def tick(self) -> int: ...
    @property
    def time_ms(self) -> float: ...
    @property
    def antimatter(self) -> Decimal: ...
    @property
    def dim_boosts(self) -> int: ...
    @property
    def galaxies(self) -> int: ...
    @property
    def dimension_amounts(self) -> DecimalArray: ...
    @property
    def dimension_bought(self) -> list[int]: ...

class SimulationResult:
    """Result of a completed simulation."""

    @property
    def total_time_s(self) -> float: ...
    @property
    def total_ticks(self) -> int: ...
    @property
    def galaxies(self) -> int: ...
    @property
    def dim_boosts(self) -> int: ...
    @property
    def final_antimatter(self) -> Decimal: ...
    @property
    def trace(self) -> list[Snapshot]: ...

def simulate(config: SimulationConfig) -> SimulationResult:
    """Run a simulation from fresh game until Big Crunch."""
    ...
