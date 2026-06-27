//! Exponent string-rendering helpers (ports from the notations library's
//! `utils.ts`). The `exponentCommas` threshold logic and recursive `formatExponent`
//! live on [`NotationStrategy::format_exponent`](crate::notations::NotationStrategy),
//! since the recursive case dispatches back into a notation's `format_decimal`.

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
mod tests;
