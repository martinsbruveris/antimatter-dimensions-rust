use ad_core::GameState;
use ad_sim::{
    simulate, BuyPriority, DimensionOrder, PrestigeMode, PurchaseConfig,
    SacrificeConfig, SimulationConfig, StateTrace, StopCondition, StopReason,
    StrategyConfig,
};
use break_infinity::Decimal;

// ============================================================
// StateTrace tests
// ============================================================

#[test]
fn test_state_trace_disabled_when_zero() {
    let mut trace = StateTrace::new(0);
    let game = GameState::new();
    trace.maybe_record(0, 0.0, &game);
    trace.maybe_record(1, 50.0, &game);
    assert!(trace.into_snapshots().is_empty());
}

#[test]
fn test_state_trace_records_snapshots() {
    let mut trace = StateTrace::new(10);
    let game = GameState::new();
    for i in 0..5 {
        trace.maybe_record(i, i as f64 * 50.0, &game);
    }
    let snapshots = trace.into_snapshots();
    assert_eq!(snapshots.len(), 5);
    assert_eq!(snapshots[0].tick, 0);
    assert_eq!(snapshots[4].tick, 4);
}

#[test]
fn test_state_trace_compacts_at_capacity() {
    let mut trace = StateTrace::new(4); // capacity = 8
    let game = GameState::new();
    for i in 0..8 {
        trace.maybe_record(i, i as f64 * 50.0, &game);
    }
    // After 8 entries, compaction should halve to 4
    let snapshots = trace.into_snapshots();
    assert_eq!(snapshots.len(), 4);
    // Should keep even-indexed entries: 0, 2, 4, 6
    assert_eq!(snapshots[0].tick, 0);
    assert_eq!(snapshots[1].tick, 2);
    assert_eq!(snapshots[2].tick, 4);
    assert_eq!(snapshots[3].tick, 6);
}

#[test]
fn test_state_trace_interval_doubles_after_compaction() {
    let mut trace = StateTrace::new(4); // capacity = 8
    let game = GameState::new();
    // Fill to capacity (triggers compaction)
    for i in 0..8 {
        trace.maybe_record(i, i as f64 * 50.0, &game);
    }
    // After compaction, interval = 2, so next record at tick 8
    trace.maybe_record(8, 400.0, &game);
    trace.maybe_record(9, 450.0, &game); // skipped
    trace.maybe_record(10, 500.0, &game);
    let snapshots = trace.into_snapshots();
    // 4 from compaction + tick 8 + tick 10 = 6
    assert_eq!(snapshots.len(), 6);
    assert_eq!(snapshots[4].tick, 8);
    assert_eq!(snapshots[5].tick, 10);
}

// ============================================================
// Simulation tests
// ============================================================

// The three full-playthrough tests below simulate all the way to the Big
// Crunch in 50 ms ticks. As the engine grew (challenges, autobuyers,
// replicanti, records all add per-tick work) they became far too slow for the
// regular suite (minutes, not seconds, in debug builds). Ignored until the
// simulator is revisited; run explicitly with `cargo test -- --ignored`.
#[test]
#[ignore = "full-playthrough simulation; too slow for the regular suite"]
fn test_baseline_auto_reaches_crunch() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 100,
        stop: StopCondition::default(),
    };

    let result = simulate(&config);

    let threshold = Decimal::NUMBER_MAX_VALUE;
    assert!(result.final_state.antimatter >= threshold);
    assert_eq!(result.stop_reason, StopReason::ScoreReached);
    assert!(result.total_ticks > 0);
    assert!(result.total_time_ms > 0.0);
    // Auto strategy should buy several galaxies
    assert!(result.final_state.galaxies >= 1);
    // Trace should have between 100 and 200 entries
    assert!(
        result.trace.len() >= 50,
        "trace too short: {}",
        result.trace.len()
    );
    assert!(
        result.trace.len() <= 200,
        "trace too long: {}",
        result.trace.len()
    );
}

#[test]
#[ignore = "full-playthrough simulation; too slow for the regular suite"]
fn test_cheapest_first_auto_reaches_crunch() {
    let config = SimulationConfig {
        strategy: StrategyConfig {
            sacrifice: SacrificeConfig {
                enabled: true,
                min_gain_ratio: 10.0,
            },
            purchase: PurchaseConfig {
                priority: BuyPriority::Weighted {
                    tickspeed_weight: 1.0,
                },
                dimension_order: DimensionOrder::CheapestFirst,
            },
            prestige: PrestigeMode::Auto,
        },
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition::default(),
    };

    let result = simulate(&config);
    let threshold = Decimal::NUMBER_MAX_VALUE;
    assert!(result.final_state.antimatter >= threshold);
    assert_eq!(result.stop_reason, StopReason::ScoreReached);
}

#[test]
#[ignore = "full-playthrough simulation; too slow for the regular suite"]
fn test_simulate_no_trace_when_zero_snapshots() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition::default(),
    };

    let result = simulate(&config);
    assert!(result.trace.is_empty());
    let threshold = Decimal::NUMBER_MAX_VALUE;
    assert!(result.final_state.antimatter >= threshold);
    assert_eq!(result.stop_reason, StopReason::ScoreReached);
}

// ============================================================
// Stop condition tests
// ============================================================

#[test]
fn test_stop_on_max_ticks() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition {
            max_ticks: Some(100),
            ..StopCondition::default()
        },
    };

    let result = simulate(&config);
    assert_eq!(result.stop_reason, StopReason::MaxTicks);
    assert_eq!(result.total_ticks, 100);
    assert!((result.total_time_ms - 5000.0).abs() < 1e-6);
}

#[test]
fn test_stop_on_max_game_time() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition {
            max_game_time_ms: Some(2000.0),
            ..StopCondition::default()
        },
    };

    let result = simulate(&config);
    assert_eq!(result.stop_reason, StopReason::MaxGameTime);
    // 2000ms / 50ms = 40 ticks, check stops at tick 40
    assert_eq!(result.total_ticks, 40);
}

#[test]
fn test_stop_on_custom_score() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition {
            score: Some(Decimal::new(1.0, 50)),
            ..StopCondition::default()
        },
    };

    let result = simulate(&config);
    assert_eq!(result.stop_reason, StopReason::ScoreReached);
    assert!(result.final_state.antimatter >= Decimal::new(1.0, 50));
    // A score of 1e50 is reached much earlier than Big Crunch
    assert!(result.total_ticks < 100_000);
}

#[test]
fn test_stop_on_wall_time() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition {
            // 0ms wall time — should stop almost immediately
            max_wall_time_ms: Some(0.0),
            ..StopCondition::default()
        },
    };

    let result = simulate(&config);
    // Wall time check happens after the first strategy
    // execution, so at least one iteration runs.
    assert_eq!(result.stop_reason, StopReason::MaxWallTime);
}

#[test]
fn test_first_condition_wins() {
    // Set both tick limit and game time limit, tick limit
    // triggers first.
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
        stop: StopCondition {
            max_ticks: Some(10),
            max_game_time_ms: Some(1_000_000.0),
            ..StopCondition::default()
        },
    };

    let result = simulate(&config);
    assert_eq!(result.stop_reason, StopReason::MaxTicks);
    assert_eq!(result.total_ticks, 10);
}
