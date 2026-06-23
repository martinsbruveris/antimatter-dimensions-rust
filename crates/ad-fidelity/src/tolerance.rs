//! Tolerance utilities for comparing Decimal values between
//! Rust and JS implementations.
//!
//! Uses log-space relative tolerance:
//!   |log10(rust) - log10(js)| < epsilon
//!
//! For values near zero or with mixed signs, falls back to
//! absolute comparison.

use break_infinity::Decimal;

/// Default epsilon for exact formula matches (single computation).
pub const EPSILON_EXACT: f64 = 1e-10;

/// Epsilon for multi-step simulations (accumulated error).
pub const EPSILON_SIMULATION: f64 = 1e-6;

/// Compare two Decimal values using log-space relative tolerance.
///
/// Returns true if the values are within `epsilon` of each other
/// in log10 space. For values near zero, uses absolute comparison.
pub fn approx_eq_log(actual: Decimal, expected: Decimal, epsilon: f64) -> bool {
    // Both zero
    if actual == Decimal::ZERO && expected == Decimal::ZERO {
        return true;
    }

    // If either is zero but not both, check if the other is tiny
    if actual == Decimal::ZERO || expected == Decimal::ZERO {
        let nonzero = if actual == Decimal::ZERO {
            expected
        } else {
            actual
        };
        // If the nonzero value is very small, consider them equal
        return nonzero.abs().log10() < -10.0;
    }

    // Both positive or both negative — compare in log space
    let log_actual = actual.abs().log10();
    let log_expected = expected.abs().log10();

    (log_actual - log_expected).abs() < epsilon
}

/// Compare two f64 values using relative tolerance.
pub fn approx_eq_f64(actual: f64, expected: f64, epsilon: f64) -> bool {
    if actual == expected {
        return true;
    }
    if expected == 0.0 {
        return actual.abs() < epsilon;
    }
    ((actual - expected) / expected).abs() < epsilon
}

/// Assert that two Decimal values are approximately equal in
/// log-space, with a descriptive message on failure.
#[track_caller]
pub fn assert_approx_eq(
    actual: Decimal,
    expected: Decimal,
    epsilon: f64,
    context: &str,
) {
    if !approx_eq_log(actual, expected, epsilon) {
        panic!(
            "Fidelity mismatch: {context}\n\
             actual:   {actual}\n\
             expected: {expected}\n\
             log10(actual):   {}\n\
             log10(expected): {}\n\
             epsilon: {epsilon}",
            if actual > Decimal::ZERO {
                actual.log10()
            } else {
                f64::NEG_INFINITY
            },
            if expected > Decimal::ZERO {
                expected.log10()
            } else {
                f64::NEG_INFINITY
            },
        );
    }
}

/// Assert that two f64 values are approximately equal with a
/// descriptive message on failure.
#[track_caller]
pub fn assert_approx_eq_f64(actual: f64, expected: f64, epsilon: f64, context: &str) {
    if !approx_eq_f64(actual, expected, epsilon) {
        panic!(
            "Fidelity mismatch: {context}\n\
             actual:   {actual}\n\
             expected: {expected}\n\
             relative error: {}\n\
             epsilon: {epsilon}",
            if expected != 0.0 {
                ((actual - expected) / expected).abs()
            } else {
                actual.abs()
            },
        );
    }
}

/// Assert that two Decimal values are exactly equal.
#[track_caller]
pub fn assert_decimal_eq(actual: Decimal, expected: Decimal, context: &str) {
    if actual != expected {
        panic!(
            "Fidelity mismatch (exact): {context}\n\
             actual:   {actual}\n\
             expected: {expected}",
        );
    }
}

/// Assert that a u64 matches an expected value.
#[track_caller]
pub fn assert_u64_eq(actual: u64, expected: u64, context: &str) {
    if actual != expected {
        panic!(
            "Fidelity mismatch: {context}\n\
             actual:   {actual}\n\
             expected: {expected}",
        );
    }
}

/// Assert that a u32 matches an expected value.
#[track_caller]
pub fn assert_u32_eq(actual: u32, expected: u32, context: &str) {
    if actual != expected {
        panic!(
            "Fidelity mismatch: {context}\n\
             actual:   {actual}\n\
             expected: {expected}",
        );
    }
}
