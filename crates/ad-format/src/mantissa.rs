//! Mantissa formatting helpers (port of the mantissa functions in the notations
//! library's `utils.ts`).

use break_infinity::Decimal;

/// Port of `formatMantissaBaseTen`: `n.toFixed(precision)`.
///
/// The JS clamps with `Math.max(0, precision)` to absorb its `-1` "unspecified"
/// sentinel; our `precision` is `u32`, so non-negativity is guaranteed by the type
/// and no clamp is needed.
///
// TODO(fidelity): JS `toFixed` rounds half-away-from-zero; Rust's `{:.*}` formatter
// rounds half-to-even. Reconcile when the `ad-fidelity` `format()` harness lands.
pub(crate) fn format_mantissa_base_ten(n: f64, precision: u32) -> String {
    format!("{:.*}", precision as usize, n)
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
/// The closures are value-formatters. The JS threads `precision` /
/// `precisionExponent` through here too, but for us those are always
/// `opts.places` / `opts.places_exponent`, so the call sites capture them instead
/// (which also keeps this engine independent of `FormatOptions`).
///
/// - `mantissa_fmt(mantissa)` — renders the mantissa.
/// - `exponent_fmt(exponent)` — renders the exponent.
pub(crate) fn format_mantissa_with_exponent<FM, FE>(
    n: &Decimal,
    spec: &MantissaSpec,
    mantissa_fmt: FM,
    exponent_fmt: FE,
) -> String
where
    FM: Fn(f64) -> String,
    FE: Fn(f64) -> String,
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

    let mut m = mantissa_fmt(mantissa);
    // Mantissa rounded up to the base (e.g. "10.00") — roll into the exponent.
    if m == mantissa_fmt(real_base) {
        m = mantissa_fmt(1.0);
        exponent += steps_f;
    }

    // High enough that the exponent absorbed the whole value.
    if exponent == 0.0 {
        return m;
    }

    let e = exponent_fmt(exponent);
    format!("{m}{}{e}", spec.separator)
}
