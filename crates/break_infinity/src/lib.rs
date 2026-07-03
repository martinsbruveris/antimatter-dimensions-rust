use std::cmp::Ordering::{self, *};
use std::f64::consts::{E, LN_10, LOG2_10, PI};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::num::{ParseFloatError, ParseIntError};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

#[macro_use]
mod macros;

#[cfg(feature = "serde")]
pub mod serde_string;

#[cfg(test)]
mod tests;

// Largest integer that can be safely represented in an f64, equals 2^53 - 1.
pub const F64_MAX_SAFE_INT: f64 = 9007199254740991.0;

// Maximum number of significant digits that can be represented in an f64, equals 17.
// Actually, f64 can represent ~15.9 digits. Technically, 17 is the maximum number of
// digits required to distinguish two f64 values.
pub const MAX_SIGNIFICANT_DIGITS: u32 = 17;

// Limit at which we consider numbers to be infinite.
pub const EXP_INF_THRESHOLD: i64 = 9_000_000_000_000_000;

/// Tolerance used for conversion to f64 to compensate for floating-point error.
pub const ROUND_TOLERANCE: f64 = 1e-12;

/// Smallest exponent that can appear in an f64, though not all mantissas are valid.
pub const F64_EXP_MIN: i32 = -324;

/// Largest exponent that can appear in an f64, though not all mantissas are valid.
pub const F64_EXP_MAX: i32 = 308;

// It might be worth turning this into a build script and compute it at compile time.
lazy_static::lazy_static! {
    static ref CACHED_POWERS: [f64; CACHED_POWERS_LEN] =
        std::array::from_fn(|i| 10.0_f64.powi(i as i32 + F64_EXP_MIN));
}

const CACHED_POWERS_LEN: usize = (F64_EXP_MAX - F64_EXP_MIN + 1) as usize;

/// Returns the power of 10 with the given exponent from the cache.
fn power_of_10(power: i64) -> f64 {
    CACHED_POWERS[(power - F64_EXP_MIN as i64) as usize]
}

/// Pads the given string with the fill string to the given max length.
pub fn pad_end(string: String, max_length: u32, fill_string: &str) -> String {
    let length = string.chars().count() as u32;
    if length >= max_length {
        return string;
    }

    let fill = if fill_string.is_empty() {
        " "
    } else {
        fill_string
    };
    let fill_len = (max_length - length) as usize;
    let padding: String = fill.chars().cycle().take(fill_len).collect();

    format!("{}{}", string, padding)
}

/// Formats the given number to a string with given number of significant digits.
pub fn to_str_fixed(num: f64, places: u32) -> String {
    format!("{:.*}", places as usize, num)
}

/// Formats the given number to given number of significant digits.
pub fn to_f64_fixed(num: f64, places: u32) -> f64 {
    to_str_fixed(num, places).parse::<f64>().unwrap()
}

/// A struct representing a decimal number, which can reach a maximum of 1e9e15
/// instead of `f64`'s maximum of 1.79e308.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Decimal {
    m: f64,
    e: i64,
}

impl Display for Decimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f64::is_nan(self.m) {
            return write!(f, "NaN");
        } else if self.e >= EXP_INF_THRESHOLD {
            return if self.m > 0.0 {
                write!(f, "Infinity")
            } else {
                write!(f, "-Infinity")
            };
        } else if self.e <= -EXP_INF_THRESHOLD || self.m == 0.0 {
            return write!(f, "0");
        } else if self.e < 21 && self.e > -7 {
            return if let Some(places) = f.precision() {
                write!(f, "{:.*}", places, self.to_f64().to_string())
            } else {
                write!(f, "{}", self.to_f64())
            };
        }

        let form = if let Some(places) = f.precision() {
            self.to_str_exp(places as u32)
        } else {
            self.to_str_exp(16)
        };

        write!(f, "{}", form)
    }
}

impl Add<Decimal> for Decimal {
    type Output = Decimal;

    fn add(self, other: Decimal) -> Decimal {
        // Figure out which is bigger, shrink the mantissa of the smaller
        // by the difference in exponents, add mantissas, normalize and return
        // TODO: Optimizations and simplification may be possible, see
        // https://github.com/Patashu/break_infinity.js/issues/8
        if self.m == 0.0 {
            return other;
        } else if other.m == 0.0 {
            return self;
        }

        let larger;
        let smaller;

        if self.e >= other.e {
            larger = self;
            smaller = other;
        } else {
            larger = other;
            smaller = self;
        }

        if larger.e - smaller.e > MAX_SIGNIFICANT_DIGITS as i64 {
            return larger;
        }

        Decimal::new(
            (1e14 * larger.m) + 1e14 * smaller.m * power_of_10(smaller.e - larger.e),
            larger.e - 14,
        )
    }
}

forward_ref_binop!(impl Add<Decimal> for Decimal, add);

impl AddAssign<Decimal> for Decimal {
    fn add_assign(&mut self, rhs: Decimal) {
        *self = *self + rhs;
    }
}

forward_ref_op_assign!(impl AddAssign<Decimal> for Decimal, add_assign);

impl Sub<Decimal> for Decimal {
    type Output = Decimal;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Decimal) -> Decimal {
        self + rhs.neg()
    }
}

forward_ref_binop!(impl Sub<Decimal> for Decimal, sub);

impl SubAssign<Decimal> for Decimal {
    fn sub_assign(&mut self, rhs: Decimal) {
        *self = *self - rhs;
    }
}

forward_ref_op_assign!(impl SubAssign<Decimal> for Decimal, sub_assign);

impl Mul<Decimal> for Decimal {
    type Output = Decimal;

    fn mul(self, rhs: Decimal) -> Decimal {
        Decimal::new(self.m * rhs.m, self.e + rhs.e)
    }
}

forward_ref_binop!(impl Mul<Decimal> for Decimal, mul);

impl MulAssign<Decimal> for Decimal {
    fn mul_assign(&mut self, rhs: Decimal) {
        *self = *self * rhs;
    }
}

forward_ref_op_assign!(impl MulAssign<Decimal> for Decimal, mul_assign);

impl Div<Decimal> for Decimal {
    type Output = Decimal;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Decimal) -> Decimal {
        self * rhs.recip()
    }
}

forward_ref_binop!(impl Div<Decimal> for Decimal, div);

impl DivAssign<Decimal> for Decimal {
    fn div_assign(&mut self, rhs: Decimal) {
        *self = *self / rhs;
    }
}

forward_ref_op_assign!(impl DivAssign<Decimal> for Decimal, div_assign);

impl Neg for &Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal::new(-self.m, self.e)
    }
}

impl Neg for Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal::new(-self.m, self.e)
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, decimal: &Self) -> Option<Ordering> {
        /*
        From smallest to largest:
        -Infinity
        -3e100
        -1e100
        -3e99
        -1e99
        -3e0
        -1e0
        -3e-99
        -1e-99
        -3e-100
        -1e-100
        0
        1e-100
        3e-100
        1e-99
        3e-99
        1e0
        3e0
        1e99
        3e99
        1e100
        3e100
        Infinity
        */

        if f64::is_nan(self.m) || f64::is_nan(decimal.m) {
            None
        } else if (f64::is_infinite(self.m) && self.m.is_sign_negative())
            || (f64::is_infinite(decimal.m) && decimal.m.is_sign_positive())
        {
            Some(Less)
        } else if (f64::is_infinite(self.m) && self.m.is_sign_negative())
            || (f64::is_infinite(decimal.m) && decimal.m.is_sign_positive())
        {
            Some(Greater)
        } else if self.m == 0.0 {
            if decimal.m == 0.0 {
                Some(Equal)
            } else if decimal.m < 0.0 {
                Some(Greater)
            } else {
                Some(Less)
            }
        } else if decimal.m == 0.0 {
            if self.m < 0.0 {
                Some(Less)
            } else {
                Some(Greater)
            }
        } else if self.m > 0.0 {
            if self.e > decimal.e || decimal.m < 0.0 {
                Some(Greater)
            } else if self.e < decimal.e {
                Some(Less)
            } else if self.m > decimal.m {
                Some(Greater)
            } else if self.m < decimal.m {
                Some(Less)
            } else {
                Some(Equal)
            }
        } else if self.e > decimal.e || decimal.m > 0.0 {
            Some(Less)
        } else if self.m > decimal.m || self.e < decimal.e {
            Some(Greater)
        } else if self.m < decimal.m {
            Some(Less)
        } else {
            Some(Equal)
        }
    }
}

impl PartialEq<Decimal> for Decimal {
    fn eq(&self, decimal: &Decimal) -> bool {
        if self.m == 0.0 && decimal.m == 0.0 {
            return true;
        }
        self.m == decimal.m && self.e == decimal.e
    }
}

impl Eq for Decimal {}

#[derive(Debug)]
pub enum ParseDecimalError {
    Float(ParseFloatError),
    Int(ParseIntError),
}

impl fmt::Display for ParseDecimalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParseDecimalError::Float(e) => write!(f, "{}", e),
            ParseDecimalError::Int(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ParseDecimalError {}

impl From<ParseFloatError> for ParseDecimalError {
    fn from(e: ParseFloatError) -> Self {
        ParseDecimalError::Float(e)
    }
}

impl From<ParseIntError> for ParseDecimalError {
    fn from(e: ParseIntError) -> Self {
        ParseDecimalError::Int(e)
    }
}

impl FromStr for Decimal {
    type Err = ParseDecimalError;

    fn from_str(string: &str) -> Result<Decimal, ParseDecimalError> {
        if let Some((mantissa, exponent)) = string.split_once('e') {
            let decimal = Decimal {
                m: mantissa.parse()?,
                e: exponent.parse()?,
            };

            Ok(decimal.normalize())
        } else if string == "NaN" {
            Ok(Decimal::NAN)
        } else {
            string
                .parse::<f64>()
                .map(Decimal::from_float)
                .map_err(ParseDecimalError::Float)
        }
    }
}

impl Default for Decimal {
    fn default() -> Self {
        Decimal::ZERO
    }
}

// This allows converting virtually any number to a Decimal.
impl_from!(i8);
impl_from!(i16);
impl_from!(i32);
impl_from!(i64);
impl_from!(i128);
impl_from!(isize);
impl_from!(u8);
impl_from!(u16);
impl_from!(u32);
impl_from!(u64);
impl_from!(u128);
impl_from!(usize);
impl_from!(f32);
impl_from!(f64);

impl Decimal {
    pub const EPS: Decimal = Decimal {
        m: 1.0,
        e: -EXP_INF_THRESHOLD,
    };
    pub const MAX_VALUE: Decimal = Decimal {
        m: 1.0,
        e: EXP_INF_THRESHOLD,
    };
    pub const MIN_VALUE: Decimal = Decimal {
        m: -1.0,
        e: EXP_INF_THRESHOLD,
    };
    pub const ZERO: Decimal = Decimal { m: 0.0, e: 0 };
    pub const ONE: Decimal = Decimal { m: 1.0, e: 0 };
    pub const NEGATIVE_ONE: Decimal = Decimal { m: -1.0, e: 0 };
    pub const E: Decimal = Decimal { m: E, e: 0 };
    pub const NAN: Decimal = Decimal { m: f64::NAN, e: 0 };

    /// Creates a new instance of Decimal with the given mantissa and exponent,
    /// normalizing them.
    pub fn new(m: f64, e: i64) -> Decimal {
        Decimal { m, e }.normalize()
    }

    /// Creates a `Decimal` from a mantissa and exponent **without normalizing**.
    ///
    /// This is `const`, so it can initialize `const`/`static` items (which
    /// `new` cannot, as normalization is not const-evaluable). The caller must
    /// pass an already-normalized mantissa — i.e. in `[1, 10)`, `(-10, -1]`, or
    /// `0` — otherwise arithmetic on the result is undefined. For runtime
    /// values, prefer `new`, which normalizes.
    pub const fn new_unchecked(m: f64, e: i64) -> Decimal {
        Decimal { m, e }
    }

    /// Creates a new instance of Decimal from an f64 value.
    pub fn from_float(value: f64) -> Decimal {
        Decimal::new(value, 0)
    }

    /// Calculates 10^power as a Decimal.
    pub fn pow10(power: f64) -> Decimal {
        let t = power.trunc(); // Integer part
        let f = power.fract(); // Fractional part

        if f == 0.0 {
            Decimal {
                m: 1.0,
                e: power as i64,
            }
        } else {
            Decimal::new(10.0_f64.powf(f), t as i64)
        }
    }

    /// Normalizes the mantissa to be in the range [1, 10) (or (-10, -1] for negatives)
    /// and adjusts the exponent accordingly.
    ///
    /// Handles all edge cases:
    /// - NaN mantissa → returns `Decimal::NAN`
    /// - Infinite mantissa → returns `MAX_VALUE` (with correct sign)
    /// - Zero mantissa → returns `Decimal::ZERO`
    /// - Result exceeds `MAX_VALUE` → clips to `MAX_VALUE` (with correct sign)
    /// - Result is between 0 and `EPS` → returns `Decimal::ZERO`
    fn normalize(&self) -> Decimal {
        if f64::is_nan(self.m) {
            return Decimal::NAN;
        } else if f64::is_infinite(self.m) {
            return if self.m > 0.0 {
                Decimal::MAX_VALUE
            } else {
                Decimal::MIN_VALUE
            };
        } else if self.m == 0.0 {
            return Decimal::ZERO;
        }

        // Already normalized: mantissa in [1, 10) or (-10, -1]
        if (1.0 <= self.m && self.m < 10.0) || (-10.0 < self.m && self.m <= -1.0) {
            // Still need to check if the exponent puts us out of range
            return self.clamp_to_range();
        }

        // Compute the adjustment to bring mantissa into [1, 10)
        let m_exp = self.m.abs().log10().floor();
        let new_m = if (m_exp as i32) == F64_EXP_MIN {
            // Special case for the smallest f64 exponent to avoid division by zero
            self.m * 10.0 / 1e-323
        } else {
            self.m / power_of_10(m_exp as i64)
        };
        // Round to 15 significant digits to remove floating-point noise from division
        let new_m = (new_m * 1e15).round() / 1e15;
        let new_e = self.e + m_exp as i64;

        let result = Decimal { m: new_m, e: new_e };
        result.clamp_to_range()
    }

    /// Clamps the Decimal to the representable range, assuming the mantissa is already
    /// normalized (in [1, 10) or (-10, -1]).
    ///
    /// - Exponent above `EXP_INF_THRESHOLD` → clips to `MAX_VALUE` (with correct sign)
    /// - Exponent below `-EXP_INF_THRESHOLD` → returns `ZERO`
    #[inline]
    fn clamp_to_range(&self) -> Decimal {
        if self.e >= EXP_INF_THRESHOLD {
            // Overflow: clip to MAX_VALUE with correct sign
            return if self.m > 0.0 {
                Decimal::MAX_VALUE
            } else {
                Decimal {
                    m: -1.0,
                    e: EXP_INF_THRESHOLD,
                }
            };
        }

        if self.e < -EXP_INF_THRESHOLD {
            // Underflow: too small to represent, collapse to zero
            return Decimal::ZERO;
        }

        *self
    }

    /// Returns the mantissa (normalized to [1, 10), (-10, -1], or 0).
    pub fn mantissa(&self) -> f64 {
        self.m
    }

    /// Returns the exponent.
    pub fn exponent(&self) -> i64 {
        self.e
    }

    /// Converts the Decimal to an f64.
    pub fn to_f64(&self) -> f64 {
        if self.e > F64_EXP_MAX as i64 {
            return if self.m > 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
        } else if self.e < F64_EXP_MIN as i64 {
            return 0.0;
        } else if self.e == F64_EXP_MIN as i64 {
            return if self.m > 0.0 { 5e-324 } else { -5e-324 };
        }

        let result: f64 = self.m * power_of_10(self.e);
        if !f64::is_finite(result) || self.e < 0 {
            return result;
        }

        // Problem: new(116.0).to_f64() returns 115.99999999999999. It's clear that
        // if to_f64() is VERY close to an integer, we want exactly the integer.
        // But it's not clear how to specifically write that. So, instead we look at
        // the difference to the rounded value.
        let rounded_result = result.round();
        if (rounded_result - result).abs() < ROUND_TOLERANCE {
            return rounded_result;
        }

        result
    }

    /// Helper function to handle all non-finite cases.
    #[inline(always)]
    fn to_str_non_finite(self) -> Option<String> {
        if f64::is_nan(self.m) {
            Some(String::from("NaN"))
        } else if self.e >= EXP_INF_THRESHOLD {
            if self.m > 0.0 {
                Some(String::from("Infinity"))
            } else {
                Some(String::from("-Infinity"))
            }
        } else {
            None
        }
    }

    /// Converts the Decimal into a string with the scientific notation.
    pub fn to_str_exp(&self, mut places: u32) -> String {
        if let Some(string) = self.to_str_non_finite() {
            return string;
        }

        let tmp = pad_end(String::from("."), places + 1, "0");
        // 1) exponent is < 308 and > -324: use basic to_str_fixed
        // 2) everything else: we have to do it ourselves!
        if self.e <= -EXP_INF_THRESHOLD || self.m == 0.0 {
            let str = if places > 0 { &tmp } else { "" };
            return format!("0{}e+0", str);
        } else if !f32::is_finite(places as f32) {
            places = MAX_SIGNIFICANT_DIGITS;
        }

        let len = places + 1;
        let num_digits = self.m.abs().log10().max(1.0) as u32;
        let rounded = (self.m * 10.0_f64.powi(len as i32 - num_digits as i32)).round()
            * 10.0_f64.powi(num_digits as i32 - len as i32);

        let mantissa = to_str_fixed(rounded, len - num_digits);
        let sign = if self.e >= 0 { "+" } else { "" };
        format!("{}e{}{}", mantissa, sign, self.e)
    }

    /// Converts the Decimal into a string with the fixed notation.
    pub fn to_str_fixed(&self, places: u32) -> String {
        if let Some(string) = self.to_str_non_finite() {
            return string;
        }

        let tmp = pad_end(String::from("."), places + 1, "0");
        if self.e <= -EXP_INF_THRESHOLD || self.m == 0.0 {
            // Two Cases:
            // 1) exponent is 17 or greater: just print out mantissa with the
            //    appropriate number of zeroes after it
            // 2) exponent is 16 or less: use basic to_str_fixed
            let str = if places > 0 { &tmp } else { "" };
            return format!("0{}", str);
        } else if self.e >= MAX_SIGNIFICANT_DIGITS as i64 {
            let str = pad_end(
                self.m.to_string().replace('.', ""),
                (self.e + 1) as u32,
                "0",
            );
            let decimals = if places > 0 { &tmp } else { "" };
            return format!("{}{}", str, decimals);
        }

        to_str_fixed(self.to_f64(), places)
    }

    /// Converts the Decimal into a string with the scientific notation if the exponent
    /// is greater than the precision.
    pub fn to_str_precision(&self, places: u32) -> String {
        if self.e <= -7 {
            return self.to_str_exp(places - 1);
        }

        if (places as i64) > self.e {
            return self.to_str_fixed((places as i64 - self.e - 1) as u32);
        }

        self.to_str_exp(places - 1)
    }

    /// Returns the mantissa with the specified precision.
    pub fn mantissa_with_decimal_places(&self, places: u32) -> f64 {
        if f64::is_nan(self.m) || self.m == 0.0 {
            return self.m;
        }
        to_f64_fixed(self.m, places)
    }

    /// Returns the absolute value of the Decimal.
    pub fn abs(&self) -> Decimal {
        Decimal {
            m: self.m.abs(),
            e: self.e,
        }
    }

    /// Returns the sign of the Decimal, according to [f64::signum].
    pub fn sign(&self) -> f64 {
        self.m.signum()
    }

    /// Rounds the Decimal. If the exponent is large, the number is effectively already
    /// an integer, so return it as is.
    pub fn round(&self) -> Decimal {
        if self.e < -1 {
            Decimal::ZERO
        } else if self.e < MAX_SIGNIFICANT_DIGITS as i64 {
            Decimal::from_float(self.to_f64().round())
        } else {
            *self
        }
    }

    /// Truncates the Decimal. If the exponent is large, the number is effectively
    /// already an integer, so return it as is.
    pub fn trunc(&self) -> Decimal {
        if self.e < 0 {
            Decimal::ZERO
        } else if self.e < MAX_SIGNIFICANT_DIGITS as i64 {
            Decimal::from_float(self.to_f64().trunc())
        } else {
            *self
        }
    }

    /// Floors the Decimal. If the exponent is large, the number is effectively
    /// already an integer, so return it as is.
    pub fn floor(&self) -> Decimal {
        if self.e < -1 {
            if self.sign() > 0.0 {
                Decimal::ZERO
            } else {
                Decimal::NEGATIVE_ONE
            }
        } else if self.e < MAX_SIGNIFICANT_DIGITS as i64 {
            Decimal::from_float(self.to_f64().floor())
        } else {
            *self
        }
    }

    /// Rounds the Decimal to its ceiling. If the exponent is large, the number is
    /// effectively already an integer, so return it as is.
    pub fn ceil(&self) -> Decimal {
        if self.e < -1 {
            if self.m == 0.0 || self.sign() < 0.0 {
                Decimal::ZERO
            } else {
                Decimal::ONE
            }
        } else if self.e < MAX_SIGNIFICANT_DIGITS as i64 {
            Decimal::from_float(self.to_f64().ceil())
        } else {
            *self
        }
    }

    /// Returns the reciprocal of the Decimal.
    pub fn recip(&self) -> Decimal {
        Decimal::new(1.0 / self.m, -self.e)
    }

    pub fn max(&self, other: &Decimal) -> Decimal {
        if self > other {
            *self
        } else {
            *other
        }
    }

    pub fn min(&self, other: &Decimal) -> Decimal {
        if self < other {
            *self
        } else {
            *other
        }
    }

    pub fn clamp(&self, min: &Decimal, max: &Decimal) -> Decimal {
        self.max(min).min(max)
    }

    pub fn cmp_tolerance(
        &self,
        decimal: &Decimal,
        tolerance: &Decimal,
    ) -> Option<Ordering> {
        if self.eq_tolerance(decimal, tolerance) {
            Some(Equal)
        } else {
            self.partial_cmp(decimal)
        }
    }

    /// Tolerance is a relative tolerance, multiplied by the greater of the magnitudes
    /// of the two arguments. For example, if you put in 1e-9, then any number closer
    /// to the larger number than (larger number) * 1e-9 will be considered equal.
    pub fn eq_tolerance(&self, decimal: &Decimal, tolerance: &Decimal) -> bool {
        // return abs(a-b) <= tolerance * max(abs(a), abs(b))
        (self - decimal)
            .abs()
            .le(&self.abs().max(&(decimal.abs() * tolerance)))
    }

    pub fn neq_tolerance(&self, decimal: &Decimal, tolerance: &Decimal) -> bool {
        !self.eq_tolerance(decimal, tolerance)
    }

    pub fn lt_tolerance(&self, decimal: &Decimal, tolerance: &Decimal) -> bool {
        !self.eq_tolerance(decimal, tolerance) && self.lt(decimal)
    }
    pub fn le_tolerance(&self, decimal: &Decimal, tolerance: &Decimal) -> bool {
        self.eq_tolerance(decimal, tolerance) || self.lt(decimal)
    }

    pub fn gt_tolerance(&self, decimal: &Decimal, tolerance: &Decimal) -> bool {
        !self.eq_tolerance(decimal, tolerance) && self.gt(decimal)
    }
    pub fn ge_tolerance(&self, decimal: &Decimal, tolerance: &Decimal) -> bool {
        self.eq_tolerance(decimal, tolerance) || self.gt(decimal)
    }

    pub fn log10(&self) -> f64 {
        self.e as f64 + self.m.log10()
    }

    pub fn abs_log10(&self) -> f64 {
        self.e as f64 + self.m.abs().log10()
    }

    pub fn pos_log10(&self) -> f64 {
        if self.m <= 0.0 || self.e < 0 {
            0.0
        } else {
            self.log10()
        }
    }

    pub fn log(&self, base: f64) -> f64 {
        // UN-SAFETY: Most incremental game cases are log(number>=1, base>=2). We assume
        // this to be true and thus only need to return a number, not a Decimal,
        self.log10() / base.log10()
    }

    pub fn log2(&self) -> f64 {
        LOG2_10 * self.log10()
    }

    pub fn ln(&self) -> f64 {
        LN_10 * self.log10()
    }

    /// Raises the Decimal to the power of the given Decimal.
    pub fn pow(&self, power: &Decimal) -> Decimal {
        if self.m == 0.0 {
            return *self;
        }

        let n = power.to_f64();
        let new_e = self.e as f64 * n;

        // Fast track: only when `e * n` is a safe *integer*, so the exponent needs
        // no adjustment and the whole result lives in the mantissa's `^n`. This
        // mirrors break_infinity.js's `Number.isSafeInteger(temp)` guard — a
        // fractional `new_e` (e.g. `1e10 ^ 0.05` → `e * n = 0.5`) must NOT take
        // this path, since `new_e as i64` would truncate the fractional exponent
        // and silently drop a factor of `10^frac`. Such cases fall through to the
        // logarithmic path below, which handles them correctly.
        if new_e.abs() < F64_MAX_SAFE_INT && new_e.fract() == 0.0 {
            let new_m = self.m.powf(n);
            if new_m.is_finite() && new_m != 0.0 {
                return Decimal::new(new_m, new_e as i64);
            }
        }

        // General path: compute via logarithms
        let result = Decimal::pow10(n * self.abs_log10());

        if self.sign() == -1.0 {
            if n.fract() != 0.0 {
                Decimal::NAN
            } else if n as i64 % 2 == 0 {
                result
            } else {
                result.neg()
            }
        } else {
            result
        }
    }

    pub fn factorial(&self) -> Decimal {
        //  Using Stirling's Approximation.
        //  https://en.wikipedia.org/wiki/Stirling%27s_approximation#Versions_suitable_for_calculators
        let n = self.to_f64() + 1.0;
        Decimal::from_float(n / E * (n * f64::sinh(1.0 / n) + 1.0 / (810.0 * n.powi(6))))
            .pow(&Decimal::from_float(n))
            * Decimal::from_float(f64::sqrt(2.0 * PI / n))
    }

    pub fn exp(&self) -> Decimal {
        // Fast track: if -706 < this < 709, we can use regular exp.
        let number = self.to_f64();
        if -706.0 < number && number < 709.0 {
            return Decimal::from_float(f64::exp(number));
        }
        Decimal::E.pow(self)
    }

    pub fn sqr(&self) -> Decimal {
        Decimal::new(self.m.powi(2), self.e * 2)
    }

    pub fn sqrt(&self) -> Decimal {
        if self.m < 0.0 {
            return Decimal::NAN;
        }

        let new_m = if self.e % 2 != 0 {
            (self.m * 10.0).sqrt()
        } else {
            self.m.sqrt()
        };

        Decimal::new(new_m, self.e.div_euclid(2))
    }

    pub fn cube(&self) -> Decimal {
        Decimal::new(self.m.powi(3), self.e * 3)
    }

    pub fn cbrt(&self) -> Decimal {
        let remainder = self.e.rem_euclid(3) as i32;
        let mantissa = (self.m * 10.0_f64.powi(remainder)).cbrt();
        Decimal::new(mantissa, self.e.div_euclid(3))
    }

    // Some hyperbolic trigonometry functions that happen to be easy
    pub fn sinh(&self) -> Decimal {
        (self.exp() - self.neg().exp()) / Decimal::from_float(2.0)
    }
    pub fn cosh(&self) -> Decimal {
        (self.exp() + self.neg().exp()) / Decimal::from_float(2.0)
    }
    pub fn tanh(&self) -> Decimal {
        self.sinh() / self.cosh()
    }

    pub fn asinh(&self) -> f64 {
        (self + (self.sqr() + Decimal::from_float(1.0)).sqrt()).ln()
    }
    pub fn acosh(&self) -> f64 {
        (self + (self.sqr() - Decimal::from_float(1.0)).sqrt()).ln()
    }
    pub fn atanh(&self) -> f64 {
        if self.abs().ge(&Decimal::from_float(1.0)) {
            return f64::NAN;
        }

        ((Decimal::from_float(1.0) + self) / (Decimal::from_float(1.0) - self)).ln()
            / 2.0
    }

    /// Returns the number of decimal places in the number.
    pub fn dp(&self) -> Option<i32> {
        if !f64::is_finite(self.m) {
            return None;
        } else if self.e >= MAX_SIGNIFICANT_DIGITS as i64 {
            return Some(0);
        }

        let mut places = -(self.e as i32);

        for i in 0..MAX_SIGNIFICANT_DIGITS {
            let scale = 10.0_f64.powi(i as i32);
            if ((self.m * scale).round() / scale - self.m).abs() <= ROUND_TOLERANCE {
                break;
            }
            places += 1;
        }

        Some(places.max(0))
    }
}
