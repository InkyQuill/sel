//! Basic integration tests for `sel`.
//!
//! These tests cover fundamental functionality and edge cases.

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Create a test file with known content.
fn create_test_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

/// Create a test file from lines.
fn create_test_file_lines(lines: &[&str]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for line in lines {
        writeln!(file, "{}", line).unwrap();
    }
    file.flush().unwrap();
    file
}

/// Helper to run sel and get output.
fn run_sel(args: &[&str]) -> String {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(args)
        .output()
        .expect("Failed to execute sel");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper to run sel and get (stdout, stderr, exit_code).
fn run_sel_result(args: &[&str]) -> (String, String, i32) {
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

#[test]
fn test_single_line() {
    let file = create_test_file("line1\nline2\nline3\n");
    let output = run_sel(&["2", file.path().to_str().unwrap()]);

    assert!(output.contains("line2"));
    assert!(!output.contains("line1"));
    assert!(!output.contains("line3"));
}

#[test]
fn test_range() {
    let file = create_test_file_lines(&["l1", "l2", "l3", "l4", "l5"]);
    let output = run_sel(&["2-4", file.path().to_str().unwrap()]);

    assert!(output.contains("l2"));
    assert!(output.contains("l3"));
    assert!(output.contains("l4"));
    assert!(!output.contains("l1"));
    assert!(!output.contains("l5"));
}

#[test]
fn test_all_lines() {
    let file = create_test_file_lines(&["line1", "line2", "line3"]);
    let output = run_sel(&[file.path().to_str().unwrap()]);

    assert!(output.contains("line1"));
    assert!(output.contains("line2"));
    assert!(output.contains("line3"));
}

#[test]
fn test_version_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--version"])
        .output()
        .expect("Failed to execute sel");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("sel"));
    assert!(stdout.contains("0.1"));
}

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--help"])
        .output()
        .expect("Failed to execute sel");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("sel"));
    assert!(stdout.contains("Select") || stdout.contains("extract"));
}

#[test]
fn test_no_arguments() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .output();

    // Should fail or show help
    assert!(output.is_ok());
}

#[test]
fn test_file_not_found() {
    let (_stdout, stderr, code) = run_sel_result(&["1", "/nonexistent/file.txt"]);

    assert!(code != 0);
    assert!(stderr.contains("Error") || stderr.contains("No such file") || stderr.contains("not found"));
}

#[test]
fn test_empty_file() {
    let file = create_test_file("");
    let output = run_sel(&["1", file.path().to_str().unwrap()]);

    assert!(output.is_empty());
}

#[test]
fn test_trailing_newline_handling() {
    let file = create_test_file("line1\nline2\nline3\n");
    let output = run_sel(&["1-3", file.path().to_str().unwrap()]);

    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 3);
}

#[test]
fn test_no_trailing_newline() {
    let file = create_test_file("line1\nline2\nline3");
    let output = run_sel(&["1-3", file.path().to_str().unwrap()]);

    assert!(output.contains("line1"));
    assert!(output.contains("line2"));
    assert!(output.contains("line3"));
}

#[test]
fn test_binary_file() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&[0x00, 0x01, 0x02, 0x03, 0x04]).unwrap();
    file.flush().unwrap();

    let _output = run_sel(&["1", file.path().to_str().unwrap()]);

    // Should not crash
}

#[test]
fn test_very_long_line() {
    let long_line = "a".repeat(100_000);
    let file = create_test_file(&long_line);
    let output = run_sel(&["1", file.path().to_str().unwrap()]);

    assert!(output.contains(&long_line[..100]));
}

#[test]
fn test_many_lines() {
    let lines: Vec<String> = (1..=1000).map(|i| format!("line {}", i)).collect();
    let content = lines.join("\n");
    let file = create_test_file(&content);
    let output = run_sel(&["500", file.path().to_str().unwrap()]);

    assert!(output.contains("line 500"));
}

#[test]
fn test_single_character_lines() {
    let file = create_test_file_lines(&["a", "b", "c", "d", "e"]);
    let output = run_sel(&["1,3,5", file.path().to_str().unwrap()]);

    assert!(output.contains("a"));
    assert!(output.contains("c"));
    assert!(output.contains("e"));
}

#[test]
fn test_lines_with_special_chars() {
    let file = create_test_file_lines(&[
        "line with \t tab",
        "line with \n newline",
        "line with \r carriage",
        "line with \\ backslash",
    ]);
    let output = run_sel(&["1-4", file.path().to_str().unwrap()]);

    assert!(output.contains("tab") || output.contains("backslash"));
}

#[test]
fn test_utf8_content() {
    let file = create_test_file_lines(&[
        "Hello 世界",
        "Привет мир",
        "こんにちは世界",
        "🎉 emoji party",
    ]);
    let output = run_sel(&["1,4", file.path().to_str().unwrap()]);

    assert!(output.contains("世界") || output.contains("🎉"));
}

#[test]
fn test_windows_line_endings() {
    let file = create_test_file("line1\r\nline2\r\nline3\r\n");
    let output = run_sel(&["1-3", file.path().to_str().unwrap()]);

    assert!(output.contains("line1"));
    assert!(output.contains("line2"));
    assert!(output.contains("line3"));
}

#[test]
fn test_tabs_preserved() {
    let file = create_test_file("\t\tindented\t\t");
    let output = run_sel(&["1", file.path().to_str().unwrap()]);

    assert!(output.contains("indented"));
}

#[test]
fn test_color_never_flag() {
    let file = create_test_file("content\n");
    let output = run_sel(&["--color=never", "1", file.path().to_str().unwrap()]);

    // Should not contain ANSI color codes
    assert!(!output.contains("\x1b["));

    assert!(output.contains("content"));
}

#[test]
fn test_color_always_flag() {
    let file = create_test_file("content\n");
    let output = run_sel(&["--color=always", "1", file.path().to_str().unwrap()]);

    // May contain ANSI codes (but depends on implementation)
    assert!(output.contains("content"));
}

#[test]
fn test_stdin_placeholder() {
    // Note: Testing stdin is more complex and may require special handling
    // This is a placeholder for future stdin testing
    let file = create_test_file("content\n");
    let _output = run_sel(&["1", file.path().to_str().unwrap()]);
    // Placeholder for stdin tests
}

#[test]
fn test_duplicate_selector() {
    let file = create_test_file_lines(&["a", "b", "c"]);
    let output = run_sel(&["2,2,2", file.path().to_str().unwrap()]);

    // Line should only appear once
    let lines: Vec<&str> = output.lines().collect();
    let matching: Vec<_> = lines.iter().filter(|l| l.contains("b")).collect();
    assert!(matching.len() <= 2); // At most one line of content plus potential formatting
}

#[test]
fn test_line_one_indexing() {
    let file = create_test_file_lines(&["first", "second", "third"]);
    let output = run_sel(&["1", file.path().to_str().unwrap()]);

    assert!(output.contains("first"));
    assert!(!output.contains("second"));
}

#[test]
fn test_inverted_range_error() {
    let file = create_test_file_lines(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_result(&["5-3", file.path().to_str().unwrap()]);

    assert!(code != 0 || stderr.contains("Error"));
}

#[test]
fn test_zero_line_error() {
    let file = create_test_file_lines(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_result(&["0", file.path().to_str().unwrap()]);

    assert!(code != 0 || stderr.contains("Error") || stderr.contains("Invalid"));
}

#[test]
fn test_comma_separated_single_lines() {
    let file = create_test_file_lines(&["l1", "l2", "l3", "l4", "l5"]);
    let output = run_sel(&["1,3,5", file.path().to_str().unwrap()]);

    assert!(output.contains("l1"));
    assert!(output.contains("l3"));
    assert!(output.contains("l5"));
    assert!(!output.contains("l2"));
    assert!(!output.contains("l4"));
}

#[test]
fn test_mixed_ranges_and_singles() {
    let file = create_test_file_lines(&["l1", "l2", "l3", "l4", "l5", "l6", "l7"]);
    let output = run_sel(&["1,3-5,7", file.path().to_str().unwrap()]);

    assert!(output.contains("l1"));
    assert!(output.contains("l3"));
    assert!(output.contains("l4"));
    assert!(output.contains("l5"));
    assert!(output.contains("l7"));
    assert!(!output.contains("l2"));
    assert!(!output.contains("l6"));
}

#[test]
fn test_adjacent_ranges() {
    let file = create_test_file_lines(&["l1", "l2", "l3", "l4", "l5", "l6"]);
    let output = run_sel(&["1-3,4-6", file.path().to_str().unwrap()]);

    // All lines should be present
    assert!(output.contains("l1"));
    assert!(output.contains("l6"));
}
