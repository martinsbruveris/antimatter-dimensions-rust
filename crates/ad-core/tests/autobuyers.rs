use ad_core::{Autobuyer, AutobuyerMode, GameState};
use break_infinity::Decimal;

// Helper: unlock an AD autobuyer regardless of the antimatter requirement,
// so timing/firing behaviour can be tested in isolation.
fn unlock_ad(game: &mut GameState, tier: usize) {
    game.autobuyers.dimensions[tier].is_bought = true;
}

// ============================================================
// Basic autobuyer state tests
// ============================================================

#[test]
fn test_autobuyers_locked_by_default() {
    let game = GameState::new();
    // The global switch is on, but no individual autobuyer is unlocked yet.
    assert!(game.autobuyers.enabled);
    for i in 0..8 {
        assert!(!game.autobuyers.dimensions[i].is_bought);
        assert!(game.autobuyers.dimensions[i].is_active);
    }
    assert!(!game.autobuyers.tickspeed.is_bought);
}

#[test]
fn test_autobuyer_default_modes() {
    let game = GameState::new();
    // AD autobuyers default to "Buys max" (BUY_10); tickspeed to single.
    for i in 0..8 {
        assert_eq!(game.autobuyers.dimensions[i].mode, AutobuyerMode::BuyMax);
    }
    assert_eq!(game.autobuyers.tickspeed.mode, AutobuyerMode::BuySingle);
}

// ============================================================
// Unlock gating tests
// ============================================================

#[test]
fn test_tab_unlocks_at_1e40_total_antimatter() {
    let mut game = GameState::new();
    assert!(!game.autobuyer_tab_unlocked());
    game.total_antimatter = Decimal::new(1.0, 40);
    assert!(game.autobuyer_tab_unlocked());
}

#[test]
fn test_ad_autobuyer_unlock_requires_threshold_and_costs_nothing() {
    let mut game = GameState::new();
    let am_before = game.antimatter;

    // Below the 1e40 requirement for tier 0: cannot unlock.
    game.total_antimatter = Decimal::new(1.0, 39);
    assert!(!game.can_unlock_ad_autobuyer(0));
    assert!(!game.unlock_ad_autobuyer(0));
    assert!(!game.autobuyers.dimensions[0].is_bought);

    // At the requirement: unlocking succeeds and spends no antimatter.
    game.total_antimatter = Decimal::new(1.0, 40);
    assert!(game.can_unlock_ad_autobuyer(0));
    assert!(game.unlock_ad_autobuyer(0));
    assert!(game.autobuyers.dimensions[0].is_bought);
    assert_eq!(game.antimatter, am_before);
}

#[test]
fn test_ad_autobuyer_requirements_scale_by_tier() {
    let mut game = GameState::new();
    // 1e70 meets tiers 0..=3 (1e40..1e70) but not tier 4 (1e80).
    game.total_antimatter = Decimal::new(1.0, 70);
    assert!(game.can_unlock_ad_autobuyer(3));
    assert!(!game.can_unlock_ad_autobuyer(4));
}

#[test]
fn test_tickspeed_autobuyer_unlocks_at_1e140() {
    let mut game = GameState::new();
    game.total_antimatter = Decimal::new(1.0, 139);
    assert!(!game.can_unlock_tickspeed_autobuyer());
    assert!(!game.unlock_tickspeed_autobuyer());

    game.total_antimatter = Decimal::new(1.0, 140);
    assert!(game.unlock_tickspeed_autobuyer());
    assert!(game.autobuyers.tickspeed.is_bought);
}

#[test]
fn test_toggle_helpers() {
    let mut game = GameState::new();
    game.toggle_ad_autobuyer(0);
    assert!(!game.autobuyers.dimensions[0].is_active);
    game.toggle_ad_autobuyer_mode(0);
    assert_eq!(game.autobuyers.dimensions[0].mode, AutobuyerMode::BuySingle);
    game.toggle_autobuyers();
    assert!(!game.autobuyers.enabled);
}

// ============================================================
// Autobuyer timer tests
// ============================================================

#[test]
fn test_autobuyer_does_not_fire_when_locked() {
    // Not unlocked -> never fires, even past the interval.
    let mut ab = Autobuyer::new(1000.0, AutobuyerMode::BuySingle);
    assert!(!ab.is_bought);
    assert!(!fire(&mut ab, 2000.0));
}

#[test]
fn test_autobuyer_does_not_fire_when_inactive() {
    let mut ab = Autobuyer::new(1000.0, AutobuyerMode::BuySingle);
    ab.is_bought = true;
    ab.is_active = false;
    assert!(!fire(&mut ab, 2000.0));
}

#[test]
fn test_autobuyer_fires_after_interval() {
    let mut ab = Autobuyer::new(1000.0, AutobuyerMode::BuySingle);
    ab.is_bought = true;
    assert!(!fire(&mut ab, 500.0));
    assert!(fire(&mut ab, 500.0));
}

#[test]
fn test_autobuyer_timer_resets_after_firing() {
    let mut ab = Autobuyer::new(1000.0, AutobuyerMode::BuySingle);
    ab.is_bought = true;
    assert!(fire(&mut ab, 1200.0));
    assert!(!fire(&mut ab, 700.0));
    assert!(fire(&mut ab, 100.0));
}

// `advance` is private; exercise it via the game tick instead. This tiny
// wrapper drives a standalone Autobuyer through one tick of the engine by
// reusing the same timer logic the engine uses.
fn fire(ab: &mut Autobuyer, dt_ms: f64) -> bool {
    let mut game = GameState::new();
    game.antimatter = Decimal::new(1.0, 30);
    game.autobuyers.dimensions[0] = ab.clone();
    let before = game.dimensions[0].bought;
    game.tick(dt_ms);
    *ab = game.autobuyers.dimensions[0].clone();
    game.dimensions[0].bought > before
}

// ============================================================
// Dimension autobuyer integration tests
// ============================================================

#[test]
fn test_dimension_autobuyer_buys_single() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(100.0);
    unlock_ad(&mut game, 0);
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

    // AD1 autobuyer interval is 500 ms.
    game.tick(500.0);

    assert_eq!(game.dimensions[0].bought, 1);
}

#[test]
fn test_dimension_autobuyer_buys_max_fills_group_of_ten() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    unlock_ad(&mut game, 0);
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuyMax;

    game.tick(500.0);

    // "Buys max" (BUY_10) fills the current group of ten in one tick.
    assert_eq!(game.dimensions[0].bought, 10);
}

#[test]
fn test_dimension_autobuyer_does_not_buy_locked_dimension() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    unlock_ad(&mut game, 4);
    game.autobuyers.dimensions[4].mode = AutobuyerMode::BuyMax;

    game.tick(1000.0);

    // Tier 5 (index 4) is not unlocked as a dimension, so nothing is bought.
    assert_eq!(game.dimensions[4].bought, 0);
}

#[test]
fn test_dimension_autobuyer_does_not_fire_before_interval() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(100.0);
    unlock_ad(&mut game, 0);
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

    // Less than the 500 ms interval.
    game.tick(400.0);

    assert_eq!(game.dimensions[0].bought, 0);
}

#[test]
fn test_multiple_dimension_autobuyers() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e10);

    for i in 0..4 {
        unlock_ad(&mut game, i);
        game.autobuyers.dimensions[i].mode = AutobuyerMode::BuySingle;
    }

    // 1000 ms exceeds every AD interval (500..=800 ms), so each fires once.
    game.tick(1000.0);

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
    game.autobuyers.tickspeed.is_bought = true;
    game.autobuyers.tickspeed.mode = AutobuyerMode::BuySingle;

    // Tickspeed autobuyer interval is 500 ms.
    game.tick(500.0);

    assert_eq!(game.tickspeed.bought, 1);
}

#[test]
fn test_tickspeed_autobuyer_buys_max() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e10);
    game.autobuyers.tickspeed.is_bought = true;
    game.autobuyers.tickspeed.mode = AutobuyerMode::BuyMax;

    game.tick(500.0);

    assert!(game.tickspeed.bought >= 2);
}

#[test]
fn test_tickspeed_autobuyer_does_not_fire_before_interval() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    game.autobuyers.tickspeed.is_bought = true;

    game.tick(400.0);

    assert_eq!(game.tickspeed.bought, 0);
}

// ============================================================
// Global switch + simulation tests
// ============================================================

#[test]
fn test_global_switch_off_blocks_firing() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    unlock_ad(&mut game, 0);
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;
    game.autobuyers.enabled = false;

    game.tick(1000.0);

    assert_eq!(game.dimensions[0].bought, 0);
}

#[test]
fn test_autobuyers_fire_during_simulation() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    unlock_ad(&mut game, 0);
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

    // 5000 ms at 100 ms ticks; AD1 interval is 500 ms -> fires 10 times.
    game.simulate(5000.0, 100.0);

    assert_eq!(game.dimensions[0].bought, 10);
}

#[test]
fn test_autobuyer_interval_change() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    unlock_ad(&mut game, 0);
    game.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;
    game.autobuyers.dimensions[0].interval_ms = 500.0;

    // 2000 ms at 100 ms ticks; fires every 500 ms -> 4 times.
    game.simulate(2000.0, 100.0);

    assert_eq!(game.dimensions[0].bought, 4);
}
