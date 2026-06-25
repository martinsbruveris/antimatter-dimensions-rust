//! Fidelity tests for antimatter galaxies (Section 5).
//!
//! Galaxy requirement: 80 + galaxies × 60 (AD8 amount)
//! Galaxy resets: all dims, tickspeed, dim boosts, sacrifice

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{
    assert_approx_eq, assert_u32_eq, assert_u64_eq, EPSILON_EXACT,
};

/// Verify galaxy requirement formula.
#[test]
fn galaxy_requirement_formula() {
    let mut state = GameState::new();

    // First galaxy: 80 AD8
    state.galaxies = 0;
    assert_u64_eq(
        state.galaxy_requirement(),
        80,
        "first galaxy requires 80 AD8",
    );

    // Second galaxy: 80 + 60 = 140
    state.galaxies = 1;
    assert_u64_eq(
        state.galaxy_requirement(),
        140,
        "second galaxy requires 140 AD8",
    );

    // Third galaxy: 80 + 120 = 200
    state.galaxies = 2;
    assert_u64_eq(
        state.galaxy_requirement(),
        200,
        "third galaxy requires 200 AD8",
    );

    // 6th galaxy: 80 + 300 = 380
    state.galaxies = 5;
    assert_u64_eq(
        state.galaxy_requirement(),
        380,
        "6th galaxy requires 380 AD8",
    );

    // 11th galaxy: 80 + 600 = 680
    state.galaxies = 10;
    assert_u64_eq(
        state.galaxy_requirement(),
        680,
        "11th galaxy requires 680 AD8",
    );
}

/// Verify galaxy can_buy check uses AD8 amount.
#[test]
fn galaxy_can_buy_check() {
    let mut state = GameState::new();
    state.galaxies = 0;
    state.dim_boosts = 4; // Need all dims unlocked

    // AD8 amount = 79: cannot buy
    state.dimensions[7].amount = Decimal::from_float(79.0);
    assert!(!state.can_buy_galaxy());

    // AD8 amount = 80: can buy
    state.dimensions[7].amount = Decimal::from_float(80.0);
    assert!(state.can_buy_galaxy());

    // AD8 amount = 80.5 (fractional): can buy (floor >= 80)
    state.dimensions[7].amount = Decimal::from_float(80.5);
    assert!(state.can_buy_galaxy());
}

/// Verify galaxy reset behavior.
#[test]
fn galaxy_reset_behavior() {
    let mut state = GameState::new();
    state.dim_boosts = 5;
    state.galaxies = 0;

    // Set up state to buy galaxy
    state.dimensions[7].amount = Decimal::from_float(85.0);
    state.dimensions[7].bought = 80;
    state.dimensions[0].amount = Decimal::from_float(1e50);
    state.dimensions[0].bought = 100;
    state.tickspeed.bought = 20;
    state.antimatter = Decimal::from_float(1e200);
    state.sacrificed = Decimal::from_float(1e50);

    assert!(state.buy_galaxy());

    // Galaxy count incremented
    assert_u32_eq(state.galaxies, 1, "galaxies after purchase");

    // Dim boosts reset
    assert_u32_eq(state.dim_boosts, 0, "dim_boosts reset after galaxy");

    // All dims reset
    assert_eq!(state.dimensions[7].bought, 0);
    assert_eq!(state.dimensions[7].amount, Decimal::ZERO);
    assert_eq!(state.dimensions[0].bought, 0);
    assert_eq!(state.dimensions[0].amount, Decimal::ZERO);

    // Tickspeed reset
    assert_eq!(state.tickspeed.bought, 0);

    // Antimatter reset to 10
    assert_approx_eq(
        state.antimatter,
        Decimal::from_float(10.0),
        EPSILON_EXACT,
        "antimatter reset to 10 after galaxy",
    );

    // Sacrificed reset
    assert_eq!(state.sacrificed, Decimal::ZERO);
    // sacrifice_multiplier resets to 1 (stateless: no sacrificed)
    assert_eq!(state.sacrifice_multiplier(), Decimal::ONE);
}

/// Verify consecutive galaxy purchases increase requirement.
#[test]
fn galaxy_consecutive_purchases() {
    let mut state = GameState::new();
    state.dim_boosts = 4;

    // Buy first galaxy
    state.dimensions[7].amount = Decimal::from_float(80.0);
    assert!(state.buy_galaxy());
    assert_u32_eq(state.galaxies, 1, "after first galaxy");

    // Now requirement is 140
    assert_u64_eq(
        state.galaxy_requirement(),
        140,
        "requirement for second galaxy",
    );

    // Can't buy with less than 140
    state.dim_boosts = 4;
    state.dimensions[7].amount = Decimal::from_float(139.0);
    assert!(!state.can_buy_galaxy());

    // Can buy with 140
    state.dimensions[7].amount = Decimal::from_float(140.0);
    assert!(state.can_buy_galaxy());
}
