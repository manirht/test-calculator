//! JSON-lines session: one request object per line in, one reply object per line out.
//!
//! Request: `{"id": 1, "expr": "1+2", "places": 4, "mode": "half-even"}`, or `{"id": 2, "cmd": "reset"}`.
//! Reply: `{"id": 1, "ok": true, "value": "3", "exact": "3"}` or
//! `{"id": 1, "ok": false, "error": {"code": "DIV_BY_ZERO", "message": "...", "pos": 4}}`.

use calc_eval::{Calculator, FormatOptions, RoundMode};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn error_reply(id: &Value, code: &str, message: &str, pos: Option<usize>) -> Value {
    let mut error = json!({ "code": code, "message": message });
    if let Some(pos) = pos {
        error["pos"] = json!(pos);
    }
    json!({ "id": id, "ok": false, "error": error })
}

fn options_for(calculator: &Calculator, request: &Value) -> Result<FormatOptions, String> {
    let mut places = calculator.options.places;
    let mut mode = calculator.options.mode;
    if let Some(value) = request.get("places") {
        places = value
            .as_u64()
            .and_then(|p| u32::try_from(p).ok())
            .ok_or_else(|| "places must be a non-negative integer".to_string())?;
    }
    if let Some(value) = request.get("mode") {
        mode = value
            .as_str()
            .ok_or("mode must be a string")?
            .parse::<RoundMode>()?;
    }
    FormatOptions::new(places, mode).map_err(|e| e.message)
}

pub fn handle(calculator: &mut Calculator, line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(e) => return error_reply(&Value::Null, "BAD_REQUEST", &format!("invalid JSON: {e}"), None),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if let Some(cmd) = request.get("cmd").and_then(Value::as_str) {
        return match cmd {
            "reset" => {
                calculator.reset();
                json!({ "id": id, "ok": true })
            }
            "ping" => json!({ "id": id, "ok": true, "value": "pong" }),
            "functions" => json!({ "id": id, "ok": true, "functions": calculator.function_names() }),
            other => error_reply(&id, "BAD_REQUEST", &format!("unknown command '{other}'"), None),
        };
    }
    let Some(expr) = request.get("expr").and_then(Value::as_str) else {
        return error_reply(&id, "BAD_REQUEST", "request needs an 'expr' string", None);
    };
    let options = match options_for(calculator, &request) {
        Ok(options) => options,
        Err(message) => return error_reply(&id, "BAD_REQUEST", &message, None),
    };
    match calculator.eval_line_with(expr, options) {
        Ok(result) => json!({ "id": id, "ok": true, "value": result.text, "exact": result.exact }),
        Err(e) => error_reply(&id, e.code.as_str(), &e.message, e.pos),
    }
}

pub fn serve<R: BufRead, W: Write>(
    calculator: &mut Calculator,
    input: R,
    mut output: W,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = handle(calculator, &line);
        writeln!(output, "{reply}")?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use calc_eval::NoBridge;

    fn calc() -> Calculator {
        Calculator::new(Box::new(NoBridge))
    }

    fn ask(c: &mut Calculator, line: &str) -> Value {
        handle(c, line)
    }

    #[test]
    fn successful_evaluation() {
        let mut c = calc();
        let reply = ask(&mut c, r#"{"id": 7, "expr": "1/3"}"#);
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["value"], "0.3333333333");
        assert_eq!(reply["exact"], "1/3");
    }

    #[test]
    fn errors_carry_code_and_position() {
        let mut c = calc();
        let reply = ask(&mut c, r#"{"id": "a", "expr": "1 + * 2"}"#);
        assert_eq!(reply["id"], "a");
        assert_eq!(reply["ok"], false);
        assert_eq!(reply["error"]["code"], "SYNTAX");
        assert_eq!(reply["error"]["pos"], 4);
        let reply = ask(&mut c, r#"{"id": 1, "expr": "1/0"}"#);
        assert_eq!(reply["error"]["code"], "DIV_BY_ZERO");
        assert!(reply["error"].get("pos").is_none());
    }

    #[test]
    fn session_state_persists_and_resets() {
        let mut c = calc();
        ask(&mut c, r#"{"id": 1, "expr": "x = 6"}"#);
        assert_eq!(ask(&mut c, r#"{"id": 2, "expr": "x * 7"}"#)["value"], "42");
        assert_eq!(ask(&mut c, r#"{"id": 3, "expr": "ans + 1"}"#)["value"], "43");
        assert_eq!(ask(&mut c, r#"{"id": 4, "cmd": "reset"}"#)["ok"], true);
        assert_eq!(
            ask(&mut c, r#"{"id": 5, "expr": "x"}"#)["error"]["code"],
            "UNKNOWN_VAR"
        );
    }

    #[test]
    fn per_request_formatting() {
        let mut c = calc();
        let reply = ask(
            &mut c,
            r#"{"id": 1, "expr": "2/3", "places": 2, "mode": "truncate"}"#,
        );
        assert_eq!(reply["value"], "0.66");
        let reply = ask(&mut c, r#"{"id": 2, "expr": "2/3"}"#);
        assert_eq!(reply["value"], "0.6666666667");
    }

    #[test]
    fn malformed_requests() {
        let mut c = calc();
        assert_eq!(ask(&mut c, "not json")["error"]["code"], "BAD_REQUEST");
        assert_eq!(ask(&mut c, r#"{"id": 1}"#)["error"]["code"], "BAD_REQUEST");
        assert_eq!(
            ask(&mut c, r#"{"id": 1, "expr": "1", "places": -1}"#)["error"]["code"],
            "BAD_REQUEST"
        );
        assert_eq!(
            ask(&mut c, r#"{"id": 1, "expr": "1", "mode": "x"}"#)["error"]["code"],
            "BAD_REQUEST"
        );
        assert_eq!(
            ask(&mut c, r#"{"id": 1, "cmd": "dance"}"#)["error"]["code"],
            "BAD_REQUEST"
        );
    }

    #[test]
    fn serve_processes_lines_in_order() {
        let mut c = calc();
        let input = "{\"id\":1,\"expr\":\"1+1\"}\n\n{\"id\":2,\"expr\":\"ans*5\"}\n";
        let mut out = Vec::new();
        serve(&mut c, input.as_bytes(), &mut out).unwrap();
        let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"value\":\"2\""));
        assert!(lines[1].contains("\"value\":\"10\""));
    }

    #[test]
    fn functions_command_lists_names() {
        let mut c = calc();
        let reply = ask(&mut c, r#"{"id": 1, "cmd": "functions"}"#);
        let names: Vec<&str> = reply["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(names.contains(&"sqrt"));
        assert!(names.contains(&"pmt"));
    }
}
