use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

fn make_jsonl(docs: &[&str]) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for text in docs {
        writeln!(f, r#"{{"text": "{text}"}}"#).unwrap();
    }
    f.flush().unwrap();
    f
}

/// 10 docs with 3 exact duplicates → output has 7 unique docs
#[test]
fn ten_docs_three_exact_duplicates() {
    let docs = vec![
        "alpha", "bravo", "charlie", "alpha", "delta", "echo", "bravo", "foxtrot", "golf",
        "alpha",
    ];
    // unique: alpha, bravo, charlie, delta, echo, foxtrot, golf = 7
    let input = make_jsonl(&docs);

    let output = Command::cargo_bin("textsift")
        .unwrap()
        .args(["--field", "text", "--stats"])
        .arg(input.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 7, "expected 7 unique docs, got {}", lines.len());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exact duplicates: 3"));
}

/// Two empty strings are duplicates of each other
#[test]
fn empty_text_field_counts_as_duplicate() {
    let input = make_jsonl(&["", ""]);

    let output = Command::cargo_bin("textsift")
        .unwrap()
        .args(["--field", "text", "--stats"])
        .arg(input.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 1, "only one empty string should survive");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exact duplicates: 1"));
}

/// All docs identical → only the first survives
#[test]
fn all_identical_docs() {
    let docs = vec!["same"; 5];
    let input = make_jsonl(&docs);

    let output = Command::cargo_bin("textsift")
        .unwrap()
        .args(["--field", "text", "--stats"])
        .arg(input.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 1);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exact duplicates: 4"));
    assert!(stderr.contains("unique docs emitted: 1"));
}

/// Zero duplicates → nothing flagged, all pass through
#[test]
fn zero_duplicates() {
    let docs = vec!["one", "two", "three", "four"];
    let input = make_jsonl(&docs);

    let output = Command::cargo_bin("textsift")
        .unwrap()
        .args(["--field", "text", "--stats"])
        .arg(input.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 4);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exact duplicates: 0"));
    assert!(stderr.contains("unique docs emitted: 4"));
}

/// ExactDedup::new() starts clean — calling the binary twice doesn't share state
#[test]
fn fresh_instance_has_no_entries() {
    let input = make_jsonl(&["hello"]);

    // First run
    let out1 = Command::cargo_bin("textsift")
        .unwrap()
        .args(["--field", "text", "--stats"])
        .arg(input.path())
        .output()
        .unwrap();

    // Second run on same input — should also output 1 doc (no cross-run state)
    let out2 = Command::cargo_bin("textsift")
        .unwrap()
        .args(["--field", "text", "--stats"])
        .arg(input.path())
        .output()
        .unwrap();

    let stdout1 = String::from_utf8_lossy(&out1.stdout);
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert_eq!(stdout1.lines().count(), 1);
    assert_eq!(stdout2.lines().count(), 1);

    let stderr2 = String::from_utf8_lossy(&out2.stderr);
    assert!(stderr2.contains("exact duplicates: 0"));
}
