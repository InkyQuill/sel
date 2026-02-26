//! Integration tests for basic selectors.

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Helper to run sel with arguments and return stdout.
fn run_sel(args: &[&str]) -> String {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(args)
        .output()
        .expect("Failed to execute sel");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper to run sel with arguments and return both stdout and stderr.
fn run_sel_with_result(args: &[&str]) -> (String, String, i32) {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(args)
        .output()
        .expect("Failed to execute sel");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    (stdout, stderr, code)
}

/// Helper to create a test file with lines.
fn create_test_file(lines: &[&str]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for line in lines {
        writeln!(file, "{}", line).unwrap();
    }
    file.flush().unwrap();
    file
}

/// Helper to create a test file with custom content.
fn create_test_file_with_content(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn test_single_line() {
    let file = create_test_file(&["line1", "line2", "line3"]);
    let output = run_sel(&["2", file.path().to_str().unwrap()]);

    assert!(output.contains("2:line2"));
    assert!(!output.contains("1:line1"));
    assert!(!output.contains("3:line3"));
}

#[test]
fn test_first_line() {
    let file = create_test_file(&["alpha", "beta", "gamma"]);
    let output = run_sel(&["1", file.path().to_str().unwrap()]);

    assert!(output.contains("1:alpha"));
    assert!(!output.contains("beta"));
}

#[test]
fn test_last_line() {
    let file = create_test_file(&["first", "second", "third"]);
    let output = run_sel(&["3", file.path().to_str().unwrap()]);

    assert!(output.contains("3:third"));
    assert!(!output.contains("first"));
}

#[test]
fn test_simple_range() {
    let file = create_test_file(&["l1", "l2", "l3", "l4", "l5"]);
    let output = run_sel(&["2-4", file.path().to_str().unwrap()]);

    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 3);

    assert!(output.contains("2:l2"));
    assert!(output.contains("3:l3"));
    assert!(output.contains("4:l4"));
    assert!(!output.contains("1:l1"));
    assert!(!output.contains("5:l5"));
}

#[test]
fn test_full_range() {
    let file = create_test_file(&["a", "b", "c", "d", "e"]);
    let output = run_sel(&["1-5", file.path().to_str().unwrap()]);

    assert!(output.contains("1:a"));
    assert!(output.contains("2:b"));
    assert!(output.contains("3:c"));
    assert!(output.contains("4:d"));
    assert!(output.contains("5:e"));
}

#[test]
fn test_multiple_single_lines() {
    let file = create_test_file(&["line1", "line2", "line3", "line4", "line5"]);
    let output = run_sel(&["1,3,5", file.path().to_str().unwrap()]);

    assert!(output.contains("1:line1"));
    assert!(output.contains("3:line3"));
    assert!(output.contains("5:line5"));
    assert!(!output.contains("2:line2"));
    assert!(!output.contains("4:line4"));
}

#[test]
fn test_mixed_selector() {
    let file = create_test_file(&["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10"]);
    let output = run_sel(&["1,3-5,8", file.path().to_str().unwrap()]);

    assert!(output.contains("1:l1"));
    assert!(output.contains("3:l3"));
    assert!(output.contains("4:l4"));
    assert!(output.contains("5:l5"));
    assert!(output.contains("8:l8"));

    assert!(!output.contains("2:l2"));
    assert!(!output.contains("6:l6"));
    assert!(!output.contains("7:l7"));
}

#[test]
fn test_multiple_ranges() {
    let file = create_test_file(&["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8"]);
    let output = run_sel(&["1-2,5-6", file.path().to_str().unwrap()]);

    assert!(output.contains("1:l1"));
    assert!(output.contains("2:l2"));
    assert!(output.contains("5:l5"));
    assert!(output.contains("6:l6"));

    assert!(!output.contains("3:l3"));
    assert!(!output.contains("4:l4"));
}

#[test]
fn test_complex_comma_selector() {
    let file = create_test_file(&[
        "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10", "l11", "l12", "l13", "l14",
        "l15",
    ]);
    let output = run_sel(&["1,3-5,10,12-15", file.path().to_str().unwrap()]);

    assert!(output.contains("1:l1"));
    assert!(output.contains("3:l3"));
    assert!(output.contains("4:l4"));
    assert!(output.contains("5:l5"));
    assert!(output.contains("10:l10"));
    assert!(output.contains("12:l12"));
    assert!(output.contains("15:l15"));
}

#[test]
fn test_no_line_numbers_flag() {
    let file = create_test_file(&["line1", "line2", "line3"]);
    let output = run_sel(&["-l", "2", file.path().to_str().unwrap()]);

    // Without -l, output would be "2:line2"
    // With -l, output should be just "line2"
    assert!(output.contains("line2"));
    assert!(!output.contains("2:line2"));
    assert!(!output.contains(":line2"));
}

#[test]
fn test_range_with_no_line_numbers() {
    let file = create_test_file(&["a", "b", "c", "d"]);
    let output = run_sel(&["-l", "2-3", file.path().to_str().unwrap()]);

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "b");
    assert_eq!(lines[1], "c");
}

#[test]
fn test_all_lines_no_selector() {
    let file = create_test_file(&["first", "second", "third"]);
    let output = run_sel(&[file.path().to_str().unwrap()]);

    // When no selector is provided, all lines are output
    assert!(output.contains("1:first"));
    assert!(output.contains("2:second"));
    assert!(output.contains("3:third"));
}

#[test]
fn test_single_line_range() {
    let file = create_test_file(&["x", "y", "z"]);
    let output = run_sel(&["2-2", file.path().to_str().unwrap()]);

    assert!(output.contains("2:y"));
    assert!(!output.contains("x"));
    assert!(!output.contains("z"));
}

#[test]
fn test_empty_file() {
    let file = create_test_file_with_content("");
    let output = run_sel(&["1", file.path().to_str().unwrap()]);

    assert!(output.is_empty());
}

#[test]
fn test_line_beyond_file() {
    let file = create_test_file(&["a", "b", "c"]);
    let output = run_sel(&["10", file.path().to_str().unwrap()]);

    // Should produce empty output
    assert!(output.is_empty() || !output.contains("a"));
}

#[test]
fn test_duplicate_line_numbers() {
    let file = create_test_file(&["a", "b", "c"]);
    let output = run_sel(&["2,2,2", file.path().to_str().unwrap()]);

    // Should only show line 2 once
    assert!(output.contains("2:b"));
    let count = output.matches("2:b").count();
    assert_eq!(count, 1);
}

#[test]
fn test_overlapping_ranges() {
    let file = create_test_file(&["l1", "l2", "l3", "l4", "l5", "l6"]);
    let output = run_sel(&["1-4,3-6", file.path().to_str().unwrap()]);

    // All lines should appear, but without duplicates
    assert!(output.contains("1:l1"));
    assert!(output.contains("2:l2"));
    assert!(output.contains("3:l3"));
    assert!(output.contains("4:l4"));
    assert!(output.contains("5:l5"));
    assert!(output.contains("6:l6"));

    // Each line should appear only once
    for i in 1..=6 {
        assert_eq!(output.matches(&format!("{}:l{}", i, i)).count(), 1);
    }
}

#[test]
fn test_long_file() {
    let lines: Vec<String> = (1..=100).map(|i| format!("line{}", i)).collect();
    let file = create_test_file(&lines.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let output = run_sel(&["50-55", file.path().to_str().unwrap()]);

    assert!(output.contains("50:line50"));
    assert!(output.contains("55:line55"));
    assert!(!output.contains("49:line49"));
    assert!(!output.contains("56:line56"));
}

#[test]
fn test_unicode_content() {
    let file = create_test_file(&["Hello 世界", "Привет мир", "こんにちは"]);
    let output = run_sel(&["2", file.path().to_str().unwrap()]);

    assert!(output.contains("Привет мир"));
}

#[test]
fn test_long_line() {
    let long_line = "a".repeat(1000);
    let file = create_test_file(&["short", &long_line, "another"]);
    let output = run_sel(&["2", file.path().to_str().unwrap()]);

    assert!(output.contains(&long_line));
}

#[test]
fn test_lines_with_special_characters() {
    let file = create_test_file(&[
        "line with spaces",
        "line\twith\ttabs",
        "line:with:colons",
        "line-with-dashes",
    ]);
    let output = run_sel(&["1,3", file.path().to_str().unwrap()]);

    assert!(output.contains("line with spaces"));
    assert!(output.contains("line:with:colons"));
}

#[test]
fn test_invalid_line_number_zero() {
    let file = create_test_file(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_with_result(&["0", file.path().to_str().unwrap()]);

    // Should return error
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("Invalid"));
}

#[test]
fn test_invalid_range_reversed() {
    let file = create_test_file(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_with_result(&["5-3", file.path().to_str().unwrap()]);

    // Should return error
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("Range"));
}
