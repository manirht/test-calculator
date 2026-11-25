//! Exact calculator engine: parse, evaluate over rationals, format.
//!
//! Statistics and finance functions live in the Python `calcstats` package and are reached
//! through a [`Bridge`].

pub mod bridge;
pub mod builtins;
pub mod env;
pub mod error;
pub mod eval;
pub mod format;
pub mod number;

pub use bridge::{Bridge, NoBridge, PythonBridge};
pub use error::{ErrorCode, EvalError};
pub use format::FormatOptions;
pub use number::{Rational, RoundMode};

use env::Env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub value: Rational,
    /// Decimal text rounded per the active [`FormatOptions`].
    pub text: String,
    /// Exact value as `n` or `n/d`.
    pub exact: String,
}

pub struct Calculator {
    env: Env,
    bridge: Box<dyn Bridge>,
    pub options: FormatOptions,
}

impl Calculator {
    pub fn new(bridge: Box<dyn Bridge>) -> Self {
        Calculator {
            env: Env::new(),
            bridge,
            options: FormatOptions::default(),
        }
    }

    pub fn eval_line(&mut self, input: &str) -> Result<Evaluation, EvalError> {
        self.eval_line_with(input, self.options)
    }

    pub fn eval_line_with(&mut self, input: &str, options: FormatOptions) -> Result<Evaluation, EvalError> {
        let expr = calc_parser::parse(input)?;
        let mut next_env = self.env.clone();
        let value = eval::eval(&expr, &mut next_env, self.bridge.as_mut())?;
        let text = format::format_value(&value, &options)?;
        next_env.set_ans(value);
        self.env = next_env;
        Ok(Evaluation {
            value,
            text,
            exact: value.to_string(),
        })
    }

    pub fn reset(&mut self) {
        self.env.clear();
    }

    pub fn variable_names(&self) -> Vec<String> {
        self.env.names()
    }

    /// Names of every callable function, for completion and help.
    pub fn function_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = builtins::names().to_vec();
        names.extend_from_slice(bridge::EXTERNAL_FUNCS);
        names.sort_unstable();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc() -> Calculator {
        Calculator::new(Box::new(NoBridge))
    }

    #[test]
    fn evaluates_and_formats() {
        let mut c = calc();
        let r = c.eval_line("1/3").unwrap();
        assert_eq!(r.text, "0.3333333333");
        assert_eq!(r.exact, "1/3");
        assert_eq!(c.eval_line("2+2").unwrap().text, "4");
    }

    #[test]
    fn flat_sums_up_to_the_tree_limit_are_supported() {
        let mut c = calc();
        for (terms, expected) in [(300, "300"), (512, "512")] {
            let input = std::iter::repeat_n("1", terms).collect::<Vec<_>>().join("+");
            assert_eq!(c.eval_line(&input).unwrap().exact, expected);
        }
    }

    #[test]
    fn ans_tracks_the_last_result() {
        let mut c = calc();
        assert_eq!(c.eval_line("ans").unwrap_err().code, ErrorCode::UnknownVar);
        c.eval_line("6*7").unwrap();
        assert_eq!(c.eval_line("ans + 1").unwrap().text, "43");
        assert_eq!(c.eval_line("ans").unwrap().text, "43");
    }

    #[test]
    fn failed_lines_keep_the_previous_ans() {
        let mut c = calc();
        c.eval_line("5").unwrap();
        assert!(c.eval_line("1/0").is_err());
        assert_eq!(c.eval_line("ans").unwrap().text, "5");
    }

    #[test]
    fn formatting_failure_keeps_existing_variable_value() {
        let mut c = calc();
        c.eval_line("x = 5").unwrap();
        let thirty = FormatOptions::new(30, RoundMode::HalfUp).unwrap();
        assert_eq!(
            c.eval_line_with("x = 123456789012/7", thirty).unwrap_err().code,
            ErrorCode::Overflow
        );
        assert_eq!(c.eval_line("x").unwrap().exact, "5");
    }

    #[test]
    fn formatting_failure_does_not_create_a_variable() {
        let mut c = calc();
        let thirty = FormatOptions::new(30, RoundMode::HalfUp).unwrap();
        assert_eq!(
            c.eval_line_with("x = 123456789012/7", thirty).unwrap_err().code,
            ErrorCode::Overflow
        );
        assert_eq!(c.eval_line("x").unwrap_err().code, ErrorCode::UnknownVar);
    }

    #[test]
    fn variables_persist_until_reset() {
        let mut c = calc();
        c.eval_line("rate = 5/100").unwrap();
        assert_eq!(c.eval_line("200 * rate").unwrap().text, "10");
        assert_eq!(c.variable_names(), vec!["rate"]);
        c.reset();
        assert!(c.eval_line("rate").is_err());
    }

    #[test]
    fn syntax_errors_are_reported_with_positions() {
        let mut c = calc();
        let err = c.eval_line("1 + * 2").unwrap_err();
        assert_eq!(err.code, ErrorCode::Syntax);
        assert_eq!(err.pos, Some(4));
        assert_eq!(c.eval_line("").unwrap_err().code, ErrorCode::Syntax);
    }

    #[test]
    fn per_call_options_override_defaults() {
        let mut c = calc();
        let two = FormatOptions::new(2, RoundMode::HalfUp).unwrap();
        assert_eq!(c.eval_line_with("2/3", two).unwrap().text, "0.67");
        assert_eq!(c.eval_line("2/3").unwrap().text, "0.6666666667");
    }

    #[test]
    fn function_names_include_both_sides_of_the_bridge() {
        let names = calc().function_names();
        assert!(names.contains(&"sqrt"));
        assert!(names.contains(&"stddev"));
        assert!(names.windows(2).all(|w| w[0] < w[1]));
    }
}
