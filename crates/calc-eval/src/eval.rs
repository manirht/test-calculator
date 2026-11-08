//! Tree-walking evaluator over exact rationals.

use crate::bridge::Bridge;
use crate::builtins;
use crate::env::Env;
use crate::error::{ErrorCode, EvalError};
use crate::number::Rational;
use calc_parser::{BinOp, Expr, UnOp};

pub fn eval(expr: &Expr, env: &mut Env, bridge: &mut dyn Bridge) -> Result<Rational, EvalError> {
    match expr {
        Expr::Number(text) => Rational::parse(text),
        Expr::Var(name) => env.get(name),
        Expr::Unary { op, expr } => {
            let value = eval(expr, env, bridge)?;
            match op {
                UnOp::Neg => value.neg(),
                UnOp::Pos => Ok(value),
            }
        }
        Expr::Binary { op, lhs, rhs } => {
            let left = eval(lhs, env, bridge)?;
            let right = eval(rhs, env, bridge)?;
            apply_binary(*op, left, right)
        }
        Expr::Factorial(inner) => factorial(eval(inner, env, bridge)?),
        Expr::Call { name, args } => {
            let mut values = Vec::with_capacity(args.len());
            for arg in args {
                values.push(eval(arg, env, bridge)?);
            }
            if builtins::is_builtin(name) {
                builtins::call(name, &values)
            } else if bridge.supports(name) {
                bridge.call(name, &values)
            } else {
                Err(EvalError::new(
                    ErrorCode::UnknownFunc,
                    format!("unknown function '{name}'"),
                ))
            }
        }
        Expr::Assign { name, value } => {
            let value = eval(value, env, bridge)?;
            env.set(name, value)?;
            Ok(value)
        }
    }
}

fn apply_binary(op: BinOp, left: Rational, right: Rational) -> Result<Rational, EvalError> {
    match op {
        BinOp::Add => left.add(right),
        BinOp::Sub => left.sub(right),
        BinOp::Mul => left.mul(right),
        BinOp::Div => left.div(right),
        BinOp::Mod => {
            if right.is_zero() {
                return Err(EvalError::new(ErrorCode::DivByZero, "modulo by zero"));
            }
            let quotient = left.div(right)?.floor();
            left.sub(right.mul(Rational::from_int(quotient))?)
        }
        BinOp::Pow => {
            if !right.is_integer() {
                return Err(EvalError::new(
                    ErrorCode::Domain,
                    "fractional exponents are not supported; use sqrt()",
                ));
            }
            let exp = i32::try_from(right.num())
                .map_err(|_| EvalError::new(ErrorCode::Overflow, "exponent is too large"))?;
            left.pow_int(exp)
        }
    }
}

fn factorial(value: Rational) -> Result<Rational, EvalError> {
    if !value.is_integer() || value.is_negative() {
        return Err(EvalError::new(
            ErrorCode::Domain,
            "factorial requires a non-negative integer",
        ));
    }
    let mut result = Rational::ONE;
    for i in 2..=value.num() {
        result = result.mul(Rational::from_int(i))?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::NoBridge;
    use calc_parser::parse;

    fn run(input: &str) -> Result<String, EvalError> {
        let mut env = Env::new();
        let value = eval(&parse(input).unwrap(), &mut env, &mut NoBridge)?;
        Ok(value.to_string())
    }

    fn ok(input: &str) -> String {
        run(input).unwrap_or_else(|e| panic!("{input}: {e}"))
    }

    fn code(input: &str) -> ErrorCode {
        run(input).unwrap_err().code
    }

    #[test]
    fn arithmetic() {
        assert_eq!(ok("1+2*3"), "7");
        assert_eq!(ok("(1+2)*3"), "9");
        assert_eq!(ok("10-4-3"), "3");
        assert_eq!(ok("100/10/5"), "2");
        assert_eq!(ok("1/3*3"), "1");
        assert_eq!(ok("0.1+0.2"), "3/10");
    }

    #[test]
    fn unary_operators() {
        assert_eq!(ok("-5"), "-5");
        assert_eq!(ok("--5"), "5");
        assert_eq!(ok("+5"), "5");
        assert_eq!(ok("-2^2"), "-4");
        assert_eq!(ok("(-2)^2"), "4");
        assert_eq!(ok("2*-3"), "-6");
    }

    #[test]
    fn powers() {
        assert_eq!(ok("2^10"), "1024");
        assert_eq!(ok("2^3^2"), "512");
        assert_eq!(ok("2^-2"), "1/4");
        assert_eq!(ok("(1/2)^3"), "1/8");
        assert_eq!(ok("0^0"), "1");
        assert_eq!(code("0^-1"), ErrorCode::DivByZero);
        assert_eq!(code("2^0.5"), ErrorCode::Domain);
        assert_eq!(code("10^50"), ErrorCode::Overflow);
        assert_eq!(code("2^99999999999"), ErrorCode::Overflow);
    }

    #[test]
    fn modulo_follows_the_divisor_sign() {
        assert_eq!(ok("7 % 3"), "1");
        assert_eq!(ok("-7 % 3"), "2");
        assert_eq!(ok("7 % -3"), "-2");
        assert_eq!(ok("-7 % -3"), "-1");
        assert_eq!(ok("5.5 % 2"), "3/2");
        assert_eq!(code("5 % 0"), ErrorCode::DivByZero);
    }

    #[test]
    fn division_by_zero() {
        assert_eq!(code("1/0"), ErrorCode::DivByZero);
        assert_eq!(code("1/(2-2)"), ErrorCode::DivByZero);
        assert_eq!(ok("0/5"), "0");
    }

    #[test]
    fn factorials() {
        assert_eq!(ok("0!"), "1");
        assert_eq!(ok("5!"), "120");
        assert_eq!(ok("3!!"), "720");
        assert_eq!(ok("-3!"), "-6");
        assert_eq!(ok("(2+3)!"), "120");
        assert_eq!(code("(-1)!"), ErrorCode::Domain);
        assert_eq!(code("2.5!"), ErrorCode::Domain);
        assert_eq!(ok("33!"), "8683317618811886495518194401280000000");
        assert_eq!(code("34!"), ErrorCode::Overflow);
    }

    #[test]
    fn variables_and_assignment() {
        let mut env = Env::new();
        let value = eval(&parse("x = 4").unwrap(), &mut env, &mut NoBridge).unwrap();
        assert_eq!(value.to_string(), "4");
        let doubled = eval(&parse("x * 2").unwrap(), &mut env, &mut NoBridge).unwrap();
        assert_eq!(doubled.to_string(), "8");
        assert_eq!(
            eval(&parse("y").unwrap(), &mut env, &mut NoBridge)
                .unwrap_err()
                .code,
            ErrorCode::UnknownVar
        );
        assert_eq!(
            eval(&parse("pi = 3").unwrap(), &mut env, &mut NoBridge)
                .unwrap_err()
                .code,
            ErrorCode::Reserved
        );
    }

    #[test]
    fn builtin_calls() {
        assert_eq!(ok("abs(-3)"), "3");
        assert_eq!(ok("max(1, 5, 3)"), "5");
        assert_eq!(ok("sqrt(16)"), "4");
        assert_eq!(ok("round(2.5)"), "3");
        assert_eq!(code("sqrt(-1)"), ErrorCode::Domain);
        assert_eq!(code("nope(1)"), ErrorCode::UnknownFunc);
        assert_eq!(code("mean(1,2)"), ErrorCode::UnknownFunc);
        assert_eq!(code("abs()"), ErrorCode::BadArity);
    }

    #[test]
    fn errors_propagate_from_arguments() {
        assert_eq!(code("abs(1/0)"), ErrorCode::DivByZero);
        assert_eq!(code("max(1, zzz)"), ErrorCode::UnknownVar);
    }

    struct Recorder(Vec<(String, Vec<String>)>);

    impl Bridge for Recorder {
        fn supports(&self, name: &str) -> bool {
            name == "mean"
        }

        fn call(&mut self, name: &str, args: &[Rational]) -> Result<Rational, EvalError> {
            self.0
                .push((name.to_string(), args.iter().map(|a| a.to_string()).collect()));
            Ok(Rational::from_int(42))
        }
    }

    #[test]
    fn external_calls_go_through_the_bridge() {
        let mut env = Env::new();
        let mut bridge = Recorder(Vec::new());
        let value = eval(&parse("mean(1, 2/4, 3)").unwrap(), &mut env, &mut bridge).unwrap();
        assert_eq!(value.to_string(), "42");
        assert_eq!(
            bridge.0,
            vec![(
                "mean".to_string(),
                vec!["1".to_string(), "1/2".to_string(), "3".to_string()]
            )]
        );
    }
}
