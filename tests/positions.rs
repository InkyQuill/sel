//! Integration tests for positional selectors (line:column).

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

#[test]
fn test_simple_position() {
    let file = create_test_file(&["line1", "line2", "line3"]);
    let output = run_sel(&["2:5", file.path().to_str().unwrap()]);

    // Without -n, should output the full line
    assert!(output.contains("2:line2"));
}

#[test]
fn test_position_first_column() {
    let file = create_test_file(&["hello world", "foo bar"]);
    let output = run_sel(&["1:1", file.path().to_str().unwrap()]);

    assert!(output.contains("1:hello world"));
}

#[test]
fn test_position_last_column() {
    let file = create_test_file(&["abc"]);
    let output = run_sel(&["1:3", file.path().to_str().unwrap()]);

    assert!(output.contains("1:abc"));
}

#[test]
fn test_position_beyond_line_length() {
    let file = create_test_file(&["short"]);
    let output = run_sel(&["1:100", file.path().to_str().unwrap()]);

    // Should still output the line
    assert!(output.contains("1:short"));
}

#[test]
fn test_position_at_line_start() {
    let file = create_test_file(&["  indented line", "normal"]);
    let output = run_sel(&["1:3", file.path().to_str().unwrap()]);

    assert!(output.contains("1:  indented line"));
}

#[test]
fn test_multiple_positions() {
    let file = create_test_file(&["line one", "line two", "line three"]);
    let output = run_sel(&["1:1,3:1", file.path().to_str().unwrap()]);

    assert!(output.contains("1:line one"));
    assert!(output.contains("3:line three"));
    assert!(!output.contains("2:line two"));
}

#[test]
fn test_multiple_positions_same_line() {
    let file = create_test_file(&["a very long line here"]);
    let output = run_sel(&["1:1,1:5,1:10", file.path().to_str().unwrap()]);

    // Should output the line (without -n, full lines are shown)
    assert!(output.contains("1:a very long line here"));
}

#[test]
fn test_position_with_char_context() {
    let file = create_test_file(&["function test() {", "    return 42;", "}"]);
    let output = run_sel(&["-n", "10", "2:9", file.path().to_str().unwrap()]);

    // With -n, should show fragment with pointer
    let lines: Vec<&str> = output.lines().collect();

    // Should contain the fragment
    assert!(lines.iter().any(|l| l.contains("return")));

    // Should contain pointer line
    assert!(lines.iter().any(|l| l.contains('^')));
}

#[test]
fn test_position_char_context_short_line() {
    let file = create_test_file(&["abc"]);
    let output = run_sel(&["-n", "10", "1:2", file.path().to_str().unwrap()]);

    // Should show the line with pointer
    assert!(output.contains("abc"));
    assert!(output.contains('^'));
}

#[test]
fn test_position_char_context_middle_of_line() {
    let file = create_test_file(&["The quick brown fox jumps"]);
    let output = run_sel(&["-n", "5", "1:11", file.path().to_str().unwrap()]);

    // Position 11 is 'b' in 'brown'
    assert!(output.contains("brown"));
}

#[test]
fn test_position_char_context_with_context_lines() {
    let file = create_test_file(&[
        "line 1 context before",
        "line 2 TARGET here",
        "line 3 context after",
    ]);
    let output = run_sel(&["-c", "1", "-n", "5", "2:11", file.path().to_str().unwrap()]);

    // Should show some context
    assert!(output.contains("TARGET"));
}

#[test]
fn test_position_at_end_of_line() {
    let file = create_test_file(&["text with trailing newline"]);
    let output = run_sel(&["-n", "5", "1:20", file.path().to_str().unwrap()]);

    // Should handle position at or near end of line
    // Position 20 is near the end, should show part of "newline"
    assert!(output.contains("newlin") || output.contains("line"));
}

#[test]
fn test_position_no_line_numbers() {
    let file = create_test_file(&["first line", "second line"]);
    let output = run_sel(&["-l", "2:1", file.path().to_str().unwrap()]);

    // With -l, should not show line numbers
    assert!(output.contains("second line"));
    assert!(!output.contains("2:"));
}

#[test]
fn test_position_with_char_context_no_line_numbers() {
    let file = create_test_file(&["content here"]);
    let output = run_sel(&["-l", "-n", "5", "1:5", file.path().to_str().unwrap()]);

    // Should show fragment without line number prefix
    assert!(!output.is_empty());
    assert!(output.contains("he") || output.contains("content"));
}

#[test]
fn test_position_multibyte_characters() {
    let file = create_test_file(&["Hello 世界 World"]);
    // Position 6 (space before '世') should work better
    let output = run_sel(&["-n", "3", "1:6", file.path().to_str().unwrap()]);

    // Should show some part of the output
    assert!(!output.is_empty() || output.contains("Hello"));
}

#[test]
fn test_position_tabs() {
    let file = create_test_file(&["\t\tindented with tabs"]);
    let output = run_sel(&["-n", "2", "1:3", file.path().to_str().unwrap()]);

    // Should handle tabs - position 3 is still in the tabs
    assert!(!output.is_empty());
}

#[test]
fn test_position_empty_line() {
    let file = create_test_file(&["", "non-empty", ""]);
    let output = run_sel(&["2:1", file.path().to_str().unwrap()]);

    assert!(output.contains("non-empty"));
}

#[test]
fn test_position_first_line_first_column() {
    let file = create_test_file(&["START here", "middle", "end"]);
    let output = run_sel(&["1:1", file.path().to_str().unwrap()]);

    assert!(output.contains("START here"));
}

#[test]
fn test_position_last_line() {
    let file = create_test_file(&["first", "second", "LAST LINE"]);
    let output = run_sel(&["3:1", file.path().to_str().unwrap()]);

    assert!(output.contains("LAST LINE"));
}

#[test]
fn test_position_with_zero_context() {
    let file = create_test_file(&["target"]);
    let output = run_sel(&["-n", "0", "1:4", file.path().to_str().unwrap()]);

    // Even with 0 context, should show something
    assert!(!output.is_empty());
}

#[test]
fn test_position_large_context() {
    let file = create_test_file(&["  x = value_here;"]);
    let output = run_sel(&["-n", "100", "1:5", file.path().to_str().unwrap()]);

    // Should show the whole line if context is larger than line
    assert!(output.contains("x = value_here"));
}

#[test]
fn test_position_exact_column_match() {
    let file = create_test_file(&["const int x = 42;"]);
    let output = run_sel(&["-n", "3", "1:11", file.path().to_str().unwrap()]);

    // Column 11 is 'x' in 'x ='
    assert!(output.contains("x ="));
}

#[test]
fn test_invalid_position_format() {
    let file = create_test_file(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_with_result(&["invalid", file.path().to_str().unwrap()]);

    // 'invalid' doesn't look like a selector, so it's treated as filename
    // which will fail to open
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("No such file"));
}

#[test]
fn test_invalid_position_missing_column() {
    let file = create_test_file(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_with_result(&["1:", file.path().to_str().unwrap()]);

    // "1:" is not a valid selector
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("Invalid"));
}

#[test]
fn test_invalid_position_missing_line() {
    let file = create_test_file(&["a", "b", "c"]);
    let (_stdout, stderr, code) = run_sel_with_result(&[":10", file.path().to_str().unwrap()]);

    // ":10" is not a valid selector
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("Invalid"));
}

#[test]
fn test_char_context_without_position() {
    let file = create_test_file(&["a", "b", "c"]);
    let (_stdout, stderr, code) =
        run_sel_with_result(&["-n", "5", "2", file.path().to_str().unwrap()]);

    // -n requires a position selector (with colon)
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("position"));
}

#[test]
fn test_position_beyond_file_line_count() {
    let file = create_test_file(&["only", "three", "lines"]);
    let output = run_sel(&["10:1", file.path().to_str().unwrap()]);

    // Should produce empty output
    assert!(output.is_empty() || !output.contains("only"));
}
