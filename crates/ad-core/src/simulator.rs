use std::time::Instant;

use break_infinity::Decimal;

use crate::data::constants::big_crunch_threshold;
use crate::observed::ObservedState;
use crate::state::GameState;
use crate::strategy::{
    BuyPriority, DimensionOrder, PrestigeMode, PrestigeStep, StrategyConfig,
};

/// Conditions under which the simulation should stop.
///
/// Multiple conditions can be set simultaneously; the simulation
/// stops as soon as any one is met. If no conditions are set,
/// the simulation runs until the Big Crunch antimatter threshold
/// is reached (the default).
#[derive(Debug, Clone, Default)]
pub struct StopCondition {
    /// Stop when antimatter reaches this value.
    /// Defaults to `big_crunch_threshold()` if `None`.
    pub score: Option<Decimal>,
    /// Stop after this many ticks.
    pub max_ticks: Option<u64>,
    /// Stop after this much game time (milliseconds).
    pub max_game_time_ms: Option<f64>,
    /// Stop after this much wall-clock time (milliseconds).
    pub max_wall_time_ms: Option<f64>,
}

/// Indicates which condition caused the simulation to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Antimatter reached the score threshold.
    ScoreReached,
    /// The tick limit was reached.
    MaxTicks,
    /// The game-time limit was reached.
    MaxGameTime,
    /// The wall-clock time limit was reached.
    MaxWallTime,
}

/// Configuration for a simulation run.
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Buying strategy.
    pub strategy: StrategyConfig,
    /// Time step in milliseconds.
    pub tick_ms: f64,
    /// Approximate number of state snapshots to return.
    /// Actual count will be between this and 2x this.
    /// Set to 0 to disable tracing.
    pub snapshot_count: usize,
    /// When to stop the simulation.
    pub stop: StopCondition,
}

/// Result of a completed simulation.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Total game time elapsed (milliseconds).
    pub total_time_ms: f64,
    /// Number of simulation ticks executed.
    pub total_ticks: u64,
    /// Which condition caused the simulation to stop.
    pub stop_reason: StopReason,
    /// Final observed state at end of simulation.
    pub final_state: ObservedState,
    /// State trace (adaptive resolution).
    pub trace: Vec<Snapshot>,
}

/// A single state snapshot in the trace.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub tick: u64,
    pub time_ms: f64,
    pub state: ObservedState,
}

/// Streaming downsampling buffer for state traces.
#[derive(Debug)]
pub struct StateTrace {
    buffer: Vec<Snapshot>,
    /// Buffer capacity = 2 * snapshot_count.
    capacity: usize,
    /// Current recording interval (in ticks): 1, 2, 4, 8, ...
    interval: u64,
    /// Next tick number at which to record.
    next_record_tick: u64,
}

impl StateTrace {
    pub fn new(snapshot_count: usize) -> Self {
        let capacity = 2 * snapshot_count;
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            interval: 1,
            next_record_tick: 0,
        }
    }

    /// Called every tick. Records the state if it's time.
    pub fn maybe_record(&mut self, tick: u64, time_ms: f64, state: &GameState) {
        if self.capacity == 0 {
            return;
        }

        if tick < self.next_record_tick {
            return;
        }

        self.buffer.push(Snapshot {
            tick,
            time_ms,
            state: ObservedState::from_game_state(state),
        });
        self.next_record_tick = tick + self.interval;

        // Compact when buffer is full
        if self.buffer.len() >= self.capacity {
            let mut i = 0;
            self.buffer.retain(|_| {
                let keep = i % 2 == 0;
                i += 1;
                keep
            });
            self.interval *= 2;
        }
    }

    pub fn into_snapshots(self) -> Vec<Snapshot> {
        self.buffer
    }
}

/// Tracks progress through a fixed prestige plan.
#[derive(Debug)]
struct PlanCursor {
    /// Index into the prestige plan.
    step_index: usize,
    /// For DimBoost(N) steps, how many boosts have been
    /// bought in this step so far.
    boosts_done: u32,
}

impl PlanCursor {
    fn new() -> Self {
        Self {
            step_index: 0,
            boosts_done: 0,
        }
    }

    /// Returns the current step, or None if plan is exhausted.
    fn current_step<'a>(&self, plan: &'a [PrestigeStep]) -> Option<&'a PrestigeStep> {
        plan.get(self.step_index)
    }

    /// Advance to the next step in the plan.
    fn advance(&mut self) {
        self.step_index += 1;
        self.boosts_done = 0;
    }
}

/// Run a complete simulation from a fresh game.
///
/// The simulation runs until one of the configured stop
/// conditions is met (see [`StopCondition`]). If no conditions
/// are set, it defaults to stopping at the Big Crunch antimatter
/// threshold.
pub fn simulate(config: &SimulationConfig) -> SimulationResult {
    let mut game = GameState::new();
    // Disable autobuyers — the strategy handles all purchases.
    game.autobuyers.enabled = false;

    let mut time_ms: f64 = 0.0;
    let mut ticks: u64 = 0;
    let mut cursor = PlanCursor::new();
    let mut trace = StateTrace::new(config.snapshot_count);

    let score_threshold = config.stop.score.unwrap_or_else(big_crunch_threshold);

    let wall_start = Instant::now();

    loop {
        // 1. Execute strategy (buy everything possible)
        execute_strategy(&mut game, &config.strategy, &mut cursor);

        // 2. Check stop conditions
        if game.antimatter >= score_threshold {
            return make_result(time_ms, ticks, StopReason::ScoreReached, &game, trace);
        }
        if let Some(max) = config.stop.max_ticks {
            if ticks >= max {
                return make_result(time_ms, ticks, StopReason::MaxTicks, &game, trace);
            }
        }
        if let Some(max) = config.stop.max_game_time_ms {
            if time_ms >= max {
                return make_result(
                    time_ms,
                    ticks,
                    StopReason::MaxGameTime,
                    &game,
                    trace,
                );
            }
        }
        if let Some(max) = config.stop.max_wall_time_ms {
            let elapsed = wall_start.elapsed().as_secs_f64() * 1000.0;
            if elapsed >= max {
                return make_result(
                    time_ms,
                    ticks,
                    StopReason::MaxWallTime,
                    &game,
                    trace,
                );
            }
        }

        // 3. Record state (if it's time)
        trace.maybe_record(ticks, time_ms, &game);

        // 4. Advance one tick (autobuyers disabled, production
        //    only)
        game.tick(config.tick_ms);
        time_ms += config.tick_ms;
        ticks += 1;
    }
}

fn make_result(
    time_ms: f64,
    ticks: u64,
    stop_reason: StopReason,
    game: &GameState,
    trace: StateTrace,
) -> SimulationResult {
    SimulationResult {
        total_time_ms: time_ms,
        total_ticks: ticks,
        stop_reason,
        final_state: ObservedState::from_game_state(game),
        trace: trace.into_snapshots(),
    }
}

/// Execute the buying strategy: loop buying until nothing more
/// can be bought.
fn execute_strategy(
    game: &mut GameState,
    strategy: &StrategyConfig,
    cursor: &mut PlanCursor,
) {
    loop {
        // 1. Prestige check
        let prestiged = match &strategy.prestige {
            PrestigeMode::Auto => try_prestige_auto(game),
            PrestigeMode::Plan(plan) => try_prestige_plan(game, plan, cursor),
        };
        if prestiged {
            continue;
        }

        // 2. Sacrifice check
        let sacrificed = try_sacrifice(game, &strategy.sacrifice);

        // 3. Purchase loop
        let purchased = buy_step(game, &strategy.purchase);

        if !sacrificed && !purchased {
            break;
        }
    }
}

/// Auto-prestige: buy galaxy if affordable, else buy dim boost.
/// Returns true if a prestige event occurred.
fn try_prestige_auto(game: &mut GameState) -> bool {
    // Galaxy takes priority — permanent tickspeed improvement
    if game.can_buy_galaxy() {
        game.buy_galaxy();
        return true;
    }
    if game.can_dim_boost() {
        game.buy_dim_boost();
        return true;
    }
    false
}

/// Follow a fixed prestige plan. Returns true if a prestige
/// event occurred.
fn try_prestige_plan(
    game: &mut GameState,
    plan: &[PrestigeStep],
    cursor: &mut PlanCursor,
) -> bool {
    let step = match cursor.current_step(plan) {
        Some(s) => *s,
        None => return false,
    };

    match step {
        PrestigeStep::DimBoost(total) => {
            if cursor.boosts_done >= total {
                cursor.advance();
                return false;
            }
            if game.can_dim_boost() {
                game.buy_dim_boost();
                cursor.boosts_done += 1;
                if cursor.boosts_done >= total {
                    cursor.advance();
                }
                return true;
            }
            false
        }
        PrestigeStep::Galaxy => {
            if game.can_buy_galaxy() {
                game.buy_galaxy();
                cursor.advance();
                return true;
            }
            false
        }
    }
}

/// Try to sacrifice if the strategy config allows it and the
/// gain ratio is sufficient.
/// Matches JS autobuyer: sacrifices when
/// `nextBoost >= max(multiplier, 1.01)`.
fn try_sacrifice(
    game: &mut GameState,
    config: &crate::strategy::SacrificeConfig,
) -> bool {
    if !config.enabled || !game.can_sacrifice() {
        return false;
    }

    let next_boost = game.next_sacrifice_boost().to_f64();
    if next_boost >= config.min_gain_ratio {
        game.sacrifice()
    } else {
        false
    }
}

/// Execute one round of purchases. Returns true if anything
/// was bought.
///
/// Uses buy-max operations for performance. The priority logic
/// determines what to buy first, but we buy as much as
/// affordable of each choice before moving on.
fn buy_step(game: &mut GameState, config: &crate::strategy::PurchaseConfig) -> bool {
    let mut bought_anything = false;

    // Outer loop: after buying max of one thing, we may now
    // be able to buy something else with remaining antimatter.
    loop {
        let tickspeed_cost = game.tickspeed.cost;
        let can_afford_tickspeed = game.antimatter >= tickspeed_cost;

        let dim_option = best_dimension(game, config.dimension_order);

        let BuyPriority::Weighted { tickspeed_weight } = config.priority;

        match (can_afford_tickspeed, dim_option) {
            (true, Some((tier, dim_cost))) => {
                let effective_ts =
                    tickspeed_cost / Decimal::from_float(tickspeed_weight);
                if effective_ts <= dim_cost {
                    let n = game.buy_max_tickspeed();
                    bought_anything |= n > 0;
                } else {
                    let n = game.buy_max_dimension(tier);
                    bought_anything |= n > 0;
                }
            }
            (true, None) => {
                let n = game.buy_max_tickspeed();
                bought_anything |= n > 0;
                if n == 0 {
                    break;
                }
            }
            (false, Some((tier, dim_cost))) => {
                if game.antimatter >= dim_cost {
                    let n = game.buy_max_dimension(tier);
                    bought_anything |= n > 0;
                } else {
                    break;
                }
            }
            (false, None) => break,
        }
    }

    bought_anything
}

/// Find the best dimension to buy according to the given order.
/// Returns (tier, unit_cost) or None if no dimension is
/// affordable (single unit).
fn best_dimension(game: &GameState, order: DimensionOrder) -> Option<(usize, Decimal)> {
    let unlocked = game.unlocked_dimensions();

    match order {
        DimensionOrder::HighestFirst => {
            for tier in (0..unlocked).rev() {
                let cost = game.dimension_cost(tier);
                if game.antimatter >= cost {
                    return Some((tier, cost));
                }
            }
            None
        }
        DimensionOrder::LowestFirst => {
            for tier in 0..unlocked {
                let cost = game.dimension_cost(tier);
                if game.antimatter >= cost {
                    return Some((tier, cost));
                }
            }
            None
        }
        DimensionOrder::CheapestFirst => {
            let mut best: Option<(usize, Decimal)> = None;
            for tier in 0..unlocked {
                let cost = game.dimension_cost(tier);
                if game.antimatter >= cost {
                    match &best {
                        None => best = Some((tier, cost)),
                        Some((_, best_cost)) => {
                            if cost < *best_cost {
                                best = Some((tier, cost));
                            }
                        }
                    }
                }
            }
            best
        }
    }
}
