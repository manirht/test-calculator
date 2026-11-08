//! Evaluation errors and their wire-level codes.

use calc_parser::ParseError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    DivByZero,
    Overflow,
    UnknownVar,
    UnknownFunc,
    BadArity,
    Domain,
    Syntax,
    External,
    TooDeep,
    Reserved,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::DivByZero => "DIV_BY_ZERO",
            ErrorCode::Overflow => "OVERFLOW",
            ErrorCode::UnknownVar => "UNKNOWN_VAR",
            ErrorCode::UnknownFunc => "UNKNOWN_FUNC",
            ErrorCode::BadArity => "BAD_ARITY",
            ErrorCode::Domain => "DOMAIN",
            ErrorCode::Syntax => "SYNTAX",
            ErrorCode::External => "EXTERNAL",
            ErrorCode::TooDeep => "TOO_DEEP",
            ErrorCode::Reserved => "RESERVED",
        }
    }

    pub fn from_wire(code: &str) -> ErrorCode {
        match code {
            "DIV_BY_ZERO" => ErrorCode::DivByZero,
            "OVERFLOW" => ErrorCode::Overflow,
            "UNKNOWN_VAR" => ErrorCode::UnknownVar,
            "UNKNOWN_FUNC" => ErrorCode::UnknownFunc,
            "BAD_ARITY" => ErrorCode::BadArity,
            "DOMAIN" => ErrorCode::Domain,
            "SYNTAX" => ErrorCode::Syntax,
            "TOO_DEEP" => ErrorCode::TooDeep,
            "RESERVED" => ErrorCode::Reserved,
            _ => ErrorCode::External,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalError {
    pub code: ErrorCode,
    pub message: String,
    pub pos: Option<usize>,
}

impl EvalError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        EvalError {
            code,
            message: message.into(),
            pos: None,
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for EvalError {}

impl From<ParseError> for EvalError {
    fn from(e: ParseError) -> Self {
        let code = if e == ParseError::TooDeep {
            ErrorCode::TooDeep
        } else {
            ErrorCode::Syntax
        };
        EvalError {
            code,
            message: e.to_string(),
            pos: e.position(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_codes_round_trip() {
        for code in [
            ErrorCode::DivByZero,
            ErrorCode::Overflow,
            ErrorCode::UnknownVar,
            ErrorCode::UnknownFunc,
            ErrorCode::BadArity,
            ErrorCode::Domain,
            ErrorCode::Syntax,
            ErrorCode::TooDeep,
            ErrorCode::Reserved,
        ] {
            assert_eq!(ErrorCode::from_wire(code.as_str()), code);
        }
        assert_eq!(ErrorCode::from_wire("SOMETHING_ELSE"), ErrorCode::External);
    }

    #[test]
    fn parse_errors_keep_their_position() {
        let err: EvalError = calc_parser::parse("1 +").unwrap_err().into();
        assert_eq!(err.code, ErrorCode::Syntax);
        assert_eq!(err.pos, Some(3));
    }

    #[test]
    fn display_includes_code() {
        let err = EvalError::new(ErrorCode::DivByZero, "division by zero");
        assert_eq!(err.to_string(), "DIV_BY_ZERO: division by zero");
    }
}
