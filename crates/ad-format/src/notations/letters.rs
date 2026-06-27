//! Letters notation (base-26 a…z exponent transcription). Port step 7, mirroring
//! the notations library's `CustomNotation` instantiated with the alphabet.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::mantissa::format_mantissa_base_ten;
use crate::options::FormatOptions;

const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

pub(crate) struct Letters;

impl NotationStrategy for Letters {
    fn name(&self) -> &'static str {
        "Letters"
    }

    fn format_decimal(
        &self,
        value: &Decimal,
        places: i32,
        _places_exponent: i32,
        _opts: &FormatOptions,
    ) -> String {
        // `CustomNotation.formatDecimal`: an engineering split (3-digit mantissa),
        // then the exponent transcribed into base-26 letters. `places_exponent` is
        // ignored, as are the exponent commas. The Letters separators are empty, so
        // mantissa and letters are simply concatenated.
        let (mantissa, exponent) = to_engineering(value);
        format!(
            "{}{}",
            format_mantissa_base_ten(mantissa, places),
            transcribe(exponent)
        )
    }
}

/// Port of `toEngineering`: rebase the value to a mantissa in `[1, 1000)` with an
/// exponent that is a multiple of 3. Returns `(mantissa, exponent)`.
fn to_engineering(value: &Decimal) -> (f64, i64) {
    let offset = value.exponent().rem_euclid(3);
    let mantissa = value.mantissa() * 10f64.powi(offset as i32);
    (mantissa, value.exponent() - offset)
}

/// Port of `CustomNotation.transcribe`: bijective base-26 encoding of
/// `exponent / 3` into letters (`a`, `b`, …, `z`, `aa`, `ab`, …).
fn transcribe(exponent: i64) -> String {
    let base = LETTERS.len() as i64;
    let mut n = exponent / 3;
    // Fast path for the common single-letter case (`a`..`z`); the loop below would
    // produce the same result but allocates a `Vec`.
    if n <= base {
        return (LETTERS[(n - 1) as usize] as char).to_string();
    }

    let mut letters = Vec::new();
    while n > base {
        let remainder = n % base;
        let letter_index = (if remainder == 0 { base } else { remainder }) - 1;
        letters.push(LETTERS[letter_index as usize]);
        n = (n - remainder) / base;
        if remainder == 0 {
            n -= 1;
        }
    }
    letters.push(LETTERS[(n - 1) as usize]);
    letters.reverse();
    String::from_utf8(letters).expect("LETTERS is ASCII")
}
