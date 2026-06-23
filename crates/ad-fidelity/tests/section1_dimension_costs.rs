//! Fidelity tests for dimension cost model (Section 1).
//!
//! Verifies that dimension costs follow the per-10-purchase
//! formula: cost = baseCost[tier] × costMult[tier]^floor(bought/10)

use ad_core::{Decimal, GameState};
use ad_fidelity::tolerance::{assert_approx_eq, EPSILON_EXACT};

/// Test basic cost calculation for each tier at various bought
/// counts.
#[test]
fn cost_per_10_purchases_ad1() {
    let mut state = GameState::new();

    // AD1: base=10, mult=1e3
    // bought=0: cost=10
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(10.0),
        EPSILON_EXACT,
        "AD1 cost at bought=0",
    );

    // Simulate buying 1 — cost should still be 10
    state.dimensions[0].bought = 1;
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(10.0),
        EPSILON_EXACT,
        "AD1 cost at bought=1",
    );

    // bought=9: still same group
    state.dimensions[0].bought = 9;
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(10.0),
        EPSILON_EXACT,
        "AD1 cost at bought=9",
    );

    // bought=10: next group, cost = 10 × 1e3 = 10,000
    state.dimensions[0].bought = 10;
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(10_000.0),
        EPSILON_EXACT,
        "AD1 cost at bought=10",
    );

    // bought=19: still in group 1
    state.dimensions[0].bought = 19;
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(10_000.0),
        EPSILON_EXACT,
        "AD1 cost at bought=19",
    );

    // bought=20: group 2, cost = 10 × 1e3^2 = 10,000,000
    state.dimensions[0].bought = 20;
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e7),
        EPSILON_EXACT,
        "AD1 cost at bought=20",
    );

    // bought=30: group 3, cost = 10 × 1e3^3 = 1e10
    state.dimensions[0].bought = 30;
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e10),
        EPSILON_EXACT,
        "AD1 cost at bought=30",
    );
}

#[test]
fn cost_per_10_purchases_ad2() {
    let mut state = GameState::new();

    // AD2: base=100, mult=1e4
    state.dimensions[1].bought = 0;
    let cost = state.dimension_cost(1);
    assert_approx_eq(
        cost,
        Decimal::from_float(100.0),
        EPSILON_EXACT,
        "AD2 cost at bought=0",
    );

    state.dimensions[1].bought = 9;
    let cost = state.dimension_cost(1);
    assert_approx_eq(
        cost,
        Decimal::from_float(100.0),
        EPSILON_EXACT,
        "AD2 cost at bought=9",
    );

    // bought=10: cost = 100 × 1e4 = 1,000,000
    state.dimensions[1].bought = 10;
    let cost = state.dimension_cost(1);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e6),
        EPSILON_EXACT,
        "AD2 cost at bought=10",
    );

    // bought=20: cost = 100 × 1e4^2 = 1e10
    state.dimensions[1].bought = 20;
    let cost = state.dimension_cost(1);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e10),
        EPSILON_EXACT,
        "AD2 cost at bought=20",
    );
}

#[test]
fn cost_per_10_purchases_ad8() {
    let mut state = GameState::new();

    // AD8: base=1e24, mult=1e15
    state.dimensions[7].bought = 0;
    let cost = state.dimension_cost(7);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e24),
        EPSILON_EXACT,
        "AD8 cost at bought=0",
    );

    // bought=10: cost = 1e24 × 1e15 = 1e39
    state.dimensions[7].bought = 10;
    let cost = state.dimension_cost(7);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e39),
        EPSILON_EXACT,
        "AD8 cost at bought=10",
    );

    // bought=80: cost = 1e24 × (1e15)^8 = 1e144
    state.dimensions[7].bought = 80;
    let cost = state.dimension_cost(7);
    assert_approx_eq(
        cost,
        Decimal::from_float(1e144),
        EPSILON_EXACT,
        "AD8 cost at bought=80",
    );
}

/// Verify that purchasing the first 10 of a tier all costs the
/// same, and the 11th costs more.
#[test]
fn cost_unchanged_within_group() {
    let mut state = GameState::new();
    state.antimatter = Decimal::from_float(1e30);

    for i in 0..10 {
        let cost = state.dimension_cost(0);
        assert_approx_eq(
            cost,
            Decimal::from_float(10.0),
            EPSILON_EXACT,
            &format!("AD1 cost before purchase {}", i + 1),
        );
        assert!(state.buy_dimension(0));
    }

    // After 10 purchases, cost should now be 10,000
    let cost = state.dimension_cost(0);
    assert_approx_eq(
        cost,
        Decimal::from_float(10_000.0),
        EPSILON_EXACT,
        "AD1 cost after 10 purchases",
    );
    assert_eq!(state.dimensions[0].bought, 10);
}

/// Tickspeed cost: 1000 × 10^bought (scales per purchase, not per
/// 10).
#[test]
fn tickspeed_cost() {
    let mut state = GameState::new();

    // bought=0: cost=1000
    assert_approx_eq(
        state.tickspeed.cost,
        Decimal::from_float(1000.0),
        EPSILON_EXACT,
        "tickspeed cost at bought=0",
    );

    // Buy and check progression
    state.antimatter = Decimal::from_float(1e60);

    state.buy_tickspeed();
    assert_approx_eq(
        state.tickspeed.cost,
        Decimal::from_float(10_000.0),
        EPSILON_EXACT,
        "tickspeed cost at bought=1",
    );

    state.buy_tickspeed();
    assert_approx_eq(
        state.tickspeed.cost,
        Decimal::from_float(100_000.0),
        EPSILON_EXACT,
        "tickspeed cost at bought=2",
    );

    // Jump ahead: bought=10, cost should be 1000 × 10^10 = 1e13
    for _ in 2..10 {
        state.buy_tickspeed();
    }
    assert_approx_eq(
        state.tickspeed.cost,
        Decimal::from_float(1e13),
        EPSILON_EXACT,
        "tickspeed cost at bought=10",
    );
}
