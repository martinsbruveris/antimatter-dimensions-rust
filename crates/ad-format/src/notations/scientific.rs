//! Scientific notation (base 10, exponent step 1). Port step 3.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::options::FormatOptions;

pub(crate) struct Scientific;

impl NotationStrategy for Scientific {
    fn name(&self) -> &'static str {
        "Scientific"
    }

    fn format_decimal(
        &self,
        value: &Decimal,
        places: i32,
        places_exponent: i32,
        opts: &FormatOptions,
    ) -> String {
        self.format_base_ten_exponent(value, places, places_exponent, opts, 1)
    }
}
