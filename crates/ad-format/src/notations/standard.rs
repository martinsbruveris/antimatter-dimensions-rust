//! Standard notation (K/M/B/T/Qa… abbreviations; `places` applies to the mantissa
//! only).

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::mantissa::{format_mantissa, format_mantissa_with_exponent, MantissaSpec};
use crate::options::FormatOptions;

const STANDARD_ABBREVIATIONS: [&str; 10] =
    ["K", "M", "B", "T", "Qa", "Qt", "Sx", "Sp", "Oc", "No"];

const STANDARD_PREFIXES: [[&str; 10]; 3] = [
    ["", "U", "D", "T", "Qa", "Qt", "Sx", "Sp", "O", "N"],
    ["", "Dc", "Vg", "Tg", "Qd", "Qi", "Se", "St", "Og", "Nn"],
    ["", "Ce", "Dn", "Tc", "Qe", "Qu", "Sc", "Si", "Oe", "Ne"],
];

const STANDARD_PREFIXES_2: [&str; 8] =
    ["", "MI-", "MC-", "NA-", "PC-", "FM-", "AT-", "ZP-"];

pub(crate) struct Standard;

impl NotationStrategy for Standard {
    fn name(&self) -> &'static str {
        "Standard"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        // steps 3 (mantissa in [1, 1000)), separator " ", forced non-negative
        // exponent; `abbreviate_standard` rescales the power-of-ten exponent into
        // thousands.
        format_mantissa_with_exponent(
            value,
            &MantissaSpec {
                steps: 3,
                separator: " ",
                force_positive_exponent: true,
            },
            |m| format_mantissa(m, opts.places),
            abbreviate_standard,
        )
    }
}

/// Turns an exponent into its letter abbreviation (`K`, `M`, …, `UDc`, …, `MI`).
/// The engine hands us a power-of-ten exponent (a multiple of 3); dividing by 3
/// recovers the thousands index the abbreviation is keyed by.
fn abbreviate_standard(raw_exp: f64) -> String {
    let exp = raw_exp as i64 / 3 - 1;
    if exp == -1 {
        return String::new();
    }
    if (exp as usize) < STANDARD_ABBREVIATIONS.len() {
        return STANDARD_ABBREVIATIONS[exp as usize].to_string();
    }

    let mut prefix: Vec<&str> = Vec::new();
    let mut e = exp;
    while e > 0 {
        prefix.push(STANDARD_PREFIXES[prefix.len() % 3][(e % 10) as usize]);
        e /= 10;
    }
    while !prefix.len().is_multiple_of(3) {
        prefix.push("");
    }

    let mut abbreviation = String::new();
    for i in (0..prefix.len() / 3).rev() {
        abbreviation.push_str(&prefix[i * 3..i * 3 + 3].concat());
        // Beyond the table only for values past break_infinity's range; treat as "".
        abbreviation.push_str(STANDARD_PREFIXES_2.get(i).copied().unwrap_or(""));
    }

    strip_trailing_dash(&strip_leading_u(&collapse_inner_dashes(&abbreviation)))
}

/// Collapse `-XX-` (a dash, two uppercase letters, a dash) down to a single `-`,
/// scanning non-overlapping and left to right (equivalent to `/-[A-Z]{2}-/g`).
fn collapse_inner_dashes(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'-'
            && i + 3 < b.len()
            && b[i + 1].is_ascii_uppercase()
            && b[i + 2].is_ascii_uppercase()
            && b[i + 3] == b'-'
        {
            out.push('-');
            i += 4;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// Drop a `U` directly before `XX-` (equivalent to `/U([A-Z]{2}-)/g`).
fn strip_leading_u(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'U'
            && i + 3 < b.len()
            && b[i + 1].is_ascii_uppercase()
            && b[i + 2].is_ascii_uppercase()
            && b[i + 3] == b'-'
        {
            out.push(b[i + 1] as char);
            out.push(b[i + 2] as char);
            out.push('-');
            i += 4;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// Drop a single trailing `-`.
fn strip_trailing_dash(s: &str) -> String {
    s.strip_suffix('-').unwrap_or(s).to_string()
}
