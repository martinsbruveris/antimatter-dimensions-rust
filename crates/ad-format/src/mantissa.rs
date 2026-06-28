//! Mantissa formatting helpers.

use break_infinity::Decimal;

/// Format `n` to a fixed `precision` number of decimal places.
///
// Accepted divergence: the original game rounds half-away-from-zero; Rust's `{:.*}`
// formatter rounds half-to-even. This only differs for mantissas that are an exact
// binary half at the rounded digit (e.g. 2.5, 1.25) — a presentation-layer corner
// that is essentially never hit in-game, so we keep the native Rust rounding. The
// behaviour is pinned in `tests/rounding_divergence.rs`.
pub(crate) fn format_mantissa(n: f64, precision: u32) -> String {
    format!("{:.*}", precision as usize, n)
}

/// The notation-static shape of a mantissa/exponent split, grouped so callers
/// read as configuration rather than a wall of positional flags.
///
/// The mantissa base is always 10, so the displayed exponent is a true power of
/// ten. Standard (which counts thousands, not powers of ten) divides it back down
/// by 3 in its own abbreviation step.
pub(crate) struct MantissaSpec<'a> {
    /// Exponent granularity, in decades: the exponent is forced to a multiple of
    /// `steps` and the mantissa lands in `[1, 10**steps)` (1 for Scientific, 3 for
    /// Engineering and Standard).
    pub steps: i32,
    /// Joins mantissa and exponent (`"e"`, or `" "` for Standard).
    pub separator: &'a str,
    /// Clamp the exponent to be non-negative (Standard never shows `K^-1`).
    pub force_positive_exponent: bool,
}

/// The shared mantissa/exponent engine behind Scientific, Engineering, and Standard.
///
/// Splits `n` per `spec` into a mantissa in `[1, 10**steps)` and an exponent that is
/// a multiple of `steps`, formats each with the supplied closures, and joins them
/// with `spec.separator`. Handles mantissa roll-over (`9.999e3 -> 1e4`) and the
/// `exponent == 0` short-circuit.
///
/// The closures capture the per-call place counts (`opts.places` /
/// `opts.places_exponent`), which keeps this engine independent of `FormatOptions`.
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
    let real_base = 10f64.powi(spec.steps);

    let mut exponent = n.log(real_base).floor() * steps_f;
    if spec.force_positive_exponent {
        exponent = exponent.max(0.0);
    }
    // Base 10 throughout, so dividing out the exponent is just `pow10`.
    let mut mantissa = (*n / Decimal::pow10(exponent)).to_f64();

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
