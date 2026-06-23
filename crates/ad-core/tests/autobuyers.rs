use ad_core::{Autobuyer, AutobuyerMode, GameState};
use break_infinity::Decimal;

// ============================================================
// Basic autobuyer state tests
// ============================================================

#[test]
fn test_autobuyers_disabled_by_default() {
    let game = GameState::new();
    for i in 0..8 {
        assert!(!game.autobuyers.dimensions[i].enabled);
    }
    assert!(!game.autobuyers.tickspeed.enabled);
}

#[test]
fn test_autobuyer_default_mode_is_buy_single() {
    let game = GameState::new();
    for i in 0..8 {
        assert_eq!(game.autobuyers.dimensions[i].mode, AutobuyerMode::BuySingle);
    }
    assert_eq!(game.autobuyers.tickspeed.mode, AutobuyerMode::BuySingle);
}

// ============================================================
// Autobuyer timer tests
// ============================================================

#[test]
fn test_autobuyer_does_not_fire_when_disabled() {
    let mut ab = Autobuyer::new(1000.0);
    ab.enabled = false;
    assert!(!ab.advance(2000.0));
}

#[test]
fn test_autobuyer_fires_after_interval() {
    let mut ab = Autobuyer::new(1000.0);
    ab.enabled = true;
    // Not enough time
    assert!(!ab.advance(500.0));
    // Now enough time elapsed (total 1500ms > 1000ms)
    assert!(ab.advance(500.0));
}

#[test]
fn test_autobuyer_timer_resets_after_firing() {
    let mut ab = Autobuyer::new(1000.0);
    ab.enabled = true;
    // Advance to 1200ms (fires at 1000ms, timer should be 200ms)
    assert!(ab.advance(1200.0));
    // Need another 800ms to fire again
    assert!(!ab.advance(700.0));
    assert!(ab.advance(100.0));
}

// ============================================================
// Dimension autobuyer integration tests
// ============================================================

#[test]
fn test_dimension_autobuyer_buys_single() {
    let mut game = GameState::new();
    // Give enough antimatter to buy AD1 (cost 10)
    game.antimatter = Decimal::from_float(100.0);
    // Enable AD1 autobuyer
    game.autobuyers.dimensions[0].enabled = true;
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

    // Tick for enough time to trigger the autobuyer (1000ms interval)
    game.tick(1000.0);

    // Should have bought one AD1
    assert_eq!(game.dimensions[0].bought, 1);
}

#[test]
fn test_dimension_autobuyer_buys_max() {
    let mut game = GameState::new();
    // Give enough antimatter to buy multiple AD1s (cost 10, then 10000)
    game.antimatter = Decimal::from_float(1e6);
    game.autobuyers.dimensions[0].enabled = true;
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuyMax;

    // Tick for enough time to trigger the autobuyer
    game.tick(1000.0);

    // Should have bought multiple AD1s
    assert!(game.dimensions[0].bought >= 2);
}

#[test]
fn test_dimension_autobuyer_does_not_buy_locked_dimension() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    // Enable autobuyer for 5th dimension (index 4), which is locked
    game.autobuyers.dimensions[4].enabled = true;
    game.autobuyers.dimensions[4].mode = AutobuyerMode::BuyMax;

    game.tick(1000.0);

    // Should not have bought any since tier 5 is locked
    assert_eq!(game.dimensions[4].bought, 0);
}

#[test]
fn test_dimension_autobuyer_does_not_fire_before_interval() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(100.0);
    game.autobuyers.dimensions[0].enabled = true;

    // Tick for less than the interval (default 1000ms)
    game.tick(500.0);

    // Should not have bought anything yet
    assert_eq!(game.dimensions[0].bought, 0);
}

#[test]
fn test_multiple_dimension_autobuyers() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e10);

    // Enable autobuyers for dims 1-4
    for i in 0..4 {
        game.autobuyers.dimensions[i].enabled = true;
        game.autobuyers.dimensions[i].mode = AutobuyerMode::BuySingle;
    }

    game.tick(1000.0);

    // Each unlocked dimension autobuyer should have fired once
    for i in 0..4 {
        assert_eq!(
            game.dimensions[i].bought,
            1,
            "Dimension {} should have 1 bought",
            i + 1
        );
    }
}

// ============================================================
// Tickspeed autobuyer integration tests
// ============================================================

#[test]
fn test_tickspeed_autobuyer_buys_single() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    game.autobuyers.tickspeed.enabled = true;
    game.autobuyers.tickspeed.mode = AutobuyerMode::BuySingle;

    game.tick(1000.0);

    assert_eq!(game.tickspeed.bought, 1);
}

#[test]
fn test_tickspeed_autobuyer_buys_max() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e10);
    game.autobuyers.tickspeed.enabled = true;
    game.autobuyers.tickspeed.mode = AutobuyerMode::BuyMax;

    game.tick(1000.0);

    // Should have bought multiple tickspeed upgrades
    assert!(game.tickspeed.bought >= 2);
}

#[test]
fn test_tickspeed_autobuyer_does_not_fire_before_interval() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    game.autobuyers.tickspeed.enabled = true;

    game.tick(500.0);

    assert_eq!(game.tickspeed.bought, 0);
}

// ============================================================
// Autobuyer with simulation tests
// ============================================================

#[test]
fn test_autobuyers_fire_during_simulation() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    game.autobuyers.dimensions[0].enabled = true;
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

    // Simulate 5 seconds at 100ms ticks (50 ticks, autobuyer fires every 1000ms = 5 times)
    game.simulate(5000.0, 100.0);

    assert_eq!(game.dimensions[0].bought, 5);
}

#[test]
fn test_autobuyer_interval_change() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    game.autobuyers.dimensions[0].enabled = true;
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;
    // Set faster interval
    game.autobuyers.dimensions[0].interval_ms = 500.0;

    // Simulate 2 seconds at 100ms ticks (autobuyer fires every 500ms = 4 times)
    game.simulate(2000.0, 100.0);

    assert_eq!(game.dimensions[0].bought, 4);
}
