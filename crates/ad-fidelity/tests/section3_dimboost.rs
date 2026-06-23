//! Fidelity tests for dimension boost multiplier (Section 3).
//!
//! Formula: DimBoost.power^max(0, boosts + 1 - tier) where
//! tier is 1-indexed. In our 0-indexed system:
//!   exponent = max(0, boosts - tier)

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{
    assert_approx_eq, assert_u32_eq, assert_u64_eq, EPSILON_EXACT,
};

/// Verify tier-dependent dim boost multiplier with 1 boost.
#[test]
fn dimboost_multiplier_1_boost() {
    let mut state = GameState::new();
    state.dim_boosts = 1;

    // tier=0 (AD1): exponent = max(0, 1-0) = 1, mult = 2^1 = 2
    // plus buy10=1 → total = 2
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "dimboost mult tier=0, boosts=1",
    );

    // tier=1 (AD2): exponent = max(0, 1-1) = 0, mult = 1
    let mult = state.dimension_multiplier(1);
    assert_approx_eq(
        mult,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "dimboost mult tier=1, boosts=1",
    );

    // tier=2 (AD3): exponent = max(0, 1-2) = 0, mult = 1
    let mult = state.dimension_multiplier(2);
    assert_approx_eq(
        mult,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "dimboost mult tier=2, boosts=1",
    );
}

/// Verify tier-dependent dim boost multiplier with 4 boosts.
#[test]
fn dimboost_multiplier_4_boosts() {
    let mut state = GameState::new();
    state.dim_boosts = 4;

    // tier=0: 2^4 = 16
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(16.0),
        EPSILON_EXACT,
        "dimboost mult tier=0, boosts=4",
    );

    // tier=1: 2^3 = 8
    let mult = state.dimension_multiplier(1);
    assert_approx_eq(
        mult,
        Decimal::from_float(8.0),
        EPSILON_EXACT,
        "dimboost mult tier=1, boosts=4",
    );

    // tier=2: 2^2 = 4
    let mult = state.dimension_multiplier(2);
    assert_approx_eq(
        mult,
        Decimal::from_float(4.0),
        EPSILON_EXACT,
        "dimboost mult tier=2, boosts=4",
    );

    // tier=3: 2^1 = 2
    let mult = state.dimension_multiplier(3);
    assert_approx_eq(
        mult,
        Decimal::from_float(2.0),
        EPSILON_EXACT,
        "dimboost mult tier=3, boosts=4",
    );

    // tier=4: 2^0 = 1
    let mult = state.dimension_multiplier(4);
    assert_approx_eq(
        mult,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "dimboost mult tier=4, boosts=4",
    );
}

/// Verify tier-dependent dim boost multiplier with 10 boosts.
#[test]
fn dimboost_multiplier_10_boosts() {
    let mut state = GameState::new();
    state.dim_boosts = 10;

    // tier=0: 2^10 = 1024
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(1024.0),
        EPSILON_EXACT,
        "dimboost mult tier=0, boosts=10",
    );

    // tier=4: 2^6 = 64
    let mult = state.dimension_multiplier(4);
    assert_approx_eq(
        mult,
        Decimal::from_float(64.0),
        EPSILON_EXACT,
        "dimboost mult tier=4, boosts=10",
    );

    // tier=7: 2^3 = 8
    let mult = state.dimension_multiplier(7);
    // Also includes sacrifice_multiplier which is 1.0 by default
    assert_approx_eq(
        mult,
        Decimal::from_float(8.0),
        EPSILON_EXACT,
        "dimboost mult tier=7, boosts=10",
    );
}

/// Verify combined buy-10 and dim boost multiplier.
#[test]
fn dimboost_combined_with_buy10() {
    let mut state = GameState::new();
    state.dim_boosts = 4;
    state.dimensions[0].bought = 20; // buy10 = 2^2 = 4

    // AD1: buy10(4) × dimboost(2^4=16) = 64
    let mult = state.dimension_multiplier(0);
    assert_approx_eq(
        mult,
        Decimal::from_float(64.0),
        EPSILON_EXACT,
        "AD1 mult: buy10=4, dimboost=16",
    );
}

/// Verify dim boost requirements (first 4 boosts).
#[test]
fn dimboost_requirements_first_4() {
    let mut state = GameState::new();

    // Boost 0→1: requires tier 4 (index 3), amount 20
    state.dim_boosts = 0;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 3, "Boost 1 requires tier index 3 (AD4)");
    assert_eq!(amount, 20, "Boost 1 requires 20");

    // Boost 1→2: requires tier 5 (index 4), amount 20
    state.dim_boosts = 1;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 4, "Boost 2 requires tier index 4 (AD5)");
    assert_eq!(amount, 20, "Boost 2 requires 20");

    // Boost 2→3: requires tier 6 (index 5), amount 20
    state.dim_boosts = 2;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 5, "Boost 3 requires tier index 5 (AD6)");
    assert_eq!(amount, 20, "Boost 3 requires 20");

    // Boost 3→4: requires tier 7 (index 6), amount 20
    state.dim_boosts = 3;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 6, "Boost 4 requires tier index 6 (AD7)");
    assert_eq!(amount, 20, "Boost 4 requires 20");
}

/// Verify dim boost requirements (5+ boosts, require AD8).
#[test]
fn dimboost_requirements_scaling() {
    let mut state = GameState::new();

    // Boost 4→5: tier 8 (index 7), amount = 20 + 0*15 = 20
    state.dim_boosts = 4;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 7, "Boost 5 requires AD8");
    assert_u64_eq(amount, 20, "Boost 5 requires 20 AD8");

    // Boost 5→6: amount = 20 + 1*15 = 35
    state.dim_boosts = 5;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 7, "Boost 6 requires AD8");
    assert_u64_eq(amount, 35, "Boost 6 requires 35 AD8");

    // Boost 10→11: amount = 20 + 6*15 = 110
    state.dim_boosts = 10;
    let (tier, amount) = state.dim_boost_requirement();
    assert_eq!(tier, 7, "Boost 11 requires AD8");
    assert_u64_eq(amount, 110, "Boost 11 requires 110 AD8");
}

/// Verify dim boost reset behavior.
#[test]
fn dimboost_reset() {
    let mut state = GameState::new();
    state.antimatter = Decimal::from_float(1e30);

    // Set up state where we can boost
    state.dim_boosts = 0;
    state.dimensions[3].amount = Decimal::from_float(25.0);
    state.dimensions[3].bought = 20;
    state.dimensions[0].amount = Decimal::from_float(1000.0);
    state.dimensions[0].bought = 50;
    state.tickspeed.bought = 10;
    state.galaxies = 2;

    assert!(state.buy_dim_boost());

    assert_u32_eq(state.dim_boosts, 1, "dim_boosts after boost");
    assert_u32_eq(state.galaxies, 2, "galaxies preserved after boost");

    // All dimensions reset
    assert_eq!(state.dimensions[0].bought, 0);
    assert_eq!(state.dimensions[0].amount, Decimal::ZERO);
    assert_eq!(state.dimensions[3].bought, 0);

    // Tickspeed reset
    assert_eq!(state.tickspeed.bought, 0);

    // Antimatter reset to 10
    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(10.0),
        EPSILON_EXACT,
        "antimatter reset to 10 after dim boost",
    );
}
