//! Fidelity tests for production simulation (Sections 7-8).
//!
//! Tests the production chain: AD8→AD7→...→AD1→AM
//! Each dimension produces the tier below it, scaled by
//! multiplier and tickspeed effect.

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{assert_approx_eq, EPSILON_EXACT};

/// Single dimension production (AD1 → AM).
#[test]
fn single_dimension_production() {
    let mut state = GameState::new();

    // AD1: amount=1, bought=0, no tickspeed/boosts
    // multiplier = buy10(1) × dimboost(1) = 1
    // tickspeed_effect = 1
    // production = 1 × 1 × 1 = 1 AM/s
    state.dimensions[0].amount = Decimal::from_float(1.0);
    state.antimatter = Decimal::ZERO;

    state.tick(1000.0);

    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "1 AD1 produces 1 AM in 1 second",
    );
}

/// Two-dimension chain (AD2 → AD1 → AM).
#[test]
fn two_dimension_chain() {
    let mut state = GameState::new();

    // AD1=1, AD2=1, no boosts/tickspeed
    // In one tick (1s):
    //   AM += AD1_prod = 1 × 1 × 1 = 1
    //   AD1 += AD2_prod = 1 × 1 × 1 = 1
    state.dimensions[0].amount = Decimal::from_float(1.0);
    state.dimensions[1].amount = Decimal::from_float(1.0);
    state.antimatter = Decimal::ZERO;

    state.tick(1000.0);

    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "AM after 1s with AD1=1, AD2=1",
    );
    assert_approx_eq(
        state.dimensions[0].amount,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "AD1 amount after 1s (produced by AD2)",
    );
}

/// Four-dimension chain in single tick.
#[test]
fn four_dimension_chain_single_tick() {
    let mut state = GameState::new();

    state.dimensions[0].amount = Decimal::from_float(1.0);
    state.dimensions[1].amount = Decimal::from_float(1.0);
    state.dimensions[2].amount = Decimal::from_float(1.0);
    state.dimensions[3].amount = Decimal::from_float(1.0);
    state.antimatter = Decimal::ZERO;

    state.tick(1000.0);

    // All production computed simultaneously for 1 second:
    // AM += 1 (from AD1=1)
    // AD1 += 1 (from AD2=1)
    // AD2 += 1 (from AD3=1)
    // AD3 += 1 (from AD4=1)
    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "AM after 1s with 4 dims",
    );
    assert_approx_eq(
        state.dimensions[0].amount,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "AD1 after 1s",
    );
    assert_approx_eq(
        state.dimensions[1].amount,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "AD2 after 1s",
    );
    assert_approx_eq(
        state.dimensions[2].amount,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "AD3 after 1s",
    );
}

/// Production with tickspeed upgrades.
#[test]
fn production_with_tickspeed() {
    let mut state = GameState::new();
    state.galaxies = 0;
    state.tickspeed.bought = 5;
    state.dimensions[0].amount = Decimal::from_float(10.0);
    state.dimensions[0].bought = 10; // buy10 = 2

    // multiplier = buy10(2) × dimboost(1) = 2
    // tickspeed_mult per purchase = 1/1.1245
    // tickspeed_effect = 1 / (1/1.1245)^5 = 1.1245^5
    let ts_mult = 1.0 / 1.1245_f64;
    let effect = 1.0 / ts_mult.powi(5);
    let expected_production = 10.0 * 2.0 * effect;

    state.antimatter = Decimal::ZERO;
    state.tick(1000.0);

    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(expected_production),
        1e-9,
        "production with 5 tickspeed purchases",
    );
}

/// Two-dimension exponential growth over 60 seconds with small
/// ticks.
///
/// AD2=1 produces 1 AD1/s. AD1 produces AM. Growth is quadratic:
///   AD1(t) = 1 + t (seconds)
///   AM(t) = integral(1+t)dt from 0..60 = 60 + 60^2/2 = 1860
/// (continuous approximation; discrete ticks are close with small
/// dt)
#[test]
fn two_dim_exponential_growth_60s() {
    let mut state = GameState::new();
    state.dimensions[0].amount = Decimal::from_float(1.0);
    state.dimensions[1].amount = Decimal::from_float(1.0);
    state.antimatter = Decimal::ZERO;

    // 60 seconds in 100ms ticks
    state.simulate(60_000.0, 100.0);

    // Continuous approximation: AM ≈ 60 + 1800 = 1860
    // Discrete with 100ms ticks: slightly different due to
    // Euler method, but close.
    let am = state.antimatter.to_f64();
    // Allow ~1% tolerance for discrete vs continuous
    assert!(
        (am - 1860.0).abs() / 1860.0 < 0.01,
        "AM after 60s should be ~1860, got {am}"
    );
}

/// Full production chain with dim boosts and tickspeed over 10
/// seconds.
#[test]
fn full_chain_with_boosts_10s() {
    let mut state = GameState::new();
    state.dim_boosts = 4; // Unlock all 8 dims

    // Start each dim with amount=1
    for i in 0..8 {
        state.dimensions[i].amount = Decimal::from_float(1.0);
    }
    state.antimatter = Decimal::ZERO;

    // With 4 boosts:
    // AD1 mult = 2^max(0, 4-0) = 16
    // AD2 mult = 2^max(0, 4-1) = 8
    // AD3 mult = 2^max(0, 4-2) = 4
    // AD4 mult = 2^max(0, 4-3) = 2
    // AD5-8 mult = 2^0 = 1
    // tickspeed_effect = 1 (no upgrades)

    // Run 10s with 50ms ticks
    state.simulate(10_000.0, 50.0);

    // Verify AM is significantly greater than 0 (exponential
    // growth with 8 chained dims)
    let am = state.antimatter.to_f64();
    assert!(
        am > 100.0,
        "AM should be substantial after 10s with full chain, \
         got {am}"
    );

    // Each tier should have grown
    for i in 0..7 {
        let amount = state.dimensions[i].amount.to_f64();
        assert!(amount > 1.0, "AD{} should have grown, got {amount}", i + 1);
    }
}

/// Verify that production is zero for locked dimensions.
#[test]
fn locked_dimensions_no_production() {
    let mut state = GameState::new();
    // Default: only 4 dims unlocked (no boosts)

    state.dimensions[4].amount = Decimal::from_float(100.0);
    state.dimensions[4].bought = 50;

    // AD5 (index 4) is locked — shouldn't produce into AD4
    let prod = state.dimension_production_per_second(4);
    assert_eq!(prod, Decimal::ZERO, "locked dimension should not produce");
}

/// Verify dim boost multiplier applies to production correctly.
#[test]
fn production_with_dimboost_multiplier() {
    let mut state = GameState::new();
    state.dim_boosts = 4;

    // AD1: amount=10, bought=0 (buy10=1)
    // dimboost_mult(tier=0) = 2^4 = 16
    // tickspeed_effect = 1
    // production = 10 × 16 × 1 = 160 AM/s
    state.dimensions[0].amount = Decimal::from_float(10.0);
    state.antimatter = Decimal::ZERO;

    state.tick(1000.0);

    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(160.0),
        EPSILON_EXACT,
        "AD1 production with 4 dim boosts",
    );
}

/// Verify sacrifice multiplier applies to AD8 production.
#[test]
fn production_with_sacrifice_multiplier() {
    let mut state = GameState::new();
    state.dim_boosts = 4; // Unlock all dims

    // Set sacrifice_boost = 100
    state.sacrifice_boost = Decimal::from_float(100.0);

    // AD8: amount=5, bought=0 (buy10=1)
    // dimboost_mult(tier=7) = 2^max(0, 4-7) = 2^0 = 1
    // sacrifice_mult = 100
    // total mult = 1 × 100 = 100
    // production into AD7 = 5 × 100 × 1 = 500/s
    state.dimensions[7].amount = Decimal::from_float(5.0);
    state.dimensions[6].amount = Decimal::ZERO;

    state.tick(1000.0);

    assert_approx_eq(
        state.dimensions[6].amount,
        Decimal::from_float(500.0),
        EPSILON_EXACT,
        "AD8 produces into AD7 with sacrifice mult",
    );
}
