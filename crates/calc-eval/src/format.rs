//! Output formatting options.

use crate::error::{ErrorCode, EvalError};
use crate::number::{Rational, RoundMode};

pub const MAX_PLACES: u32 = 30;
pub const DEFAULT_PLACES: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub places: u32,
    pub mode: RoundMode,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            places: DEFAULT_PLACES,
            mode: RoundMode::HalfUp,
        }
    }
}

impl FormatOptions {
    pub fn new(places: u32, mode: RoundMode) -> Result<Self, EvalError> {
        if places > MAX_PLACES {
            return Err(EvalError::new(
                ErrorCode::Domain,
                format!("places must be at most {MAX_PLACES}"),
            ));
        }
        Ok(FormatOptions { places, mode })
    }
}

pub fn format_value(value: &Rational, options: &FormatOptions) -> Result<String, EvalError> {
    value.to_decimal(options.places, options.mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let o = FormatOptions::default();
        assert_eq!(o.places, 10);
        assert_eq!(o.mode, RoundMode::HalfUp);
    }

    #[test]
    fn places_are_bounded() {
        assert!(FormatOptions::new(30, RoundMode::HalfUp).is_ok());
        assert_eq!(
            FormatOptions::new(31, RoundMode::HalfUp).unwrap_err().code,
            ErrorCode::Domain
        );
    }

    #[test]
    fn formats_with_options() {
        let third = Rational::new(1, 3).unwrap();
        let two = FormatOptions::new(2, RoundMode::HalfUp).unwrap();
        assert_eq!(format_value(&third, &two).unwrap(), "0.33");
        assert_eq!(format_value(&Rational::new(2, 3).unwrap(), &two).unwrap(), "0.67");
    }
}
