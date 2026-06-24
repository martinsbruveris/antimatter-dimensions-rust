use ad_core::simulator::{simulate, SimulationConfig, StateTrace};
use ad_core::strategy::{
    BuyPriority, DimensionOrder, PrestigeMode, PurchaseConfig, SacrificeConfig,
    StrategyConfig,
};
use ad_core::GameState;
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

#[test]
fn test_baseline_auto_reaches_crunch() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 100,
    };

    let result = simulate(&config);

    let threshold = Decimal::new(1.7976931348623157, 308);
    assert!(result.final_state.antimatter >= threshold);
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
    };

    let result = simulate(&config);
    let threshold = Decimal::new(1.7976931348623157, 308);
    assert!(result.final_state.antimatter >= threshold);
}

#[test]
fn test_simulate_no_trace_when_zero_snapshots() {
    let config = SimulationConfig {
        strategy: StrategyConfig::baseline(),
        tick_ms: 50.0,
        snapshot_count: 0,
    };

    let result = simulate(&config);
    assert!(result.trace.is_empty());
    let threshold = Decimal::new(1.7976931348623157, 308);
    assert!(result.final_state.antimatter >= threshold);
}
