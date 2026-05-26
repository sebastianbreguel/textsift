use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn textsift() -> Command {
    Command::cargo_bin("textsift").unwrap()
}

#[test]
fn passthrough_preserves_all_fields() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"text":"hello","id":1,"meta":"keep"}}"#).unwrap();
    writeln!(file, r#"{{"text":"world","id":2,"extra":true}}"#).unwrap();

    let output = textsift()
        .arg(file.path())
        .args(["--field", "text"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2);

    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["meta"], "keep");
    let v: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(v["extra"], true);
}

#[test]
fn reads_from_stdin() {
    let output = textsift()
        .arg("-")
        .args(["--field", "text"])
        .write_stdin(r#"{"text":"stdin works","id":1}"#)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("stdin works"));
}

#[test]
fn missing_field_flag_errors() {
    textsift()
        .arg("-")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--field"));
}

#[test]
fn missing_input_file_errors() {
    textsift()
        .arg("nonexistent.jsonl")
        .args(["--field", "text"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot open"));
}

#[test]
fn skips_lines_with_missing_field() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"text":"keep","id":1}}"#).unwrap();
    writeln!(file, r#"{{"other":"skip","id":2}}"#).unwrap();
    writeln!(file, r#"{{"text":"also keep","id":3}}"#).unwrap();

    let output = textsift()
        .arg(file.path())
        .args(["--field", "text"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2);

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing field"));
}

#[test]
fn nested_json_preserved() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{"text":"hello","nested":{{"a":1,"b":[2,3]}}}}"#
    )
    .unwrap();

    let output = textsift()
        .arg(file.path())
        .args(["--field", "text"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["nested"]["a"], 1);
    assert_eq!(v["nested"]["b"][1], 3);
}
