use ad_core::GameState;
use break_infinity::Decimal;

// ============================================================
// Basic game state tests
// ============================================================

#[test]
fn test_new_game_starts_with_10_antimatter() {
    let game = GameState::new();
    assert_eq!(game.antimatter, Decimal::from_float(10.0));
}

#[test]
fn test_new_game_has_4_unlocked_dimensions() {
    let game = GameState::new();
    assert_eq!(game.unlocked_dimensions(), 4);
    assert!(game.is_dimension_unlocked(0));
    assert!(game.is_dimension_unlocked(3));
    assert!(!game.is_dimension_unlocked(4));
}

// ============================================================
// Dimension purchasing tests
// ============================================================

#[test]
fn test_buy_dimension_spends_antimatter() {
    let mut game = GameState::new();
    // AD1 costs 10, we have 10
    assert!(game.buy_dimension(0));
    assert_eq!(game.antimatter, Decimal::from_float(0.0));
    assert_eq!(game.dimensions[0].bought, 1);
    assert_eq!(game.dimensions[0].amount, Decimal::from_float(1.0));
}

#[test]
fn test_cannot_buy_dimension_without_antimatter() {
    let mut game = GameState::new();
    game.buy_dimension(0); // spend 10 AM (now have 0)
    assert!(!game.buy_dimension(0)); // can't afford (cost still 10, but 0 AM)
}

#[test]
fn test_cannot_buy_locked_dimension() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    // Dimension 5 (index 4) is locked at start
    assert!(!game.buy_dimension(4));
}

#[test]
fn test_cost_increases_after_10_purchases() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);

    // AD1 base cost is 10, cost multiplier is 1e3
    // Purchases 1-10 should all cost 10
    let cost_first = game.dimension_cost(0);
    assert_eq!(cost_first, Decimal::from_float(10.0));

    for _ in 0..10 {
        game.buy_dimension(0);
    }

    // After 10 purchases, cost should jump to 10 * 1e3 = 10000
    let cost_after_10 = game.dimension_cost(0);
    assert_eq!(cost_after_10, Decimal::from_float(10_000.0));
}

#[test]
fn test_buy_max_dimension() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    let bought = game.buy_max_dimension(0);
    // With per-10 pricing: first 10 cost 10 each (100 total),
    // next 10 cost 10000 each (100000 total), etc.
    // Should buy at least 20
    assert!(bought >= 20);
    assert_eq!(game.dimensions[0].bought, bought);
}

// ============================================================
// Production / tick tests
// ============================================================

#[test]
fn test_ad1_produces_antimatter() {
    let mut game = GameState::new();
    game.buy_dimension(0); // 1 AD1, 0 antimatter
    game.tick(1000.0); // 1 second
                       // Production = 1 * 1.0 (mult) * 1.0 (tickspeed_effect) * 1s = 1
    assert_eq!(game.antimatter, Decimal::from_float(1.0));
}

#[test]
fn test_ad2_produces_ad1() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    game.buy_dimension(0); // buy 1 AD1
    game.buy_dimension(1); // buy 1 AD2 (cost 100)

    let ad1_before = game.dimensions[0].amount;
    game.tick(1000.0); // 1 second

    // AD2 should have produced into AD1
    assert!(game.dimensions[0].amount > ad1_before);
}

#[test]
fn test_production_chain_all_4_dims() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e15);

    // Buy 1 of each unlocked dimension
    for i in 0..4 {
        game.buy_dimension(i);
    }

    let am_before = game.antimatter;
    game.tick(1000.0);

    // Antimatter should increase (AD1 produces it)
    assert!(game.antimatter > am_before);
    // AD1 amount should increase (AD2 produces it)
    assert!(game.dimensions[0].amount > Decimal::from_float(1.0));
}

// ============================================================
// Tickspeed tests
// ============================================================

#[test]
fn test_initial_tickspeed() {
    let game = GameState::new();
    assert_eq!(game.current_tickspeed_ms(), 1000.0);
    assert_eq!(game.tickspeed_effect(), Decimal::from_float(1.0));
}

#[test]
fn test_buy_tickspeed() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    assert!(game.buy_tickspeed());
    assert_eq!(game.tickspeed.bought, 1);
    // Tickspeed should be faster (less than 1000ms)
    assert!(game.current_tickspeed_ms() < 1000.0);
}

#[test]
fn test_tickspeed_increases_production() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e6);
    game.buy_dimension(0);

    // Measure production rate without tickspeed
    let prod_without = game.dimension_production_per_second(0);

    // Buy tickspeed and check production rate is higher
    game.buy_tickspeed();
    let prod_with = game.dimension_production_per_second(0);

    assert!(prod_with > prod_without);
}

// ============================================================
// Dimension boost tests
// ============================================================

#[test]
fn test_dim_boost_requirement_first() {
    let game = GameState::new();
    // First boost requires 20 of dim 4 (index 3)
    let (tier, amount) = game.dim_boost_requirement();
    assert_eq!(tier, 3);
    assert_eq!(amount, 20);
}

#[test]
fn test_cannot_dim_boost_initially() {
    let game = GameState::new();
    assert!(!game.can_dim_boost());
}

#[test]
fn test_dim_boost_unlocks_5th_dimension() {
    let mut game = GameState::new();
    // Set 20 amount on 4th dimension (index 3) to meet requirement
    game.dimensions[3].amount = Decimal::from_float(20.0);

    assert!(game.can_dim_boost());
    assert!(game.buy_dim_boost());
    assert_eq!(game.dim_boosts, 1);
    assert_eq!(game.unlocked_dimensions(), 5);
    assert!(game.is_dimension_unlocked(4));
}

#[test]
fn test_dim_boost_resets_dimensions() {
    let mut game = GameState::new();
    // Set up enough to dim boost (need 20 amount of dim 4)
    game.dimensions[3].amount = Decimal::from_float(20.0);
    game.dimensions[0].bought = 5;
    game.dimensions[0].amount = Decimal::from_float(10.0);

    game.buy_dim_boost();

    // Dimensions should be reset
    assert_eq!(game.dimensions[0].bought, 0);
    assert_eq!(game.dimensions[0].amount, Decimal::from_float(0.0));
    // Antimatter reset to 10
    assert_eq!(game.antimatter, Decimal::from_float(10.0));
}

#[test]
fn test_dim_boost_multiplier_effect() {
    let mut game = GameState::new();
    // No boosts: multiplier should be 1.0 for all tiers
    assert_eq!(game.dimension_multiplier(0), Decimal::from_float(1.0));

    // 1 boost: AD1 (tier 0) = 2^max(0, 1-0) = 2^1 = 2
    game.dim_boosts = 1;
    assert_eq!(game.dimension_multiplier(0), Decimal::from_float(2.0));
    // AD2 (tier 1) = 2^max(0, 1-1) = 2^0 = 1
    assert_eq!(game.dimension_multiplier(1), Decimal::from_float(1.0));

    // 4 boosts: AD1 = 2^4 = 16, AD4 = 2^1 = 2, AD5 = 2^0 = 1
    game.dim_boosts = 4;
    assert_eq!(game.dimension_multiplier(0), Decimal::from_float(16.0));
    assert_eq!(game.dimension_multiplier(3), Decimal::from_float(2.0));
    assert_eq!(game.dimension_multiplier(4), Decimal::from_float(1.0));
}

// ============================================================
// Galaxy tests
// ============================================================

#[test]
fn test_galaxy_requirement_first() {
    let game = GameState::new();
    assert_eq!(game.galaxy_requirement(), 80);
}

#[test]
fn test_galaxy_requirement_subsequent() {
    let mut game = GameState::new();
    game.galaxies = 1;
    assert_eq!(game.galaxy_requirement(), 140); // 80 + 60
    game.galaxies = 2;
    assert_eq!(game.galaxy_requirement(), 200); // 80 + 120
}

#[test]
fn test_cannot_buy_galaxy_initially() {
    let game = GameState::new();
    assert!(!game.can_buy_galaxy());
}

#[test]
fn test_galaxy_improves_tickspeed_multiplier() {
    let mut game = GameState::new();
    let mult_before = game.tickspeed_purchase_multiplier();

    game.galaxies = 1;
    let mult_after = game.tickspeed_purchase_multiplier();

    // Galaxy should reduce the multiplier (making tickspeed purchases more effective)
    assert!(mult_after < mult_before);
}

#[test]
fn test_galaxy_resets_state() {
    let mut game = GameState::new();
    game.antimatter = Decimal::from_float(1e30);
    game.dim_boosts = 5;
    game.dimensions[7].amount = Decimal::from_float(80.0);
    game.dimensions[0].amount = Decimal::from_float(1000.0);

    assert!(game.buy_galaxy());
    assert_eq!(game.galaxies, 1);
    assert_eq!(game.antimatter, Decimal::from_float(10.0));
    assert_eq!(game.dim_boosts, 0);
    assert_eq!(game.dimensions[0].amount, Decimal::from_float(0.0));
    assert_eq!(game.dimensions[0].bought, 0);
}

// ============================================================
// Sacrifice tests
// ============================================================

#[test]
fn test_sacrifice_not_available_initially() {
    let game = GameState::new();
    assert!(!game.can_sacrifice());
}

#[test]
fn test_sacrifice_requires_ad1_amount() {
    let mut game = GameState::new();
    game.sacrifice_unlocked = true;
    // No AD1 amount, can't sacrifice
    assert!(!game.can_sacrifice());
}

#[test]
fn test_sacrifice_resets_lower_dimensions() {
    let mut game = GameState::new();
    game.sacrifice_unlocked = true;
    game.dimensions[0].amount = Decimal::from_float(100.0);
    game.dimensions[1].amount = Decimal::from_float(50.0);
    game.dimensions[6].amount = Decimal::from_float(10.0);
    game.dimensions[7].amount = Decimal::from_float(5.0);

    assert!(game.sacrifice());

    // Dims 1-7 (indices 0-6) should be reset
    assert_eq!(game.dimensions[0].amount, Decimal::from_float(0.0));
    assert_eq!(game.dimensions[6].amount, Decimal::from_float(0.0));
    // Dim 8 (index 7) should be unchanged
    assert_eq!(game.dimensions[7].amount, Decimal::from_float(5.0));
    // Sacrificed total should be 100
    assert_eq!(game.sacrificed, Decimal::from_float(100.0));
}

#[test]
fn test_sacrifice_multiplier_increases_with_amount() {
    let mut game = GameState::new();
    game.sacrifice_unlocked = true;

    // First sacrifice with AD1 = 1e20
    game.dimensions[0].amount = Decimal::from_float(1e20);
    game.sacrifice();
    let mult1 = game.sacrifice_multiplier();

    // Second sacrifice with AD1 = 1e40
    game.dimensions[0].amount = Decimal::from_float(1e40);
    game.sacrifice();
    let mult2 = game.sacrifice_multiplier();

    assert!(mult2 > mult1);
    assert!(mult1 > Decimal::from_float(1.0));
}

// ============================================================
// Simulation test
// ============================================================

#[test]
fn test_simulate_produces_antimatter() {
    let mut game = GameState::new();
    game.buy_dimension(0); // Buy 1 AD1

    // Simulate 10 seconds at 100ms ticks
    game.simulate(10000.0, 100.0);

    // Should have produced ~10 antimatter (1 AD1 * 10s)
    let expected = Decimal::from_float(10.0);
    let tolerance = Decimal::from_float(0.1);
    assert!(game.antimatter.ge_tolerance(&expected, &tolerance));
}
