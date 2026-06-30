//! Fidelity tests for sacrifice (Section 6).
//!
//! Pre-infinity totalBoost formula (stateless from sacrificed):
//!   if sacrificed == 0: totalBoost = 1
//!   prePowerBoost = max(1, log10(sacrificed) / 10)
//!   totalBoost = prePowerBoost^2
//!
//! nextBoost formula (individual gain ratio):
//!   sacrificed_clamped = max(sacrificed, 1)
//!   prePowerMult = max(1, (log10(AD1) / 10) /
//!                       max(log10(sacrificed_clamped) / 10, 1))
//!   nextBoost = prePowerMult^2

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{assert_approx_eq, EPSILON_EXACT};

/// Verify the two distinct sacrifice gates: *visibility*
/// (`Sacrifice.isVisible = Achievement(18).isUnlocked`, set by buying an 8th
/// Antimatter Dimension and persistent forever) and *enable*
/// (`Sacrifice.canSacrifice`, requiring `DimBoost.purchasedBoosts > 4`).
#[test]
fn sacrifice_unlock() {
    let mut state = GameState::new();

    // Fresh game: neither visible nor performable.
    assert!(!state.can_sacrifice());
    assert!(!state.sacrifice_unlocked());

    // Boosts alone do not make the button visible — visibility is achievement 18.
    state.dim_boosts = 5;
    assert!(!state.sacrifice_unlocked());

    // Buying an 8th dimension unlocks achievement 18 → the button is visible and
    // stays so (achievements never re-lock).
    state.dimensions[6].amount = Decimal::ONE; // own a 7th so the 8th is buyable
    state.antimatter = Decimal::new(1.0, 70);
    assert!(state.buy_dimension(7));
    assert!(state.sacrifice_unlocked());

    // Visible but still not performable until AD8 > 0 with a meaningful AD1.
    state.dimensions[0].amount = Decimal::from_float(1e20);
    assert!(state.can_sacrifice());
}

/// Verify sacrifice requires AD8 amount > 0.
#[test]
fn sacrifice_requires_ad8() {
    let mut state = GameState::new();
    state.dim_boosts = 5;
    state.dimensions[0].amount = Decimal::from_float(1e20);
    // AD8 = 0
    assert!(!state.can_sacrifice());
}

/// Verify next_sacrifice_boost formula with various states.
#[test]
fn sacrifice_next_boost_formula() {
    let mut state = GameState::new();
    state.dim_boosts = 5;
    state.dimensions[7].amount = Decimal::from_float(1.0);

    // Case: AD1=1e100, total_sacrificed=1e50
    // prePowerMult = (100/10) / max(50/10, 1) = 10/5 = 2
    // nextBoost = 2^2 = 4
    state.dimensions[0].amount = Decimal::from_float(1e100);
    state.sacrificed = Decimal::from_float(1e50);

    let actual = state.next_sacrifice_boost();
    assert_approx_eq(
        actual,
        Decimal::from_float(4.0),
        EPSILON_EXACT,
        "next_sacrifice_boost: AD1=1e100, sacrificed=1e50",
    );

    // Case: AD1=1e200, total_sacrificed=1e100
    // prePowerMult = (200/10) / (100/10) = 20/10 = 2
    // nextBoost = 4
    state.dimensions[0].amount = Decimal::from_float(1e200);
    state.sacrificed = Decimal::from_float(1e100);

    let actual = state.next_sacrifice_boost();
    assert_approx_eq(
        actual,
        Decimal::from_float(4.0),
        EPSILON_EXACT,
        "next_sacrifice_boost: AD1=1e200, sacrificed=1e100",
    );

    // Case: AD1=1e50, total_sacrificed=1e100
    // prePowerMult = (50/10) / (100/10) = 5/10 = 0.5 → clamped
    //   to 1 nextBoost = 1
    state.dimensions[0].amount = Decimal::from_float(1e50);
    state.sacrificed = Decimal::from_float(1e100);

    let actual = state.next_sacrifice_boost();
    assert_approx_eq(
        actual,
        Decimal::from_float(1.0),
        EPSILON_EXACT,
        "next_sacrifice_boost: AD1=1e50, sacrificed=1e100 (clamped)",
    );
}

/// Verify stateless totalBoost formula.
#[test]
fn sacrifice_total_boost_stateless() {
    let mut state = GameState::new();
    state.dim_boosts = 5;
    state.dimensions[7].amount = Decimal::from_float(1.0);

    // No sacrifices yet: totalBoost = 1
    assert_eq!(state.sacrifice_multiplier(), Decimal::ONE);

    // sacrificed = 1e100: prePowerBoost = max(1, 100/10) = 10
    // totalBoost = 10^2 = 100
    state.sacrificed = Decimal::from_float(1e100);
    assert_approx_eq(
        state.sacrifice_multiplier(),
        Decimal::from_float(100.0),
        EPSILON_EXACT,
        "totalBoost with sacrificed=1e100",
    );

    // sacrificed = 1e200: prePowerBoost = 200/10 = 20
    // totalBoost = 20^2 = 400
    state.sacrificed = Decimal::from_float(1e200);
    assert_approx_eq(
        state.sacrifice_multiplier(),
        Decimal::from_float(400.0),
        EPSILON_EXACT,
        "totalBoost with sacrificed=1e200",
    );
}

/// Verify first sacrifice (no prior sacrifices).
#[test]
fn sacrifice_first_time() {
    let mut state = GameState::new();
    state.dim_boosts = 5;
    state.dimensions[0].amount = Decimal::from_float(1e20);
    state.dimensions[0].bought = 50;
    state.dimensions[7].amount = Decimal::from_float(1.0);

    // First sacrifice: total_sacrificed=0
    // After: sacrificed=1e20
    // totalBoost = max(1, log10(1e20)/10)^2 = (20/10)^2 = 4
    assert!(state.sacrifice());

    assert_approx_eq(
        state.sacrifice_multiplier(),
        Decimal::from_float(4.0),
        EPSILON_EXACT,
        "sacrifice_multiplier after first sacrifice (AD1=1e20)",
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

/// Verify cumulative sacrifice multiplier is stateless.
/// After two sacrifices, totalBoost depends only on total
/// sacrificed, not the order or individual boosts.
#[test]
fn sacrifice_cumulative_boost() {
    let mut state = GameState::new();
    state.dim_boosts = 5;
    state.dimensions[7].amount = Decimal::from_float(1.0);

    // First sacrifice: AD1=1e100
    // After: sacrificed=1e100
    // totalBoost = max(1, 100/10)^2 = 100
    state.dimensions[0].amount = Decimal::from_float(1e100);
    assert!(state.sacrifice());

    assert_approx_eq(
        state.sacrifice_multiplier(),
        Decimal::from_float(100.0),
        EPSILON_EXACT,
        "sacrifice_multiplier after first sacrifice (AD1=1e100)",
    );

    // Second sacrifice: AD1=1e200
    // After: sacrificed ≈ 1e200 (1e100 + 1e200 ≈ 1e200)
    // totalBoost = max(1, 200/10)^2 = 400
    state.dimensions[0].amount = Decimal::from_float(1e200);
    state.dimensions[7].amount = Decimal::from_float(1.0);
    assert!(state.sacrifice());

    assert_approx_eq(
        state.sacrifice_multiplier(),
        Decimal::from_float(400.0),
        EPSILON_EXACT,
        "sacrifice_multiplier after two sacrifices",
    );
}
