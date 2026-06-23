//! Fidelity tests for sacrifice (Section 6).
//!
//! Pre-infinity totalBoost formula:
//!   prePowerBoost = max(1, log10(total_sacrificed) / 10)
//!   totalBoost = prePowerBoost^2
//!
//! nextBoost formula:
//!   prePowerMult = max(1, (log10(AD1) / 10) /
//!                       max(log10(total_sacrificed) / 10, 1))
//!   nextBoost = prePowerMult^2

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{assert_approx_eq, EPSILON_EXACT};

/// Verify sacrifice unlock condition (requires dim_boosts >= 5
/// in JS, but our impl uses sacrifice_unlocked flag set at
/// boost 1+).
#[test]
fn sacrifice_unlock() {
    let mut state = GameState::new();

    // Fresh game: sacrifice not unlocked
    assert!(!state.can_sacrifice());

    // After a dim boost, sacrifice becomes available
    state.sacrifice_unlocked = true;
    state.dimensions[0].amount = Decimal::from_float(100.0);
    assert!(state.can_sacrifice());
}

/// Verify sacrifice cannot proceed without AD1 amount.
#[test]
fn sacrifice_requires_ad1() {
    let mut state = GameState::new();
    state.sacrifice_unlocked = true;
    state.dimensions[0].amount = Decimal::ZERO;

    assert!(!state.can_sacrifice());
}

/// Verify next_sacrifice_boost formula with various states.
#[test]
fn sacrifice_next_boost_formula() {
    let mut state = GameState::new();
    state.sacrifice_unlocked = true;

    // Case: AD1=1e100, total_sacrificed=1e50
    // prePowerMult = (100/10) / max(50/10, 1) = 10/5 = 2
    // nextBoost = 2^2 = 4
    state.dimensions[0].amount = Decimal::from_float(1e100);
    state.sacrificed = Decimal::from_float(1e50);

    let expected_total = state.sacrifice_boost * Decimal::from_float(4.0);
    let actual = state.sacrifice_multiplier_if_sacrificed();
    assert_approx_eq(
        actual,
        expected_total,
        EPSILON_EXACT,
        "sacrifice boost: AD1=1e100, sacrificed=1e50",
    );

    // Case: AD1=1e200, total_sacrificed=1e100
    // prePowerMult = (200/10) / (100/10) = 20/10 = 2
    // nextBoost = 4
    state.dimensions[0].amount = Decimal::from_float(1e200);
    state.sacrificed = Decimal::from_float(1e100);
    state.sacrifice_boost = Decimal::ONE;

    let actual = state.sacrifice_multiplier_if_sacrificed();
    assert_approx_eq(
        actual,
        Decimal::from_float(4.0),
        EPSILON_EXACT,
        "sacrifice boost: AD1=1e200, sacrificed=1e100",
    );

    // Case: AD1=1e50, total_sacrificed=1e100
    // prePowerMult = (50/10) / (100/10) = 5/10 = 0.5 → clamped
    //   to 1 nextBoost = 1 (not worth sacrificing)
    state.dimensions[0].amount = Decimal::from_float(1e50);
    state.sacrificed = Decimal::from_float(1e100);
    state.sacrifice_boost = Decimal::ONE;

    let actual = state.sacrifice_multiplier_if_sacrificed();
    assert_approx_eq(
        actual,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "sacrifice boost: AD1=1e50, sacrificed=1e100 (clamped)",
    );
}

/// Verify first sacrifice (no prior sacrifices).
#[test]
fn sacrifice_first_time() {
    let mut state = GameState::new();
    state.sacrifice_unlocked = true;
    state.dim_boosts = 5;
    state.dimensions[0].amount = Decimal::from_float(1e20);
    state.dimensions[0].bought = 50;

    // First sacrifice: total_sacrificed=0
    // With sacrificed=0, log10(0) is undefined.
    // The JS code: max(log10(sacrificed)/10, 1) — if sacrificed=0,
    // log10(0)=-inf, so max(-inf/10, 1) = 1
    // prePowerMult = (20/10) / 1 = 2
    // nextBoost = 2^2 = 4
    assert!(state.sacrifice());

    assert_approx_eq(
        state.sacrifice_boost,
        Decimal::from_float(4.0),
        EPSILON_EXACT,
        "sacrifice_boost after first sacrifice (AD1=1e20)",
    );

    // total_sacrificed should now be 1e20
    assert_approx_eq(
        state.sacrificed,
        Decimal::from_float(1e20),
        EPSILON_EXACT,
        "total_sacrificed after first sacrifice",
    );

    // AD1 amount reset to 0
    assert_eq!(state.dimensions[0].amount, Decimal::ZERO);
    // AD1 bought preserved
    assert_eq!(state.dimensions[0].bought, 50);
}

/// Verify sacrifice resets dims 1-7 but preserves dim 8.
#[test]
fn sacrifice_reset_behavior() {
    let mut state = GameState::new();
    state.sacrifice_unlocked = true;
    state.dim_boosts = 5;

    // Set up various dimensions
    state.dimensions[0].amount = Decimal::from_float(1e100);
    state.dimensions[0].bought = 100;
    state.dimensions[1].amount = Decimal::from_float(1e80);
    state.dimensions[1].bought = 80;
    state.dimensions[6].amount = Decimal::from_float(1e20);
    state.dimensions[6].bought = 20;
    state.dimensions[7].amount = Decimal::from_float(100.0);
    state.dimensions[7].bought = 80;

    assert!(state.sacrifice());

    // Dims 1-7 (indices 0-6): amounts reset
    assert_eq!(state.dimensions[0].amount, Decimal::ZERO);
    assert_eq!(state.dimensions[1].amount, Decimal::ZERO);
    assert_eq!(state.dimensions[6].amount, Decimal::ZERO);

    // Dim 8 (index 7): preserved!
    assert_approx_eq(
        state.dimensions[7].amount,
        Decimal::from_float(100.0),
        EPSILON_EXACT,
        "AD8 amount preserved after sacrifice",
    );

    // All bought counts preserved
    assert_eq!(state.dimensions[0].bought, 100);
    assert_eq!(state.dimensions[7].bought, 80);
}

/// Verify cumulative sacrifice multiplier (running product).
#[test]
fn sacrifice_cumulative_boost() {
    let mut state = GameState::new();
    state.sacrifice_unlocked = true;
    state.dim_boosts = 5;

    // First sacrifice: AD1=1e100
    // sacrificed=0, prePowerMult=(100/10)/1=10, boost=100
    state.dimensions[0].amount = Decimal::from_float(1e100);
    assert!(state.sacrifice());

    assert_approx_eq(
        state.sacrifice_boost,
        Decimal::from_float(100.0),
        EPSILON_EXACT,
        "sacrifice_boost after first sacrifice (AD1=1e100)",
    );

    // Second sacrifice: AD1=1e200
    // sacrificed=1e100, log(sacrificed)/10=10
    // prePowerMult = (200/10) / 10 = 2, boost = 4
    // cumulative = 100 × 4 = 400
    state.dimensions[0].amount = Decimal::from_float(1e200);
    assert!(state.sacrifice());

    assert_approx_eq(
        state.sacrifice_boost,
        Decimal::from_float(400.0),
        EPSILON_EXACT,
        "cumulative sacrifice_boost after two sacrifices",
    );
}
