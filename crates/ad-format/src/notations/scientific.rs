//! Scientific notation (base 10, exponent step 1). Port step 3.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::mantissa::{
    format_mantissa_base_ten, format_mantissa_with_exponent, MantissaSpec,
};
use crate::options::FormatOptions;

pub(crate) struct Scientific;

impl NotationStrategy for Scientific {
    fn name(&self) -> &'static str {
        "Scientific"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        // Base 10, step 1: mantissa in [1, 10), exponent rendered after an "e".
        format_mantissa_with_exponent(
            value,
            &MantissaSpec {
                base: 10.0,
                steps: 1,
                separator: "e",
                force_positive_exponent: false,
            },
            |m| format_mantissa_base_ten(m, opts.places),
            |exp| self.format_exponent(exp, opts),
        )
    }
}
