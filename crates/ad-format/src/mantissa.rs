//! Mantissa formatting helpers (port of the mantissa functions in the notations
//! library's `utils.ts`).

use break_infinity::Decimal;

use crate::exponent::is_exponent_fully_shown;
use crate::options::ExponentCommas;

/// Port of `formatMantissaBaseTen`: `n.toFixed(max(0, precision))`.
///
/// `precision` is clamped to 0 because the JS uses `-1` as a sentinel and guards
/// with `Math.max(0, precision)`.
///
// TODO(fidelity): JS `toFixed` rounds half-away-from-zero; Rust's `{:.*}` formatter
// rounds half-to-even. Reconcile when the `ad-fidelity` `format()` harness lands.
pub(crate) fn format_mantissa_base_ten(n: f64, precision: i32) -> String {
    format!("{:.*}", precision.max(0) as usize, n)
}

/// `base ** exponent` as a `Decimal`. Specialised to `pow10` for base 10 (the only
/// base where exactness matters for the common notations); falls back to
/// `Decimal::pow` otherwise.
fn decimal_pow(base: f64, exponent: f64) -> Decimal {
    if base == 10.0 {
        Decimal::pow10(exponent)
    } else {
        Decimal::from_float(base).pow(&Decimal::from_float(exponent))
    }
}

/// The notation-static shape of a mantissa/exponent split, grouped so callers
/// read as configuration rather than a wall of positional flags. Mirrors the
/// trailing arguments of JS `formatMantissaWithExponent`.
pub(crate) struct MantissaSpec<'a> {
    /// Mantissa/exponent base (10 for Scientific/Engineering, 1000 for Standard).
    pub base: f64,
    /// Exponent granularity: the exponent is forced to a multiple of `steps` and
    /// the mantissa lands in `[1, base**steps)` (1 for Scientific, 3 for
    /// Engineering).
    pub steps: i32,
    /// Joins mantissa and exponent (`"e"`, or `" "` for Standard).
    pub separator: &'a str,
    /// Clamp the exponent to be non-negative (Standard never shows `K^-1`).
    pub force_positive_exponent: bool,
    /// Port of JS `useLogIfExponentIsFormatted`: when the exponent is itself
    /// rendered in notation (not plain/comma), drop the mantissa entirely so the
    /// output is just the formatted exponent. `false` for all four M1 notations.
    pub use_log_if_exponent_is_formatted: bool,
}

/// Port of `formatMantissaWithExponent` — the shared mantissa/exponent engine
/// behind Scientific, Engineering, Standard, and the custom (Letters/Emoji)
/// notations.
///
/// Splits `n` per `spec` into a mantissa in `[1, base**steps)` and an exponent
/// that is a multiple of `steps`, formats each with the supplied closures, and
/// joins them with `spec.separator`. Handles mantissa roll-over (`9.999e3 -> 1e4`)
/// and the `exponent == 0` short-circuit exactly as the JS does.
///
/// - `mantissa_fmt(mantissa, precision)` — renders the mantissa.
/// - `exponent_fmt(exponent, precision_exponent)` — renders the exponent.
pub(crate) fn format_mantissa_with_exponent<FM, FE>(
    n: &Decimal,
    precision: i32,
    precision_exponent: i32,
    spec: &MantissaSpec,
    mantissa_fmt: FM,
    exponent_fmt: FE,
    exponent_commas: &ExponentCommas,
) -> String
where
    FM: Fn(f64, i32) -> String,
    FE: Fn(f64, i32) -> String,
{
    let steps_f = spec.steps as f64;
    let real_base = spec.base.powi(spec.steps);

    let mut exponent = n.log(real_base).floor() * steps_f;
    if spec.force_positive_exponent {
        exponent = exponent.max(0.0);
    }
    let mut mantissa = (*n / decimal_pow(spec.base, exponent)).to_f64();

    // Rare precision correction (e.g. 0.8e1e15 whose log rounds the wrong way).
    if !(1.0..real_base).contains(&mantissa) {
        let adjust = (mantissa.ln() / real_base.ln()).floor();
        mantissa /= real_base.powf(adjust);
        exponent += steps_f * adjust;
    }

    let mut m = mantissa_fmt(mantissa, precision);
    // Mantissa rounded up to the base (e.g. "10.00") — roll into the exponent.
    if m == mantissa_fmt(real_base, precision) {
        m = mantissa_fmt(1.0, precision);
        exponent += steps_f;
    }

    // High enough that the exponent absorbed the whole value.
    if exponent == 0.0 {
        return m;
    }

    let e = exponent_fmt(exponent, precision_exponent);
    // When the exponent is itself in notation, some notations show only the
    // exponent (e.g. Logarithm). None of the M1 four opt in, so `m` is kept.
    if spec.use_log_if_exponent_is_formatted
        && !is_exponent_fully_shown(exponent, exponent_commas)
    {
        m = String::new();
    }
    format!("{m}{}{e}", spec.separator)
}
