//! `calc`: evaluate one expression, or run a JSON-lines session over stdin/stdout.

mod server;

use calc_eval::{Calculator, FormatOptions, PythonBridge, RoundMode};
use std::process::ExitCode;

const USAGE: &str = "usage:
  calc eval <expression> [--places N] [--mode half-up|half-even|truncate]
  calc serve [--places N] [--mode half-up|half-even|truncate]
  calc --version";

struct Args {
    command: String,
    expression: Option<String>,
    options: FormatOptions,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let command = args.first().cloned().ok_or("missing command")?;
    let mut expression = None;
    let mut places = FormatOptions::default().places;
    let mut mode = FormatOptions::default().mode;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--places" => {
                i += 1;
                let value = args.get(i).ok_or("--places needs a value")?;
                places = value
                    .parse()
                    .map_err(|_| format!("invalid --places value '{value}'"))?;
            }
            "--mode" => {
                i += 1;
                let value = args.get(i).ok_or("--mode needs a value")?;
                mode = value.parse::<RoundMode>()?;
            }
            other if expression.is_none() && !other.starts_with("--") => expression = Some(other.to_string()),
            other => return Err(format!("unexpected argument '{other}'")),
        }
        i += 1;
    }
    let options = FormatOptions::new(places, mode).map_err(|e| e.message)?;
    Ok(Args {
        command,
        expression,
        options,
    })
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.first().map(String::as_str) == Some("--version") {
        println!("calc {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(&raw) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("calc: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let mut calculator = Calculator::new(Box::new(PythonBridge::new()));
    calculator.options = args.options;
    match args.command.as_str() {
        "eval" => {
            let Some(expression) = args.expression else {
                eprintln!("calc: eval needs an expression\n{USAGE}");
                return ExitCode::from(2);
            };
            match calculator.eval_line(&expression) {
                Ok(result) => {
                    println!("{}", result.text);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error[{}]: {}", e.code.as_str(), e.message);
                    ExitCode::from(1)
                }
            }
        }
        "serve" => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            match server::serve(&mut calculator, stdin.lock(), stdout.lock()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("calc: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("calc: unknown command '{other}'\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_eval_with_options() {
        let parsed = parse_args(&args(&["eval", "1+2", "--places", "3", "--mode", "truncate"])).unwrap();
        assert_eq!(parsed.command, "eval");
        assert_eq!(parsed.expression.as_deref(), Some("1+2"));
        assert_eq!(parsed.options.places, 3);
        assert_eq!(parsed.options.mode, RoundMode::Truncate);
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&args(&["eval", "1", "--places"])).is_err());
        assert!(parse_args(&args(&["eval", "1", "--places", "x"])).is_err());
        assert!(parse_args(&args(&["eval", "1", "--places", "99"])).is_err());
        assert!(parse_args(&args(&["eval", "1", "--mode", "sideways"])).is_err());
        assert!(parse_args(&args(&["eval", "1", "2"])).is_err());
    }

    #[test]
    fn negative_expressions_are_accepted_as_expressions() {
        let parsed = parse_args(&args(&["eval", "-5+1"])).unwrap();
        assert_eq!(parsed.expression.as_deref(), Some("-5+1"));
    }
}
