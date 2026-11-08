//! End-to-end tests through the real `calc` binary, including the Python bridge.
//! Tests that need `python3` skip themselves when it is not installed.

use std::io::Write;
use std::process::{Command, Stdio};

fn calc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_calc"))
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn eval(expr: &str) -> (bool, String, String) {
    let out = calc().args(["eval", expr]).output().expect("run calc");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    )
}

fn session(lines: &[&str]) -> Vec<serde_json::Value> {
    let mut child = calc()
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn calc");
    {
        let stdin = child.stdin.as_mut().unwrap();
        for line in lines {
            writeln!(stdin, "{line}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn eval_prints_the_result() {
    assert_eq!(eval("2+3*4"), (true, "14".to_string(), String::new()));
    assert_eq!(eval("1/3").1, "0.3333333333");
}

#[test]
fn eval_reports_errors_on_stderr_with_exit_code_1() {
    let (ok, out, err) = eval("1/0");
    assert!(!ok);
    assert_eq!(out, "");
    assert_eq!(err, "error[DIV_BY_ZERO]: division by zero");
    let output = calc().args(["eval", "1/0"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn usage_errors_exit_with_code_2() {
    let output = calc().args(["eval"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let output = calc().args(["frobnicate"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn options_change_the_output() {
    let out = calc()
        .args(["eval", "2/3", "--places", "3", "--mode", "truncate"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "0.666");
}

#[test]
fn serve_keeps_state_between_requests() {
    let replies = session(&[
        r#"{"id":1,"expr":"a = 10"}"#,
        r#"{"id":2,"expr":"a / 4"}"#,
        r#"{"id":3,"expr":"nope"}"#,
    ]);
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[1]["value"], "2.5");
    assert_eq!(replies[2]["error"]["code"], "UNKNOWN_VAR");
}

#[test]
fn python_functions_work_through_the_bridge() {
    if !python_available() {
        return;
    }
    assert_eq!(eval("mean(1, 2, 4)").1, "2.3333333333");
    assert_eq!(eval("median(5, 1, 3)").1, "3");
    assert_eq!(eval("percentile(90, 1, 2, 3, 4, 5)").1, "4.6");
    assert_eq!(eval("pmt(5/1200, 360, 200000)").1, "1073.6432460243");
}

#[test]
fn python_errors_keep_their_codes() {
    if !python_available() {
        return;
    }
    let (ok, _, err) = eval("median()");
    assert!(!ok);
    assert!(err.starts_with("error[DOMAIN]"), "{err}");
    let (_, _, err) = eval("variance(1)");
    assert!(err.starts_with("error[DOMAIN]"), "{err}");
}

#[test]
fn python_results_feed_back_into_expressions() {
    if !python_available() {
        return;
    }
    let replies = session(&[
        r#"{"id":1,"expr":"avg = mean(10, 20, 60)"}"#,
        r#"{"id":2,"expr":"avg * 3"}"#,
        r#"{"id":3,"expr":"round(stddev(2,4,4,4,5,5,7,9), 4)"}"#,
    ]);
    assert_eq!(replies[0]["value"], "30");
    assert_eq!(replies[1]["value"], "90");
    assert_eq!(replies[2]["value"], "2.1381");
}
