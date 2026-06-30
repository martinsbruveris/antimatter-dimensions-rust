//! Mixed engineering notation: Standard below 1e33, Engineering above.

use break_infinity::Decimal;

use super::{NotationStrategy, Standard};
use crate::mantissa::{format_mantissa, format_mantissa_with_exponent, MantissaSpec};
use crate::options::FormatOptions;

pub(crate) struct MixedEngineering;

impl NotationStrategy for MixedEngineering {
    fn name(&self) -> &'static str {
        "Mixed engineering"
    }

    fn format_decimal(&self, value: &Decimal, opts: &FormatOptions) -> String {
        // Below 1e33 Standard; at or above, Engineering (step 3) routed through our
        // own `format_exponent` so the recursive-exponent case stays Mixed. See
        // `MixedScientific` for the rationale.
        if value.exponent() < 33 {
            return Standard.format_decimal(value, opts);
        }
        format_mantissa_with_exponent(
            value,
            &MantissaSpec {
                steps: 3,
                separator: "e",
                force_positive_exponent: false,
            },
            |m| format_mantissa(m, opts.places),
            |exp| self.format_exponent(exp, opts),
        )
    }
}
