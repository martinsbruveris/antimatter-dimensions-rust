//! Fidelity tests for buy-10 multiplier (Section 2).
//!
//! Every 10 purchases of a dimension grants a 2x production
//! multiplier to that dimension.

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{assert_approx_eq, EPSILON_EXACT};

/// Verify the buy-10 multiplier at various bought counts.
#[test]
fn buy10_multiplier_basic() {
    let mut state = GameState::new();

    // bought=0: multiplier contribution from buy10 = 2^0 = 1
    state.dimensions[0].bought = 0;
    let mult = state.dimension_multiplier(0);
    // With 0 boosts, dimboost_mult(tier=0) = 2^max(0, 0-0) = 2^0 = 1
    // buy10 = 2^0 = 1
    // total = 1 × 1 = 1
    assert_approx_eq(
        mult,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "AD1 multiplier at bought=0, no boosts",
    );

    // bought=10: buy10 = 2^1 = 2
    state.dimensions[0].bought = 10;
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "AD1 multiplier at bought=10, no boosts",
    );

    // bought=30: buy10 = 2^3 = 8
    state.dimensions[0].bought = 30;
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(8.0),
        EPSILON_EXACT,
        "AD1 multiplier at bought=30, no boosts",
    );

    // bought=100: buy10 = 2^10 = 1024
    state.dimensions[0].bought = 100;
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(1024.0),
        EPSILON_EXACT,
        "AD1 multiplier at bought=100, no boosts",
    );
}

/// Verify buy-10 multiplier contributes to production rate.
#[test]
fn buy10_in_production() {
    let mut state = GameState::new();

    // AD1: amount=10, bought=10 → buy10_mult=2
    // No boosts, no galaxies, no tickspeed
    // dimboost_mult(tier=0) = 2^max(0, 0-0) = 1
    // multiplier = buy10(2) × dimboost(1) = 2
    // tickspeed_effect = 1000/1000 = 1
    // production = 10 × 2 × 1 = 20 AM/s
    state.dimensions[0].amount = Decimal::from_float(10.0);
    state.dimensions[0].bought = 10;

    let production = state.dimension_production_per_second(0);
    assert_approx_eq(
        production,
        Decimal::from_float(20.0),
        EPSILON_EXACT,
        "AD1 production with buy10=2, no boosts/tickspeed",
    );
}

/// Verify that different tiers independently track their buy-10
/// multiplier.
#[test]
fn buy10_per_tier_independence() {
    let mut state = GameState::new();
    state.dim_boosts = 4; // Unlock all tiers

    // AD1: bought=20, buy10=2^2=4
    state.dimensions[0].bought = 20;
    state.dimensions[0].amount = Decimal::from_float(5.0);

    // AD3: bought=10, buy10=2^1=2
    state.dimensions[2].bought = 10;
    state.dimensions[2].amount = Decimal::from_float(3.0);

    // AD1 multiplier: buy10(4) × dimboost(2^(4-0)=16) = 64
    let mult1 = state.dimension_multiplier(0);
    assert_approx_eq(
        mult1,
        Decimal::from_float(64.0),
        EPSILON_EXACT,
        "AD1 mult: buy10=4, dimboost=16",
    );

    // AD3 multiplier: buy10(2) × dimboost(2^(4-2)=4) = 8
    let mult3 = state.dimension_multiplier(2);
    assert_approx_eq(
        mult3,
        Decimal::from_float(8.0),
        EPSILON_EXACT,
        "AD3 mult: buy10=2, dimboost=4",
    );
}
