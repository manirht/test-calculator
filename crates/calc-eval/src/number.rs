//! Exact rational arithmetic on `i128` with checked operations.
//!
//! Every constructor reduces to lowest terms with a positive denominator, so equal values
//! compare equal field by field. `i128::MIN` is rejected so negation and `abs` never overflow.

// The arithmetic methods are fallible, so they cannot implement the std operator traits.
#![allow(clippy::should_implement_trait)]

use crate::error::{ErrorCode, EvalError};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundMode {
    /// Round half away from zero.
    HalfUp,
    /// Round half to the nearest even digit.
    HalfEven,
    /// Drop the remainder, toward zero.
    Truncate,
}

impl FromStr for RoundMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "half-up" => Ok(RoundMode::HalfUp),
            "half-even" => Ok(RoundMode::HalfEven),
            "truncate" => Ok(RoundMode::Truncate),
            other => Err(format!("unknown rounding mode '{other}'")),
        }
    }
}

fn overflow() -> EvalError {
    EvalError::new(ErrorCode::Overflow, "arithmetic overflow")
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn pow10(n: u32) -> Result<i128, EvalError> {
    10i128.checked_pow(n).ok_or_else(overflow)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    num: i128,
    den: i128,
}

impl Rational {
    pub const ZERO: Rational = Rational { num: 0, den: 1 };
    pub const ONE: Rational = Rational { num: 1, den: 1 };

    pub fn new(num: i128, den: i128) -> Result<Rational, EvalError> {
        if den == 0 {
            return Err(EvalError::new(ErrorCode::DivByZero, "division by zero"));
        }
        if num == i128::MIN || den == i128::MIN {
            return Err(overflow());
        }
        let g = gcd(num, den);
        let (mut n, mut d) = (num / g, den / g);
        if d < 0 {
            n = -n;
            d = -d;
        }
        Ok(Rational { num: n, den: d })
    }

    pub fn from_int(n: i128) -> Rational {
        Rational { num: n, den: 1 }
    }

    pub fn num(&self) -> i128 {
        self.num
    }

    pub fn den(&self) -> i128 {
        self.den
    }

    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    pub fn is_integer(&self) -> bool {
        self.den == 1
    }

    pub fn is_negative(&self) -> bool {
        self.num < 0
    }

    pub fn signum(&self) -> i128 {
        self.num.signum()
    }

    pub fn neg(self) -> Result<Rational, EvalError> {
        Rational::new(-self.num, self.den)
    }

    pub fn abs(self) -> Result<Rational, EvalError> {
        if self.is_negative() {
            self.neg()
        } else {
            Ok(self)
        }
    }

    pub fn add(self, other: Rational) -> Result<Rational, EvalError> {
        let g = gcd(self.den, other.den);
        let l = other.den / g;
        let r = self.den / g;
        let a = self.num.checked_mul(l).ok_or_else(overflow)?;
        let b = other.num.checked_mul(r).ok_or_else(overflow)?;
        let n = a.checked_add(b).ok_or_else(overflow)?;
        let d = self.den.checked_mul(l).ok_or_else(overflow)?;
        Rational::new(n, d)
    }

    pub fn sub(self, other: Rational) -> Result<Rational, EvalError> {
        self.add(other.neg()?)
    }

    pub fn mul(self, other: Rational) -> Result<Rational, EvalError> {
        let g1 = gcd(self.num, other.den);
        let g2 = gcd(other.num, self.den);
        let n = (self.num / g1).checked_mul(other.num / g2).ok_or_else(overflow)?;
        let d = (self.den / g2).checked_mul(other.den / g1).ok_or_else(overflow)?;
        Rational::new(n, d)
    }

    pub fn div(self, other: Rational) -> Result<Rational, EvalError> {
        if other.is_zero() {
            return Err(EvalError::new(ErrorCode::DivByZero, "division by zero"));
        }
        let reciprocal = if other.num < 0 {
            Rational {
                num: -other.den,
                den: -other.num,
            }
        } else {
            Rational {
                num: other.den,
                den: other.num,
            }
        };
        self.mul(reciprocal)
    }

    pub fn pow_int(self, exp: i32) -> Result<Rational, EvalError> {
        if exp == 0 {
            return Ok(Rational::ONE);
        }
        let base = if exp < 0 {
            if self.is_zero() {
                return Err(EvalError::new(
                    ErrorCode::DivByZero,
                    "zero cannot be raised to a negative power",
                ));
            }
            Rational::ONE.div(self)?
        } else {
            self
        };
        let mut e = exp.unsigned_abs();
        let mut base = base;
        let mut result = Rational::ONE;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(base)?;
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(base)?;
            }
        }
        Ok(result)
    }

    pub fn compare(&self, other: &Rational) -> Result<Ordering, EvalError> {
        let left = self.num.checked_mul(other.den).ok_or_else(overflow)?;
        let right = other.num.checked_mul(self.den).ok_or_else(overflow)?;
        Ok(left.cmp(&right))
    }

    pub fn floor(&self) -> i128 {
        self.num.div_euclid(self.den)
    }

    pub fn ceil(&self) -> i128 {
        -((-self.num).div_euclid(self.den))
    }

    /// Parses `12`, `-1.5`, `2.5e-3` or an exact fraction such as `1/3`.
    pub fn parse(text: &str) -> Result<Rational, EvalError> {
        let text = text.trim();
        if let Some((n, d)) = text.split_once('/') {
            return parse_decimal(n)?.div(parse_decimal(d)?);
        }
        parse_decimal(text)
    }

    /// The value scaled by `10^places` and rounded to a whole number of units.
    pub fn round_units(&self, places: u32, mode: RoundMode) -> Result<i128, EvalError> {
        let scaled = self.mul(Rational::from_int(pow10(places)?))?;
        let (n, d) = (scaled.num, scaled.den);
        // Truncate first so the quotient product never has to fall below i128::MIN.
        let q = n / d;
        let rem = (n % d).abs();
        // Compare 2 * rem with d without overflowing when the denominator is large.
        let half = rem.cmp(&(d - rem));
        let increment = match mode {
            RoundMode::Truncate => false,
            RoundMode::HalfUp => half != Ordering::Less,
            RoundMode::HalfEven => half == Ordering::Greater || (half == Ordering::Equal && q % 2 != 0),
        };
        if increment {
            q.checked_add(n.signum()).ok_or_else(overflow)
        } else {
            Ok(q)
        }
    }

    /// Decimal text with at most `places` fractional digits and no trailing zeros.
    pub fn to_decimal(&self, places: u32, mode: RoundMode) -> Result<String, EvalError> {
        if self.is_integer() {
            return Ok(self.num.to_string());
        }
        let units = self.round_units(places, mode)?;
        let scale = pow10(places)?;
        let magnitude = units.abs();
        let int_part = magnitude / scale;
        let mut frac = format!("{:0width$}", magnitude % scale, width = places as usize);
        while frac.ends_with('0') {
            frac.pop();
        }
        let sign = if units < 0 { "-" } else { "" };
        if frac.is_empty() {
            Ok(format!("{sign}{int_part}"))
        } else {
            Ok(format!("{sign}{int_part}.{frac}"))
        }
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

fn parse_decimal(text: &str) -> Result<Rational, EvalError> {
    let bad = || EvalError::new(ErrorCode::Syntax, format!("invalid number '{text}'"));
    let text = text.trim();
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (mantissa, exp) = match rest.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().map_err(|_| bad())?),
        None => (rest, 0),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{int_part}{frac_part}");
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let value: i128 = digits.parse().map_err(|_| overflow())?;
    let frac_len = i32::try_from(frac_part.len()).map_err(|_| overflow())?;
    let exp10 = exp.checked_sub(frac_len).ok_or_else(overflow)?;
    let value = if negative { -value } else { value };
    if exp10 >= 0 {
        let scale = pow10(u32::try_from(exp10).map_err(|_| overflow())?)?;
        Rational::new(value.checked_mul(scale).ok_or_else(overflow)?, 1)
    } else {
        Rational::new(value, pow10(exp10.unsigned_abs())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Rational {
        Rational::new(n, d).unwrap()
    }

    fn p(s: &str) -> Rational {
        Rational::parse(s).unwrap()
    }

    #[test]
    fn construction_reduces_and_normalizes_sign() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(1, -2), r(-1, 2));
        assert_eq!(r(0, 5), Rational::ZERO);
        assert_eq!(r(6, 3).to_string(), "2");
        assert_eq!(r(-1, 3).to_string(), "-1/3");
        assert!(Rational::new(1, 0).is_err());
    }

    #[test]
    fn basic_arithmetic() {
        assert_eq!(r(1, 2).add(r(1, 3)).unwrap(), r(5, 6));
        assert_eq!(r(1, 2).sub(r(1, 3)).unwrap(), r(1, 6));
        assert_eq!(r(2, 3).mul(r(3, 4)).unwrap(), r(1, 2));
        assert_eq!(r(1, 2).div(r(1, 4)).unwrap(), r(2, 1));
        assert_eq!(r(1, 2).div(r(-1, 4)).unwrap(), r(-2, 1));
        assert!(r(1, 2).div(Rational::ZERO).is_err());
    }

    #[test]
    fn arithmetic_detects_overflow() {
        let big = Rational::from_int(i128::MAX / 2 + 1);
        assert_eq!(big.add(big).unwrap_err().code, ErrorCode::Overflow);
        assert_eq!(
            big.mul(Rational::from_int(2)).unwrap_err().code,
            ErrorCode::Overflow
        );
    }

    #[test]
    fn integer_powers() {
        assert_eq!(r(2, 3).pow_int(3).unwrap(), r(8, 27));
        assert_eq!(r(2, 3).pow_int(-2).unwrap(), r(9, 4));
        assert_eq!(r(5, 1).pow_int(0).unwrap(), Rational::ONE);
        assert_eq!(Rational::ZERO.pow_int(0).unwrap(), Rational::ONE);
        assert_eq!(Rational::ZERO.pow_int(-1).unwrap_err().code, ErrorCode::DivByZero);
        assert_eq!(r(10, 1).pow_int(40).unwrap_err().code, ErrorCode::Overflow);
        assert_eq!(r(1, 1).pow_int(i32::MAX).unwrap(), Rational::ONE);
    }

    #[test]
    fn floor_and_ceil() {
        assert_eq!(r(7, 2).floor(), 3);
        assert_eq!(r(-7, 2).floor(), -4);
        assert_eq!(r(7, 2).ceil(), 4);
        assert_eq!(r(-7, 2).ceil(), -3);
        assert_eq!(r(4, 1).floor(), 4);
        assert_eq!(r(4, 1).ceil(), 4);
    }

    #[test]
    fn parsing() {
        assert_eq!(p("12"), r(12, 1));
        assert_eq!(p("-1.5"), r(-3, 2));
        assert_eq!(p("+0.25"), r(1, 4));
        assert_eq!(p(".5"), r(1, 2));
        assert_eq!(p("1e3"), r(1000, 1));
        assert_eq!(p("2.5e-3"), r(1, 400));
        assert_eq!(p("1/3"), r(1, 3));
        assert_eq!(p("-6/4"), r(-3, 2));
        assert!(Rational::parse("abc").is_err());
        assert!(Rational::parse("").is_err());
        assert!(Rational::parse("1/0").is_err());
        assert!(Rational::parse("1e99").is_err());
    }

    #[test]
    fn compare_orders_values() {
        assert_eq!(r(1, 3).compare(&r(1, 2)).unwrap(), Ordering::Less);
        assert_eq!(r(-1, 3).compare(&r(-1, 2)).unwrap(), Ordering::Greater);
        assert_eq!(r(2, 4).compare(&r(1, 2)).unwrap(), Ordering::Equal);
    }

    #[test]
    fn decimal_formatting() {
        let third = r(1, 3);
        assert_eq!(third.to_decimal(5, RoundMode::HalfUp).unwrap(), "0.33333");
        assert_eq!(r(2, 3).to_decimal(5, RoundMode::HalfUp).unwrap(), "0.66667");
        assert_eq!(r(5, 2).to_decimal(10, RoundMode::HalfUp).unwrap(), "2.5");
        assert_eq!(r(4, 1).to_decimal(10, RoundMode::HalfUp).unwrap(), "4");
        assert_eq!(r(-1, 8).to_decimal(2, RoundMode::HalfUp).unwrap(), "-0.13");
        assert_eq!(r(0, 1).to_decimal(2, RoundMode::HalfUp).unwrap(), "0");
    }

    #[test]
    fn rounding_modes_on_exact_halves() {
        let cases = [(5, 2), (7, 2), (-5, 2), (-7, 2), (1, 2), (-1, 2)];
        let up: Vec<i128> = cases
            .iter()
            .map(|&(n, d)| r(n, d).round_units(0, RoundMode::HalfUp).unwrap())
            .collect();
        let even: Vec<i128> = cases
            .iter()
            .map(|&(n, d)| r(n, d).round_units(0, RoundMode::HalfEven).unwrap())
            .collect();
        let trunc: Vec<i128> = cases
            .iter()
            .map(|&(n, d)| r(n, d).round_units(0, RoundMode::Truncate).unwrap())
            .collect();
        assert_eq!(up, vec![3, 4, -3, -4, 1, -1]);
        assert_eq!(even, vec![2, 4, -2, -4, 0, 0]);
        assert_eq!(trunc, vec![2, 3, -2, -3, 0, 0]);
    }

    #[test]
    fn small_negative_values_do_not_print_a_sign() {
        assert_eq!(r(-1, 10000).to_decimal(2, RoundMode::HalfUp).unwrap(), "0");
        assert_eq!(r(-1, 3).to_decimal(2, RoundMode::Truncate).unwrap(), "-0.33");
    }

    #[test]
    fn integers_print_as_is() {
        assert_eq!(r(7, 1).to_decimal(0, RoundMode::HalfUp).unwrap(), "7");
        assert_eq!(r(-12, 1).to_decimal(3, RoundMode::Truncate).unwrap(), "-12");
    }

    #[test]
    fn extreme_decimal_exponents_return_overflow() {
        for input in ["0.1e-2147483648", "0.01e-2147483647"] {
            assert_eq!(Rational::parse(input).unwrap_err().code, ErrorCode::Overflow);
        }
    }

    #[test]
    fn rounding_large_negative_values_does_not_overflow_intermediates() {
        let value = r(-i128::MAX, 3);
        assert_eq!(
            value.round_units(0, RoundMode::HalfUp).unwrap(),
            -56_713_727_820_156_410_577_229_101_238_628_035_242
        );
    }

    #[test]
    fn rounding_large_remainders_does_not_require_doubling_them() {
        let value = r(i128::MAX / 2 + 1, i128::MAX);
        assert_eq!(value.round_units(0, RoundMode::HalfUp).unwrap(), 1);
        assert_eq!(value.round_units(0, RoundMode::HalfEven).unwrap(), 1);
        assert_eq!(value.round_units(0, RoundMode::Truncate).unwrap(), 0);
        assert_eq!(
            value.neg().unwrap().round_units(0, RoundMode::HalfUp).unwrap(),
            -1
        );
    }

    #[test]
    fn round_mode_names() {
        assert_eq!("half-even".parse::<RoundMode>().unwrap(), RoundMode::HalfEven);
        assert!("sideways".parse::<RoundMode>().is_err());
    }
}
