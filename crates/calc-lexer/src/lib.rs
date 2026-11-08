//! Tokenizer for calculator expressions.
//!
//! Numbers may contain `_` separators between digits (`1_000`), a fraction (`.5`, `2.5`) and an
//! exponent (`1e3`, `2.5E-2`). Positions in tokens and errors are byte offsets into the input.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Normalized number text: no underscores, never starts with `.`.
    Number(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Bang,
    LParen,
    RParen,
    Comma,
    Assign,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    UnexpectedChar { ch: char, pos: usize },
    InvalidNumber { text: String, pos: usize },
}

impl LexError {
    pub fn position(&self) -> usize {
        match self {
            LexError::UnexpectedChar { pos, .. } | LexError::InvalidNumber { pos, .. } => *pos,
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedChar { ch, pos } => write!(f, "unexpected character '{ch}' at {pos}"),
            LexError::InvalidNumber { text, pos } => write!(f, "invalid number '{text}' at {pos}"),
        }
    }
}

impl std::error::Error for LexError {}

pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let simple = match b {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
                continue;
            }
            b'+' => Some(TokenKind::Plus),
            b'-' => Some(TokenKind::Minus),
            b'*' => Some(TokenKind::Star),
            b'/' => Some(TokenKind::Slash),
            b'%' => Some(TokenKind::Percent),
            b'^' => Some(TokenKind::Caret),
            b'!' => Some(TokenKind::Bang),
            b'(' => Some(TokenKind::LParen),
            b')' => Some(TokenKind::RParen),
            b',' => Some(TokenKind::Comma),
            b'=' => Some(TokenKind::Assign),
            _ => None,
        };
        if let Some(kind) = simple {
            tokens.push(Token {
                kind,
                span: Span { start: i, end: i + 1 },
            });
            i += 1;
        } else if b.is_ascii_digit() || b == b'.' {
            let (text, end) = lex_number(input, i)?;
            tokens.push(Token {
                kind: TokenKind::Number(text),
                span: Span { start: i, end },
            });
            i = end;
        } else if b.is_ascii_alphabetic() || b == b'_' {
            let mut end = i;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Ident(input[i..end].to_string()),
                span: Span { start: i, end },
            });
            i = end;
        } else {
            let ch = input[i..].chars().next().expect("index is on a char boundary");
            return Err(LexError::UnexpectedChar { ch, pos: i });
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: input.len(),
            end: input.len(),
        },
    });
    Ok(tokens)
}

fn lex_number(input: &str, start: usize) -> Result<(String, usize), LexError> {
    let bytes = input.as_bytes();
    let n = bytes.len();
    let invalid = |end: usize| LexError::InvalidNumber {
        text: input[start..end.min(n)].to_string(),
        pos: start,
    };
    let mut j = start;
    let mut text = String::new();

    let mut int_digits = 0;
    while j < n && (bytes[j].is_ascii_digit() || bytes[j] == b'_') {
        if bytes[j] == b'_' {
            let between_digits =
                j > start && bytes[j - 1].is_ascii_digit() && j + 1 < n && bytes[j + 1].is_ascii_digit();
            if !between_digits {
                return Err(invalid(j + 1));
            }
        } else {
            text.push(bytes[j] as char);
            int_digits += 1;
        }
        j += 1;
    }

    let mut frac_digits = 0;
    if j < n && bytes[j] == b'.' {
        j += 1;
        let mut frac = String::new();
        while j < n && bytes[j].is_ascii_digit() {
            frac.push(bytes[j] as char);
            frac_digits += 1;
            j += 1;
        }
        if int_digits == 0 && frac_digits == 0 {
            return Err(invalid(j));
        }
        if int_digits == 0 {
            text.push('0');
        }
        if frac_digits > 0 {
            text.push('.');
            text.push_str(&frac);
        }
    }

    if j < n && (bytes[j] == b'e' || bytes[j] == b'E') {
        let mut k = j + 1;
        if k < n && (bytes[k] == b'+' || bytes[k] == b'-') {
            k += 1;
        }
        if k < n && bytes[k].is_ascii_digit() {
            text.push('e');
            text.push_str(&input[j + 1..k]);
            while k < n && bytes[k].is_ascii_digit() {
                text.push(bytes[k] as char);
                k += 1;
            }
            j = k;
        }
    }
    Ok((text, j))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Vec<TokenKind> {
        tokenize(input).unwrap().into_iter().map(|t| t.kind).collect()
    }

    fn num(s: &str) -> TokenKind {
        TokenKind::Number(s.to_string())
    }

    #[test]
    fn empty_input_is_just_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
        assert_eq!(kinds("   \t"), vec![TokenKind::Eof]);
    }

    #[test]
    fn operators_and_punctuation() {
        assert_eq!(
            kinds("+-*/%^!(),="),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Caret,
                TokenKind::Bang,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Comma,
                TokenKind::Assign,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn integers_and_decimals() {
        assert_eq!(kinds("42"), vec![num("42"), TokenKind::Eof]);
        assert_eq!(kinds("3.25"), vec![num("3.25"), TokenKind::Eof]);
        assert_eq!(kinds(".5"), vec![num("0.5"), TokenKind::Eof]);
        assert_eq!(kinds("5."), vec![num("5"), TokenKind::Eof]);
    }

    #[test]
    fn underscore_separators() {
        assert_eq!(kinds("1_000_000"), vec![num("1000000"), TokenKind::Eof]);
        assert!(matches!(tokenize("1__0"), Err(LexError::InvalidNumber { .. })));
        assert!(matches!(tokenize("10_"), Err(LexError::InvalidNumber { .. })));
    }

    #[test]
    fn exponents() {
        assert_eq!(kinds("1e3"), vec![num("1e3"), TokenKind::Eof]);
        assert_eq!(kinds("2.5E-2"), vec![num("2.5e-2"), TokenKind::Eof]);
        assert_eq!(kinds("1e+4"), vec![num("1e+4"), TokenKind::Eof]);
    }

    #[test]
    fn exponent_marker_without_digits_is_an_identifier() {
        assert_eq!(
            kinds("2e"),
            vec![num("2"), TokenKind::Ident("e".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("2e+"),
            vec![
                num("2"),
                TokenKind::Ident("e".into()),
                TokenKind::Plus,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lone_dot_is_invalid() {
        assert!(matches!(tokenize("."), Err(LexError::InvalidNumber { .. })));
        assert!(matches!(
            tokenize("1 + ."),
            Err(LexError::InvalidNumber { pos: 4, .. })
        ));
    }

    #[test]
    fn identifiers() {
        assert_eq!(
            kinds("sqrt(x_1)"),
            vec![
                TokenKind::Ident("sqrt".into()),
                TokenKind::LParen,
                TokenKind::Ident("x_1".into()),
                TokenKind::RParen,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn spans_are_byte_offsets() {
        let tokens = tokenize("12 + ab").unwrap();
        assert_eq!(tokens[0].span, Span { start: 0, end: 2 });
        assert_eq!(tokens[1].span, Span { start: 3, end: 4 });
        assert_eq!(tokens[2].span, Span { start: 5, end: 7 });
        assert_eq!(tokens[3].span, Span { start: 7, end: 7 });
    }

    #[test]
    fn unexpected_characters_report_position() {
        assert_eq!(
            tokenize("1 $ 2"),
            Err(LexError::UnexpectedChar { ch: '$', pos: 2 })
        );
        assert_eq!(tokenize("é"), Err(LexError::UnexpectedChar { ch: 'é', pos: 0 }));
        assert_eq!(
            tokenize("1 + ٣"),
            Err(LexError::UnexpectedChar { ch: '٣', pos: 4 })
        );
    }
}
