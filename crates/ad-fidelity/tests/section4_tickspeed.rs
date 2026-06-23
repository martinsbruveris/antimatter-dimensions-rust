//! Fidelity tests for tickspeed and galaxy effects (Section 4).
//!
//! Tickspeed formula:
//!   Pre-3 galaxies: base_mult[g] - g * 0.02
//!   3+ galaxies:    0.8 * 0.965^(g - 4)
//!
//! Tickspeed value:
//!   current = 1000 * multiplier^bought
//!   perSecond = 1000 / current

use ad_core::GameState;
use ad_fidelity::tolerance::{assert_approx_eq_f64, EPSILON_EXACT};

/// Verify tickspeed purchase multiplier at various galaxy counts.
#[test]
fn tickspeed_multiplier_0_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 0;

    // 1/1.1245 ≈ 0.88936
    let mult = state.tickspeed_purchase_multiplier();
    let expected = 1.0 / 1.1245;
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 0 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_1_galaxy() {
    let mut state = GameState::new();
    state.galaxies = 1;

    // base = 1/1.11888888, reduction = 1 * 0.02
    let expected = 1.0 / 1.11888888 - 1.0 * 0.02;
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(mult, expected, EPSILON_EXACT, "tickspeed mult at 1 galaxy");
}

#[test]
fn tickspeed_multiplier_2_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 2;

    // base = 1/1.11267177, reduction = 2 * 0.02
    let expected = 1.0 / 1.11267177 - 2.0 * 0.02;
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 2 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_3_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 3;

    // Exponential formula: 0.8 * 0.965^(3-4) = 0.8 * 0.965^-1
    let expected = 0.8 * 0.965_f64.powf(-1.0);
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 3 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_4_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 4;

    // 0.8 * 0.965^0 = 0.8
    let expected = 0.8;
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 4 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_5_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 5;

    // 0.8 * 0.965^1 = 0.772
    let expected = 0.8 * 0.965;
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 5 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_10_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 10;

    // 0.8 * 0.965^6
    let expected = 0.8 * 0.965_f64.powf(6.0);
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 10 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_20_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 20;

    // 0.8 * 0.965^16
    let expected = 0.8 * 0.965_f64.powf(16.0);
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 20 galaxies",
    );
}

#[test]
fn tickspeed_multiplier_50_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 50;

    // 0.8 * 0.965^46
    let expected = 0.8 * 0.965_f64.powf(46.0);
    let mult = state.tickspeed_purchase_multiplier();
    assert_approx_eq_f64(
        mult,
        expected,
        EPSILON_EXACT,
        "tickspeed mult at 50 galaxies",
    );
}

/// Verify tickspeed_ms and ticks_per_second.
#[test]
fn tickspeed_value_no_upgrades() {
    let state = GameState::new();
    // 0 galaxies, 0 bought: current = 1000 * mult^0 = 1000
    let current = state.current_tickspeed_ms();
    assert_approx_eq_f64(
        current,
        1000.0,
        EPSILON_EXACT,
        "tickspeed_ms with no upgrades",
    );
}

#[test]
fn tickspeed_value_with_purchases() {
    let mut state = GameState::new();
    state.galaxies = 0;
    state.tickspeed.bought = 10;

    let mult = state.tickspeed_purchase_multiplier();
    let expected_ms = 1000.0 * mult.powi(10);
    let current = state.current_tickspeed_ms();
    assert_approx_eq_f64(
        current,
        expected_ms,
        EPSILON_EXACT,
        "tickspeed_ms with 10 purchases, 0 galaxies",
    );
}

#[test]
fn tickspeed_value_with_galaxies() {
    let mut state = GameState::new();
    state.galaxies = 5;
    state.tickspeed.bought = 10;

    let mult = state.tickspeed_purchase_multiplier();
    let expected_ms = 1000.0 * mult.powi(10);
    let current = state.current_tickspeed_ms();
    assert_approx_eq_f64(
        current,
        expected_ms,
        EPSILON_EXACT,
        "tickspeed_ms with 10 purchases, 5 galaxies",
    );

    // perSecond = 1000 / current
    let effect = state.tickspeed_effect();
    let expected_effect = 1000.0 / expected_ms;
    assert_approx_eq_f64(
        effect.to_f64(),
        expected_effect,
        1e-9,
        "tickspeed effect with 10 purchases, 5 galaxies",
    );
}

/// Verify tickspeed effect integrates correctly into production.
#[test]
fn tickspeed_in_production() {
    let mut state = GameState::new();
    state.galaxies = 0;
    state.tickspeed.bought = 5;
    state.dimensions[0].amount = Decimal::from_float(10.0);
    state.dimensions[0].bought = 10; // buy10 = 2

    use ad_core::Decimal;

    // multiplier = buy10(2) × dimboost(2^max(0,0-0)=1) = 2
    // tickspeed_mult = 1/1.1245
    // ticks_per_second = 1000 / (1000 × mult^5) = 1/mult^5
    let ts_mult = state.tickspeed_purchase_multiplier();
    let ticks_per_second = 1.0 / ts_mult.powi(5);
    // production = 10 × 2 × ticks_per_second
    let expected = 10.0 * 2.0 * ticks_per_second;

    let production = state.dimension_production_per_second(0);
    assert_approx_eq_f64(
        production.to_f64(),
        expected,
        1e-9,
        "AD1 production with tickspeed",
    );
}
