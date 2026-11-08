//! Calls into the Python `calcstats` package over a JSON-lines pipe.
//!
//! Requests are `{"id":1,"fn":"mean","args":["1","2"]}`; replies are
//! `{"id":1,"ok":true,"value":"3/2"}` or `{"id":1,"ok":false,"error":{"code":"DOMAIN","message":"..."}}`.
//! Numbers travel as exact strings (`"7"` or `"1/3"`) so no precision is lost on the way.

use crate::error::{ErrorCode, EvalError};
use crate::number::Rational;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub const EXTERNAL_FUNCS: &[&str] = &[
    "mean",
    "median",
    "mode",
    "variance",
    "stddev",
    "pvariance",
    "pstddev",
    "percentile",
    "pmt",
    "fv",
    "pv",
    "npv",
    "irr",
    "compound",
];

pub trait Bridge {
    fn supports(&self, name: &str) -> bool;
    fn call(&mut self, name: &str, args: &[Rational]) -> Result<Rational, EvalError>;
}

/// A bridge with no external functions.
pub struct NoBridge;

impl Bridge for NoBridge {
    fn supports(&self, _name: &str) -> bool {
        false
    }

    fn call(&mut self, name: &str, _args: &[Rational]) -> Result<Rational, EvalError> {
        Err(EvalError::new(
            ErrorCode::UnknownFunc,
            format!("unknown function '{name}'"),
        ))
    }
}

struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct PythonBridge {
    python: String,
    dir: PathBuf,
    session: Option<Session>,
    next_id: u64,
}

impl Default for PythonBridge {
    fn default() -> Self {
        PythonBridge::new()
    }
}

impl PythonBridge {
    pub fn new() -> Self {
        let python = std::env::var("CALC_PYTHON").unwrap_or_else(|_| "python3".to_string());
        let dir = std::env::var("CALC_PYTHON_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../python"));
        PythonBridge {
            python,
            dir,
            session: None,
            next_id: 1,
        }
    }

    fn external(message: impl Into<String>) -> EvalError {
        EvalError::new(ErrorCode::External, message)
    }

    fn start(&self) -> Result<Session, EvalError> {
        let mut child = Command::new(&self.python)
            .args(["-m", "calcstats"])
            .env("PYTHONPATH", &self.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Self::external(format!("cannot start {}: {e}", self.python)))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Self::external("python stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Self::external("python stdout unavailable"))?;
        Ok(Session {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    fn roundtrip(&mut self, request: &Value) -> Result<Value, EvalError> {
        if self.session.is_none() {
            self.session = Some(self.start()?);
        }
        let session = self.session.as_mut().expect("session started above");
        writeln!(session.stdin, "{request}")
            .map_err(|e| Self::external(format!("write to python failed: {e}")))?;
        session
            .stdin
            .flush()
            .map_err(|e| Self::external(format!("write to python failed: {e}")))?;
        let mut line = String::new();
        let read = session
            .stdout
            .read_line(&mut line)
            .map_err(|e| Self::external(format!("read from python failed: {e}")))?;
        if read == 0 {
            return Err(Self::external("python process exited"));
        }
        serde_json::from_str(&line).map_err(|e| Self::external(format!("invalid reply from python: {e}")))
    }
}

impl Bridge for PythonBridge {
    fn supports(&self, name: &str) -> bool {
        EXTERNAL_FUNCS.contains(&name)
    }

    fn call(&mut self, name: &str, args: &[Rational]) -> Result<Rational, EvalError> {
        let id = self.next_id;
        self.next_id += 1;
        let wire_args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let request = json!({ "id": id, "fn": name, "args": wire_args });
        let reply = match self.roundtrip(&request) {
            Ok(reply) => reply,
            Err(e) => {
                self.session = None;
                return Err(e);
            }
        };
        if reply["id"] != json!(id) {
            self.session = None;
            return Err(Self::external("mismatched reply id from python"));
        }
        if reply["ok"] == json!(true) {
            let text = reply["value"]
                .as_str()
                .ok_or_else(|| Self::external("reply has no value"))?;
            Rational::parse(text)
        } else {
            let code = reply["error"]["code"].as_str().unwrap_or("EXTERNAL");
            let message = reply["error"]["message"].as_str().unwrap_or("python call failed");
            Err(EvalError::new(ErrorCode::from_wire(code), message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_known_functions() {
        let bridge = PythonBridge::new();
        assert!(bridge.supports("mean"));
        assert!(bridge.supports("pmt"));
        assert!(!bridge.supports("sqrt"));
    }

    #[test]
    fn no_bridge_rejects_everything() {
        let mut bridge = NoBridge;
        assert!(!bridge.supports("mean"));
        assert_eq!(bridge.call("mean", &[]).unwrap_err().code, ErrorCode::UnknownFunc);
    }

    #[test]
    fn missing_interpreter_is_an_external_error() {
        let mut bridge = PythonBridge::new();
        bridge.python = "definitely-not-a-python-binary".to_string();
        let err = bridge.call("mean", &[Rational::ONE]).unwrap_err();
        assert_eq!(err.code, ErrorCode::External);
        assert!(err.message.contains("cannot start"));
    }
}
