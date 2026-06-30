//! Exponent string-rendering helpers. The exponent-display threshold logic and the
//! recursive case live on
//! [`NotationStrategy::format_exponent`](crate::notations::NotationStrategy), since
//! the recursion dispatches back into a notation's `format_decimal`.

/// Group the integer part of `value` into comma-separated thousands. Any fractional
/// part (e.g. the `∞`-count in Infinity notation) is left untouched, matching the
/// game's `formatWithCommas` which only groups the digits before the decimal point.
pub(crate) fn format_with_commas(value: &str) -> String {
    let (int_part, frac) = match value.split_once('.') {
        Some((int_part, frac)) => (int_part, Some(frac)),
        None => (value, None),
    };
    let (sign, digits) = match int_part.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", int_part),
    };
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let first = digits.len() % 3;
    for (i, ch) in digits.chars().enumerate() {
        if i != 0 && i >= first && (i - first) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    match frac {
        Some(frac) => format!("{sign}{out}.{frac}"),
        None => format!("{sign}{out}"),
    }
}

#[cfg(test)]
mod tests;
