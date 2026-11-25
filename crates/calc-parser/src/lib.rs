//! Pratt parser for calculator expressions.
//!
//! Precedence, loosest to tightest: `+ -`, `* / %`, unary `+ -`, `^` (right associative),
//! postfix `!`. So `-2^2` is `-(2^2)`, `2^3^2` is `2^(3^2)` and `-3!` is `-(3!)`.

use calc_lexer::{tokenize, LexError, Token, TokenKind};
use std::fmt;

/// Nesting limit; deeper input is rejected instead of overflowing the stack.
pub const MAX_DEPTH: usize = 200;

/// Bound the tree depth as well as recursive syntax nesting. Flat operator chains
/// can form deep trees, whose evaluation, formatting and destruction use the stack.
const MAX_TREE_DEPTH: usize = 512;

const ADD_BP: (u8, u8) = (10, 11);
const MUL_BP: (u8, u8) = (20, 21);
const PREFIX_BP: u8 = 25;
const POW_BP: (u8, u8) = (31, 30);
const POSTFIX_BP: u8 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

impl BinOp {
    fn symbol(self) -> char {
        match self {
            BinOp::Add => '+',
            BinOp::Sub => '-',
            BinOp::Mul => '*',
            BinOp::Div => '/',
            BinOp::Mod => '%',
            BinOp::Pow => '^',
        }
    }

    fn precedence(self) -> u8 {
        match self {
            BinOp::Add | BinOp::Sub => 10,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 20,
            BinOp::Pow => 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number(String),
    Var(String),
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Factorial(Box<Expr>),
    Assign {
        name: String,
        value: Box<Expr>,
    },
}

impl Expr {
    fn precedence(&self) -> u8 {
        match self {
            Expr::Number(_) | Expr::Var(_) | Expr::Call { .. } => 100,
            Expr::Factorial(_) => POSTFIX_BP,
            Expr::Unary { .. } => PREFIX_BP,
            Expr::Binary { op, .. } => op.precedence(),
            Expr::Assign { .. } => 0,
        }
    }

    fn write(&self, f: &mut fmt::Formatter<'_>, min: u8) -> fmt::Result {
        let parens = self.precedence() < min;
        if parens {
            f.write_str("(")?;
        }
        match self {
            Expr::Number(text) => f.write_str(text)?,
            Expr::Var(name) => f.write_str(name)?,
            Expr::Unary { op, expr } => {
                f.write_str(if *op == UnOp::Neg { "-" } else { "+" })?;
                expr.write(f, PREFIX_BP)?;
            }
            Expr::Binary { op, lhs, rhs } => {
                let p = op.precedence();
                let (left_min, right_min) = if *op == BinOp::Pow { (p + 1, p) } else { (p, p + 1) };
                lhs.write(f, left_min)?;
                write!(f, "{}", op.symbol())?;
                rhs.write(f, right_min)?;
            }
            Expr::Call { name, args } => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    arg.write(f, 0)?;
                }
                f.write_str(")")?;
            }
            Expr::Factorial(inner) => {
                inner.write(f, POSTFIX_BP)?;
                f.write_str("!")?;
            }
            Expr::Assign { name, value } => {
                write!(f, "{name} = ")?;
                value.write(f, 0)?;
            }
        }
        if parens {
            f.write_str(")")?;
        }
        Ok(())
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Lex(LexError),
    UnexpectedToken { found: String, pos: usize },
    UnexpectedEof { pos: usize },
    TooDeep,
}

impl ParseError {
    pub fn position(&self) -> Option<usize> {
        match self {
            ParseError::Lex(e) => Some(e.position()),
            ParseError::UnexpectedToken { pos, .. } | ParseError::UnexpectedEof { pos } => Some(*pos),
            ParseError::TooDeep => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Lex(e) => write!(f, "{e}"),
            ParseError::UnexpectedToken { found, pos } => write!(f, "unexpected '{found}' at {pos}"),
            ParseError::UnexpectedEof { pos } => write!(f, "unexpected end of input at {pos}"),
            ParseError::TooDeep => write!(f, "expression is nested too deeply"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError::Lex(e)
    }
}

pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        depth: 0,
    };
    let expr = if matches!(parser.kind(0), TokenKind::Ident(_)) && matches!(parser.kind(1), TokenKind::Assign)
    {
        let name = match parser.advance().kind {
            TokenKind::Ident(name) => name,
            _ => unreachable!("checked above"),
        };
        parser.advance();
        let value = parser.parse_expr(0)?;
        ParsedExpr::new(
            Expr::Assign {
                name,
                value: Box::new(value.expr),
            },
            value.height + 1,
        )?
        .expr
    } else {
        parser.parse_expr(0)?.expr
    };
    parser.expect_eof()?;
    Ok(expr)
}

struct ParsedExpr {
    expr: Expr,
    height: usize,
}

impl ParsedExpr {
    fn new(expr: Expr, height: usize) -> Result<Self, ParseError> {
        if height > MAX_TREE_DEPTH {
            return Err(ParseError::TooDeep);
        }
        Ok(Self { expr, height })
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn kind(&self, offset: usize) -> &TokenKind {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn unexpected(&self, tok: &Token) -> ParseError {
        match &tok.kind {
            TokenKind::Eof => ParseError::UnexpectedEof { pos: tok.span.start },
            other => ParseError::UnexpectedToken {
                found: describe(other),
                pos: tok.span.start,
            },
        }
    }

    fn expect_eof(&mut self) -> Result<(), ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Eof => Ok(()),
            _ => Err(self.unexpected(&tok)),
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        let tok = self.advance();
        if tok.kind == kind {
            Ok(())
        } else {
            Err(self.unexpected(&tok))
        }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<ParsedExpr, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ParseError::TooDeep);
        }
        let result = self.parse_expr_inner(min_bp);
        self.depth -= 1;
        result
    }

    fn parse_expr_inner(&mut self, min_bp: u8) -> Result<ParsedExpr, ParseError> {
        let tok = self.advance();
        let mut lhs = match tok.kind.clone() {
            TokenKind::Number(text) => ParsedExpr::new(Expr::Number(text), 1)?,
            TokenKind::Ident(name) => {
                if matches!(self.peek().kind, TokenKind::LParen) {
                    self.advance();
                    let (args, height) = self.parse_args()?;
                    ParsedExpr::new(Expr::Call { name, args }, height + 1)?
                } else {
                    ParsedExpr::new(Expr::Var(name), 1)?
                }
            }
            TokenKind::Minus => {
                let inner = self.parse_expr(PREFIX_BP)?;
                ParsedExpr::new(
                    Expr::Unary {
                        op: UnOp::Neg,
                        expr: Box::new(inner.expr),
                    },
                    inner.height + 1,
                )?
            }
            TokenKind::Plus => {
                let inner = self.parse_expr(PREFIX_BP)?;
                ParsedExpr::new(
                    Expr::Unary {
                        op: UnOp::Pos,
                        expr: Box::new(inner.expr),
                    },
                    inner.height + 1,
                )?
            }
            TokenKind::LParen => {
                let inner = self.parse_expr(0)?;
                self.expect(TokenKind::RParen)?;
                inner
            }
            _ => return Err(self.unexpected(&tok)),
        };

        loop {
            let kind = self.peek().kind.clone();
            if kind == TokenKind::Bang {
                if POSTFIX_BP < min_bp {
                    break;
                }
                self.advance();
                lhs = ParsedExpr::new(Expr::Factorial(Box::new(lhs.expr)), lhs.height + 1)?;
                continue;
            }
            let (op, (l_bp, r_bp)) = match kind {
                TokenKind::Plus => (BinOp::Add, ADD_BP),
                TokenKind::Minus => (BinOp::Sub, ADD_BP),
                TokenKind::Star => (BinOp::Mul, MUL_BP),
                TokenKind::Slash => (BinOp::Div, MUL_BP),
                TokenKind::Percent => (BinOp::Mod, MUL_BP),
                TokenKind::Caret => (BinOp::Pow, POW_BP),
                _ => break,
            };
            if l_bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr(r_bp)?;
            let height = lhs.height.max(rhs.height) + 1;
            lhs = ParsedExpr::new(
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs.expr),
                    rhs: Box::new(rhs.expr),
                },
                height,
            )?;
        }
        Ok(lhs)
    }

    fn parse_args(&mut self) -> Result<(Vec<Expr>, usize), ParseError> {
        let mut args = Vec::new();
        let mut height = 0;
        if matches!(self.peek().kind, TokenKind::RParen) {
            self.advance();
            return Ok((args, height));
        }
        loop {
            let arg = self.parse_expr(0)?;
            height = height.max(arg.height);
            args.push(arg.expr);
            let tok = self.advance();
            match tok.kind {
                TokenKind::Comma => continue,
                TokenKind::RParen => return Ok((args, height)),
                _ => return Err(self.unexpected(&tok)),
            }
        }
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Number(t) | TokenKind::Ident(t) => t.clone(),
        TokenKind::Plus => "+".into(),
        TokenKind::Minus => "-".into(),
        TokenKind::Star => "*".into(),
        TokenKind::Slash => "/".into(),
        TokenKind::Percent => "%".into(),
        TokenKind::Caret => "^".into(),
        TokenKind::Bang => "!".into(),
        TokenKind::LParen => "(".into(),
        TokenKind::RParen => ")".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Assign => "=".into(),
        TokenKind::Eof => "end of input".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(input: &str) -> String {
        parse(input).unwrap().to_string()
    }

    #[test]
    fn precedence_of_binary_operators() {
        assert_eq!(show("1+2*3"), "1+2*3");
        assert_eq!(show("(1+2)*3"), "(1+2)*3");
        assert_eq!(show("1-2-3"), "1-2-3");
        assert_eq!(show("1-(2-3)"), "1-(2-3)");
        assert_eq!(show("8/4/2"), "8/4/2");
        assert_eq!(show("8/(4/2)"), "8/(4/2)");
    }

    #[test]
    fn subtraction_is_left_associative() {
        match parse("10-4-3").unwrap() {
            Expr::Binary {
                op: BinOp::Sub, lhs, ..
            } => {
                assert!(matches!(*lhs, Expr::Binary { op: BinOp::Sub, .. }));
            }
            other => panic!("unexpected tree {other:?}"),
        }
    }

    #[test]
    fn power_is_right_associative() {
        match parse("2^3^2").unwrap() {
            Expr::Binary {
                op: BinOp::Pow, rhs, ..
            } => {
                assert!(matches!(*rhs, Expr::Binary { op: BinOp::Pow, .. }));
            }
            other => panic!("unexpected tree {other:?}"),
        }
        assert_eq!(show("(2^3)^2"), "(2^3)^2");
    }

    #[test]
    fn unary_minus_binds_looser_than_power() {
        assert_eq!(show("-2^2"), "-2^2");
        match parse("-2^2").unwrap() {
            Expr::Unary { op: UnOp::Neg, expr } => {
                assert!(matches!(*expr, Expr::Binary { op: BinOp::Pow, .. }))
            }
            other => panic!("unexpected tree {other:?}"),
        }
        assert_eq!(show("(-2)^2"), "(-2)^2");
        assert_eq!(show("2^-3"), "2^(-3)");
        assert_eq!(show("2*-3"), "2*-3");
    }

    #[test]
    fn factorial_is_postfix_and_tight() {
        assert_eq!(show("3!"), "3!");
        assert_eq!(show("-3!"), "-3!");
        assert_eq!(show("(2+1)!"), "(2+1)!");
        match parse("-3!").unwrap() {
            Expr::Unary { expr, .. } => assert!(matches!(*expr, Expr::Factorial(_))),
            other => panic!("unexpected tree {other:?}"),
        }
        assert_eq!(show("3!!"), "3!!");
    }

    #[test]
    fn calls_and_arguments() {
        assert_eq!(show("max(1, 2+3, x)"), "max(1, 2+3, x)");
        assert_eq!(show("pi()"), "pi()");
        assert_eq!(show("min(max(1,2),3)"), "min(max(1, 2), 3)");
        assert!(matches!(parse("max(1,"), Err(ParseError::UnexpectedEof { .. })));
        assert!(matches!(
            parse("max(1 2)"),
            Err(ParseError::UnexpectedToken { .. })
        ));
    }

    #[test]
    fn assignment() {
        assert_eq!(show("x = 1 + 2"), "x = 1+2");
        assert!(matches!(
            parse("x = y = 2"),
            Err(ParseError::UnexpectedToken { .. })
        ));
        assert!(matches!(parse("1 = 2"), Err(ParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn errors_carry_positions() {
        assert_eq!(parse("1 +").unwrap_err(), ParseError::UnexpectedEof { pos: 3 });
        assert_eq!(parse("1 + * 2").unwrap_err().position(), Some(4));
        assert_eq!(parse("(1+2").unwrap_err(), ParseError::UnexpectedEof { pos: 4 });
        assert_eq!(parse("1 2").unwrap_err().position(), Some(2));
        assert_eq!(parse("1 + $").unwrap_err().position(), Some(4));
        assert!(matches!(parse(")"), Err(ParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(matches!(parse(""), Err(ParseError::UnexpectedEof { pos: 0 })));
    }

    #[test]
    fn deep_nesting_is_rejected() {
        let ok = format!("{}1{}", "(".repeat(100), ")".repeat(100));
        assert!(parse(&ok).is_ok());
        let deep = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
        assert_eq!(parse(&deep).unwrap_err(), ParseError::TooDeep);
    }

    #[test]
    fn flat_tree_limit_has_a_safe_boundary() {
        let sum = std::iter::repeat_n("1", MAX_TREE_DEPTH)
            .collect::<Vec<_>>()
            .join("+");
        assert_eq!(show(&sum), sum);
        assert_eq!(parse(&format!("{sum}+1")).unwrap_err(), ParseError::TooDeep);
        assert_eq!(parse(&format!("x = {sum}")).unwrap_err(), ParseError::TooDeep);
        assert_eq!(parse(&format!("f({sum})")).unwrap_err(), ParseError::TooDeep);
    }

    #[test]
    fn postfix_tree_limit_has_a_safe_boundary() {
        let postfix = format!("1{}", "!".repeat(MAX_TREE_DEPTH - 1));
        assert_eq!(show(&postfix), postfix);
        assert_eq!(parse(&format!("{postfix}!")).unwrap_err(), ParseError::TooDeep);
    }

    #[test]
    fn display_round_trips() {
        for input in [
            "1+2*3-4/5%6",
            "-(1+2)^3!",
            "f(1, g(2), -x)^2",
            "a = (1+2)*3",
            "2^(3^2)",
            "(2^3)^2",
        ] {
            let tree = parse(input).unwrap();
            let again = parse(&tree.to_string()).unwrap();
            assert_eq!(tree, again, "round trip of {input}");
        }
    }
}
