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

/// Port of `formatMantissaWithExponent` — the shared mantissa/exponent engine
/// behind Scientific, Engineering, Standard, and the custom (Letters/Emoji)
/// notations.
///
/// Splits `n` into a mantissa in `[1, base**steps)` and an exponent that is a
/// multiple of `steps`, formats each with the supplied closures, and joins them
/// with `separator`. Handles mantissa roll-over (`9.999e3 -> 1e4`) and the
/// `exponent == 0` short-circuit exactly as the JS does.
///
/// - `mantissa_fmt(mantissa, precision)` — renders the mantissa.
/// - `exponent_fmt(exponent, precision_exponent)` — renders the exponent.
/// - `mantissa_fmt_if_exp_formatted` — alternate mantissa rendering used when the
///   exponent itself had to be formatted in notation (e.g. drop the decimals).
///   `None` matches the JS `undefined`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn format_mantissa_with_exponent<FM, FE, FI>(
    n: &Decimal,
    precision: i32,
    precision_exponent: i32,
    base: f64,
    steps: i32,
    mantissa_fmt: FM,
    exponent_fmt: FE,
    mantissa_fmt_if_exp_formatted: Option<FI>,
    separator: &str,
    force_positive_exponent: bool,
    exponent_commas: &ExponentCommas,
) -> String
where
    FM: Fn(f64, i32) -> String,
    FE: Fn(f64, i32) -> String,
    FI: Fn(f64, i32) -> String,
{
    let steps_f = steps as f64;
    let real_base = base.powi(steps);

    let mut exponent = n.log(real_base).floor() * steps_f;
    if force_positive_exponent {
        exponent = exponent.max(0.0);
    }
    let mut mantissa = (*n / decimal_pow(base, exponent)).to_f64();

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
    if let Some(fmt_if) = mantissa_fmt_if_exp_formatted {
        if !is_exponent_fully_shown(exponent, exponent_commas) {
            m = fmt_if(mantissa, precision);
        }
    }
    format!("{m}{separator}{e}")
}
