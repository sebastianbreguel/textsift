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
fn lines_with_missing_field_pass_through_with_warning() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"text":"keep","id":1}}"#).unwrap();
    writeln!(file, r#"{{"other":"fieldless","id":2}}"#).unwrap();
    writeln!(file, r#"{{"text":"also keep","id":3}}"#).unwrap();

    let output = textsift()
        .arg(file.path())
        .args(["--field", "text"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 3, "fieldless docs are not dropped");
    assert!(stdout.contains("fieldless"));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing field"));
}

#[test]
fn nested_json_preserved() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"text":"hello","nested":{{"a":1,"b":[2,3]}}}}"#).unwrap();

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

#[test]
fn rejects_zero_shingle_size() {
    textsift()
        .arg("-")
        .args(["--field", "text", "--shingle-size", "0"])
        .write_stdin(r#"{"text":"a b c"}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("shingle_size must be at least 1"));
}

#[test]
fn rejects_zero_num_perm() {
    textsift()
        .arg("-")
        .args(["--field", "text", "--num-perm", "0"])
        .write_stdin(r#"{"text":"a b c"}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("num_perm must be at least 1"));
}

#[test]
fn rejects_out_of_range_threshold() {
    textsift()
        .arg("-")
        .args(["--field", "text", "--threshold", "1.5"])
        .write_stdin(r#"{"text":"a b c"}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("threshold must be between"));
}

#[test]
fn passthrough_is_byte_identical() {
    // Odd key order, extra spacing — output must be the exact input line.
    let input = r#"{"z": 1,  "a": 2, "text": "byte identical check"}"#;
    let output = textsift()
        .arg("-")
        .args(["--field", "text"])
        .write_stdin(input)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim_end(), input);
}

#[test]
fn docs_missing_field_pass_through() {
    let output = textsift()
        .arg("-")
        .args(["--field", "text"])
        .write_stdin("{\"text\":\"has the field\"}\n{\"other\":\"no text field\"}\n")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stdout.trim().lines().count(),
        2,
        "missing-field doc is kept"
    );
    assert!(stdout.contains("no text field"));
    assert!(stderr.contains("missing field"));
}

#[test]
fn clusters_mode_assigns_cluster_to_missing_field_docs() {
    let output = textsift()
        .arg("-")
        .args(["--field", "text", "--clusters"])
        .write_stdin("{\"text\":\"some doc\"}\n{\"other\":\"fieldless\"}\n")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    let v: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(v["is_representative"], true);
    assert!(v["cluster_id"].is_u64());
}
