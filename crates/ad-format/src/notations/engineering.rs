//! Engineering notation (base 10, exponent step 3). Port step 4.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::mantissa::{
    format_mantissa_base_ten, format_mantissa_with_exponent, MantissaSpec,
};
use crate::options::FormatOptions;

pub(crate) struct Engineering;

impl NotationStrategy for Engineering {
    fn name(&self) -> &'static str {
        "Engineering"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        // Base 10, step 3: forces the exponent to multiples of 3 (mantissa in
        // [1, 1000)), exponent rendered after an "e".
        format_mantissa_with_exponent(
            value,
            &MantissaSpec {
                base: 10.0,
                steps: 3,
                separator: "e",
                force_positive_exponent: false,
            },
            |m| format_mantissa_base_ten(m, opts.places),
            |exp| self.format_exponent(exp, opts),
        )
    }
}
