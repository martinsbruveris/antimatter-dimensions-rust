//! Infinity notation: the value counted in multiples of the float `MAX_VALUE`.

use break_infinity::Decimal;

use super::NotationStrategy;
use crate::exponent::format_with_commas;
use crate::mantissa::format_mantissa;
use crate::options::FormatOptions;

pub(crate) struct Infinity;

impl NotationStrategy for Infinity {
    fn name(&self) -> &'static str {
        "Infinity"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        // How many "infinities" the value is, where one infinity is the largest
        // representable f64 (`Number.MAX_VALUE` in the game). Smaller counts get an
        // extra decimal place; the count is then comma-grouped and `∞`-suffixed.
        let infinities = value.log10() / f64::MAX.log10();
        let inf_places: u32 = if infinities < 1000.0 { 4 } else { 3 };
        let formatted = format_mantissa(infinities, inf_places.max(opts.places));
        if opts.exponent_display.show {
            format!("{}\u{221e}", format_with_commas(&formatted))
        } else {
            format!("{formatted}\u{221e}")
        }
    }
}
