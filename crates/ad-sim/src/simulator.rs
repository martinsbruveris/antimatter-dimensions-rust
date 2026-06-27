use std::time::Instant;

use ad_core::data::constants::BIG_CRUNCH_THRESHOLD;
use ad_core::observed::ObservedState;
use ad_core::state::GameState;
use break_infinity::Decimal;

use crate::controller::{Controller, StrategyController};
use crate::strategy::StrategyConfig;

/// Conditions under which the simulation should stop.
///
/// Multiple conditions can be set simultaneously; the simulation
/// stops as soon as any one is met. If no conditions are set,
/// the simulation runs until the Big Crunch antimatter threshold
/// is reached (the default).
#[derive(Debug, Clone, Default)]
pub struct StopCondition {
    /// Stop when antimatter reaches this value.
    /// Defaults to `BIG_CRUNCH_THRESHOLD` if `None`.
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

/// Configuration for a strategy-driven simulation run.
///
/// This is the convenience entry point used by [`simulate`] (and
/// the Python bindings): it pairs a [`StrategyConfig`] with the
/// simulation parameters. For controllers other than
/// [`StrategyController`], call [`run_simulation`] directly.
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

/// Drive a fresh game with the given controller until a stop
/// condition is met.
///
/// Each tick the controller is asked, repeatedly, for the actions
/// it wants to take given the current [`ObservedState`]; every
/// action is routed through [`GameState::apply_action`]. Once the
/// controller is quiescent (returns no further applicable action)
/// the engine advances one production tick. The controller can
/// only emit `Action`s the engine validates, so it can never put
/// the game into an illegal state — the separation between
/// "deciding" and "the rules" is enforced by this seam.
///
/// `on_start` runs once before the loop, letting a controller set
/// up engine flags it depends on (e.g. the [`StrategyController`]
/// disables in-game autobuyers there, since it does all the
/// buying itself).
pub fn run_simulation<C: Controller>(
    mut controller: C,
    tick_ms: f64,
    snapshot_count: usize,
    stop: &StopCondition,
) -> SimulationResult {
    let mut game = GameState::new();
    controller.on_start(&mut game);

    let mut time_ms: f64 = 0.0;
    let mut ticks: u64 = 0;
    let mut trace = StateTrace::new(snapshot_count);

    let score_threshold = stop.score.unwrap_or(BIG_CRUNCH_THRESHOLD);
    let wall_start = Instant::now();

    loop {
        // 1. Let the controller act until it is quiescent. A fresh
        //    observation is taken before each batch so the
        //    controller sees the effect of its previous actions
        //    (e.g. spent antimatter, a now-affordable galaxy).
        loop {
            let obs = ObservedState::from_game_state(&game);
            let actions = controller.act(&obs, time_ms);
            if actions.is_empty() {
                break;
            }
            let mut progressed = false;
            for action in actions {
                progressed |= game.apply_action(action).applied;
            }
            if !progressed {
                break;
            }
        }

        // 2. Check stop conditions.
        if game.antimatter >= score_threshold {
            return make_result(time_ms, ticks, StopReason::ScoreReached, &game, trace);
        }
        if let Some(max) = stop.max_ticks {
            if ticks >= max {
                return make_result(time_ms, ticks, StopReason::MaxTicks, &game, trace);
            }
        }
        if let Some(max) = stop.max_game_time_ms {
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
        if let Some(max) = stop.max_wall_time_ms {
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

        // 3. Record state (if it's time).
        trace.maybe_record(ticks, time_ms, &game);

        // 4. Advance one tick.
        game.tick(tick_ms);
        time_ms += tick_ms;
        ticks += 1;
    }
}

/// Run a complete strategy-driven simulation from a fresh game.
///
/// Thin wrapper over [`run_simulation`] that constructs a
/// [`StrategyController`] from `config.strategy`. Kept for API
/// parity with the previous `ad_core::simulator::simulate` and as
/// the entry point for the Python bindings.
pub fn simulate(config: &SimulationConfig) -> SimulationResult {
    let controller = StrategyController::new(config.strategy.clone());
    run_simulation(
        controller,
        config.tick_ms,
        config.snapshot_count,
        &config.stop,
    )
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
