//! Variable storage, the `ans` register and built-in constants.

use crate::error::{ErrorCode, EvalError};
use crate::number::Rational;
use std::collections::HashMap;

pub const RESERVED: &[&str] = &["ans", "pi", "e"];

fn pi() -> Rational {
    Rational::new(314_159_265_358_979, 100_000_000_000_000).expect("constant")
}

fn euler() -> Rational {
    Rational::new(271_828_182_845_905, 100_000_000_000_000).expect("constant")
}

#[derive(Debug, Default, Clone)]
pub struct Env {
    vars: HashMap<String, Rational>,
    ans: Option<Rational>,
}

impl Env {
    pub fn new() -> Self {
        Env::default()
    }

    pub fn get(&self, name: &str) -> Result<Rational, EvalError> {
        match name {
            "ans" => self
                .ans
                .ok_or_else(|| EvalError::new(ErrorCode::UnknownVar, "no previous result")),
            "pi" => Ok(pi()),
            "e" => Ok(euler()),
            _ => {
                self.vars.get(name).copied().ok_or_else(|| {
                    EvalError::new(ErrorCode::UnknownVar, format!("unknown variable '{name}'"))
                })
            }
        }
    }

    pub fn set(&mut self, name: &str, value: Rational) -> Result<(), EvalError> {
        if RESERVED.contains(&name) {
            return Err(EvalError::new(
                ErrorCode::Reserved,
                format!("'{name}' is reserved and cannot be assigned"),
            ));
        }
        self.vars.insert(name.to_string(), value);
        Ok(())
    }

    pub fn set_ans(&mut self, value: Rational) {
        self.ans = Some(value);
    }

    pub fn clear(&mut self) {
        self.vars.clear();
        self.ans = None;
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.vars.keys().cloned().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut env = Env::new();
        env.set("x", Rational::from_int(5)).unwrap();
        assert_eq!(env.get("x").unwrap(), Rational::from_int(5));
        env.set("x", Rational::from_int(6)).unwrap();
        assert_eq!(env.get("x").unwrap(), Rational::from_int(6));
    }

    #[test]
    fn unknown_variable() {
        let env = Env::new();
        assert_eq!(env.get("nope").unwrap_err().code, ErrorCode::UnknownVar);
        assert_eq!(env.get("ans").unwrap_err().code, ErrorCode::UnknownVar);
    }

    #[test]
    fn constants_and_reserved_names() {
        let mut env = Env::new();
        assert!(env.get("pi").is_ok());
        assert!(env.get("e").is_ok());
        for name in RESERVED {
            assert_eq!(
                env.set(name, Rational::ONE).unwrap_err().code,
                ErrorCode::Reserved
            );
        }
    }

    #[test]
    fn ans_and_clear() {
        let mut env = Env::new();
        env.set_ans(Rational::from_int(9));
        assert_eq!(env.get("ans").unwrap(), Rational::from_int(9));
        env.set("a", Rational::ONE).unwrap();
        env.clear();
        assert!(env.get("ans").is_err());
        assert!(env.get("a").is_err());
        assert!(env.names().is_empty());
    }

    #[test]
    fn names_are_sorted() {
        let mut env = Env::new();
        env.set("b", Rational::ONE).unwrap();
        env.set("a", Rational::ONE).unwrap();
        assert_eq!(env.names(), vec!["a", "b"]);
    }
}
