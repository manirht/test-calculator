//! Functions implemented directly in Rust.

use crate::error::{ErrorCode, EvalError};
use crate::number::{Rational, RoundMode};
use std::cmp::Ordering;

const NAMES: &[&str] = &[
    "abs", "sqrt", "min", "max", "round", "floor", "ceil", "gcd", "lcm", "sign",
];

pub fn is_builtin(name: &str) -> bool {
    NAMES.contains(&name)
}

pub fn names() -> &'static [&'static str] {
    NAMES
}

fn arity(name: &str, args: &[Rational], min: usize, max: usize) -> Result<(), EvalError> {
    if args.len() < min || args.len() > max {
        let expected = if min == max {
            format!("{min}")
        } else if max == usize::MAX {
            format!("at least {min}")
        } else {
            format!("{min} to {max}")
        };
        return Err(EvalError::new(
            ErrorCode::BadArity,
            format!("{name} expects {expected} argument(s), got {}", args.len()),
        ));
    }
    Ok(())
}

fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

fn integer_arg(name: &str, value: &Rational) -> Result<i128, EvalError> {
    if value.is_integer() {
        Ok(value.num())
    } else {
        Err(EvalError::new(
            ErrorCode::Domain,
            format!("{name} requires integer arguments"),
        ))
    }
}

pub fn sqrt(x: Rational) -> Result<Rational, EvalError> {
    if x.is_negative() {
        return Err(EvalError::new(
            ErrorCode::Domain,
            "square root of a negative number",
        ));
    }
    if x.is_zero() {
        return Ok(Rational::ZERO);
    }
    let (n, d) = (x.num() as u128, x.den() as u128);
    let (rn, rd) = (isqrt(n), isqrt(d));
    if rn * rn == n && rd * rd == d {
        return Rational::new(rn as i128, rd as i128);
    }
    const SCALE: u128 = 1_000_000_000_000;
    let scaled = n
        .checked_mul(SCALE * SCALE)
        .ok_or_else(|| EvalError::new(ErrorCode::Overflow, "arithmetic overflow"))?
        / d;
    Rational::new(isqrt(scaled) as i128, SCALE as i128)
}

fn gcd_i(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

pub fn call(name: &str, args: &[Rational]) -> Result<Rational, EvalError> {
    match name {
        "abs" => {
            arity(name, args, 1, 1)?;
            args[0].abs()
        }
        "sqrt" => {
            arity(name, args, 1, 1)?;
            sqrt(args[0])
        }
        "sign" => {
            arity(name, args, 1, 1)?;
            Ok(Rational::from_int(args[0].signum()))
        }
        "floor" => {
            arity(name, args, 1, 1)?;
            Ok(Rational::from_int(args[0].floor()))
        }
        "ceil" => {
            arity(name, args, 1, 1)?;
            Ok(Rational::from_int(args[0].ceil()))
        }
        "round" => {
            arity(name, args, 1, 2)?;
            let places = match args.get(1) {
                Some(p) => {
                    let p = integer_arg(name, p)?;
                    if !(0..=20).contains(&p) {
                        return Err(EvalError::new(
                            ErrorCode::Domain,
                            "round places must be between 0 and 20",
                        ));
                    }
                    p as u32
                }
                None => 0,
            };
            let units = args[0].round_units(places, RoundMode::HalfUp)?;
            Rational::new(units, 10i128.pow(places))
        }
        "min" | "max" => {
            arity(name, args, 1, usize::MAX)?;
            let mut best = args[0];
            for value in &args[1..] {
                let ordering = value.compare(&best)?;
                let better = if name == "min" {
                    ordering == Ordering::Less
                } else {
                    ordering == Ordering::Greater
                };
                if better {
                    best = *value;
                }
            }
            Ok(best)
        }
        "gcd" => {
            arity(name, args, 2, 2)?;
            let (a, b) = (integer_arg(name, &args[0])?, integer_arg(name, &args[1])?);
            Ok(Rational::from_int(gcd_i(a, b)))
        }
        "lcm" => {
            arity(name, args, 2, 2)?;
            let (a, b) = (integer_arg(name, &args[0])?, integer_arg(name, &args[1])?);
            if a == 0 || b == 0 {
                return Ok(Rational::ZERO);
            }
            let g = gcd_i(a, b);
            (a / g)
                .checked_mul(b)
                .map(|v| Rational::from_int(v.abs()))
                .ok_or_else(|| EvalError::new(ErrorCode::Overflow, "arithmetic overflow"))
        }
        _ => Err(EvalError::new(
            ErrorCode::UnknownFunc,
            format!("unknown function '{name}'"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: i128) -> Rational {
        Rational::from_int(n)
    }

    fn frac(n: i128, d: i128) -> Rational {
        Rational::new(n, d).unwrap()
    }

    #[test]
    fn abs_sign_floor_ceil() {
        assert_eq!(call("abs", &[int(-3)]).unwrap(), int(3));
        assert_eq!(call("sign", &[frac(-1, 2)]).unwrap(), int(-1));
        assert_eq!(call("sign", &[int(0)]).unwrap(), int(0));
        assert_eq!(call("floor", &[frac(-7, 2)]).unwrap(), int(-4));
        assert_eq!(call("ceil", &[frac(-7, 2)]).unwrap(), int(-3));
    }

    #[test]
    fn sqrt_exact_and_approximate() {
        assert_eq!(sqrt(int(144)).unwrap(), int(12));
        assert_eq!(sqrt(frac(9, 4)).unwrap(), frac(3, 2));
        let root2 = sqrt(int(2)).unwrap();
        assert_eq!(root2.to_decimal(10, RoundMode::HalfUp).unwrap(), "1.4142135624");
        assert_eq!(sqrt(int(-1)).unwrap_err().code, ErrorCode::Domain);
        assert_eq!(sqrt(int(0)).unwrap(), int(0));
    }

    #[test]
    fn round_with_places() {
        assert_eq!(call("round", &[frac(5, 2)]).unwrap(), int(3));
        assert_eq!(call("round", &[frac(-5, 2)]).unwrap(), int(-3));
        assert_eq!(
            call("round", &[frac(31415, 10000), int(2)]).unwrap(),
            frac(314, 100)
        );
        assert_eq!(
            call("round", &[int(1), int(21)]).unwrap_err().code,
            ErrorCode::Domain
        );
        assert_eq!(
            call("round", &[int(1), frac(1, 2)]).unwrap_err().code,
            ErrorCode::Domain
        );
    }

    #[test]
    fn min_and_max() {
        assert_eq!(call("min", &[int(3), int(-1), int(2)]).unwrap(), int(-1));
        assert_eq!(call("max", &[int(3), int(-1), int(2)]).unwrap(), int(3));
        assert_eq!(call("max", &[frac(1, 3)]).unwrap(), frac(1, 3));
        assert_eq!(call("max", &[]).unwrap_err().code, ErrorCode::BadArity);
    }

    #[test]
    fn gcd_and_lcm() {
        assert_eq!(call("gcd", &[int(12), int(18)]).unwrap(), int(6));
        assert_eq!(call("gcd", &[int(-12), int(18)]).unwrap(), int(6));
        assert_eq!(call("lcm", &[int(4), int(6)]).unwrap(), int(12));
        assert_eq!(call("lcm", &[int(0), int(6)]).unwrap(), int(0));
        assert_eq!(
            call("gcd", &[frac(1, 2), int(2)]).unwrap_err().code,
            ErrorCode::Domain
        );
    }

    #[test]
    fn arity_is_checked() {
        let err = call("abs", &[int(1), int(2)]).unwrap_err();
        assert_eq!(err.code, ErrorCode::BadArity);
        assert!(err.message.contains("abs expects 1"));
        assert_eq!(call("round", &[]).unwrap_err().code, ErrorCode::BadArity);
    }

    #[test]
    fn registry_matches_dispatch() {
        for name in names() {
            assert!(is_builtin(name));
        }
        assert!(!is_builtin("mean"));
    }
}
