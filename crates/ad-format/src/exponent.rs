//! Exponent-rendering helpers — ports of the `exponentCommas` logic in the
//! notations library's `utils.ts`. The recursive `formatExponent` itself is a
//! default method on [`NotationStrategy`](crate::notations::NotationStrategy), since
//! it dispatches back into a notation's `format_decimal`.

use crate::options::ExponentCommas;

/// `exponent < min`: render the exponent plain, no commas. Port of
/// `noSpecialFormatting`.
pub(crate) fn no_special_formatting(exponent: f64, commas: &ExponentCommas) -> bool {
    exponent < commas.min as f64
}

/// `show && exponent < max`: render the exponent comma-grouped. Port of
/// `showCommas`.
pub(crate) fn show_commas(exponent: f64, commas: &ExponentCommas) -> bool {
    commas.show && exponent < commas.max as f64
}

/// The exponent is shown in full (plain or comma-grouped) rather than recursively
/// formatted in notation. Port of `isExponentFullyShown`.
pub(crate) fn is_exponent_fully_shown(exponent: f64, commas: &ExponentCommas) -> bool {
    no_special_formatting(exponent, commas) || show_commas(exponent, commas)
}

/// Render an integer-valued exponent as a base-10 string (port of the default
/// `specialFormat = n => n.toString()`).
pub(crate) fn exponent_to_string(exponent: f64) -> String {
    format!("{}", exponent as i64)
}

/// Group the integer part of `value` into comma-separated thousands. Port of
/// `formatWithCommas` for the (decimal-point-free) exponent case.
pub(crate) fn format_with_commas(value: &str) -> String {
    let (sign, digits) = match value.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", value),
    };
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let first = digits.len() % 3;
    for (i, ch) in digits.chars().enumerate() {
        if i != 0 && i >= first && (i - first) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{sign}{out}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commas_group_thousands() {
        assert_eq!(format_with_commas("100000"), "100,000");
        assert_eq!(format_with_commas("1234567"), "1,234,567");
        assert_eq!(format_with_commas("999"), "999");
        assert_eq!(format_with_commas("1000"), "1,000");
        assert_eq!(format_with_commas("-12345"), "-12,345");
    }
}
