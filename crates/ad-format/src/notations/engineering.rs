//! Engineering notation (base 10, exponent step 3). Port step 4.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::options::FormatOptions;

pub(crate) struct Engineering;

impl NotationStrategy for Engineering {
    fn name(&self) -> &'static str {
        "Engineering"
    }

    fn format_decimal(
        &self,
        value: &Decimal,
        places: i32,
        places_exponent: i32,
        opts: &FormatOptions,
    ) -> String {
        // Steps of 3 force the exponent to multiples of 3 (mantissa in [1, 1000)).
        self.format_base_ten_exponent(value, places, places_exponent, opts, 3)
    }
}
