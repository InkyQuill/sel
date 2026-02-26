//! Integration tests for regex mode (-e flag).

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
#[allow(dead_code)]
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
fn test_simple_literal_match() {
    let file = create_test_file(&["error: something failed", "info: all good", "warning: check this"]);
    let output = run_sel(&["-e", "error", file.path().to_str().unwrap()]);

    assert!(output.contains("error: something failed"));
    assert!(!output.contains("info: all good"));
    assert!(!output.contains("warning: check this"));
}

#[test]
fn test_regex_match_multiple() {
    let file = create_test_file(&[
        "ERROR: first error",
        "INFO: normal message",
        "ERROR: second error",
        "DEBUG: debug message",
    ]);
    let output = run_sel(&["-e", "ERROR", file.path().to_str().unwrap()]);

    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 2);
    assert!(output.contains("ERROR: first error"));
    assert!(output.contains("ERROR: second error"));
    assert!(!output.contains("INFO: normal message"));
}

#[test]
fn test_regex_case_sensitive() {
    let file = create_test_file(&["Error: message", "error: another", "ERROR: third"]);
    let output = run_sel(&["-e", "Error", file.path().to_str().unwrap()]);

    // Should only match exact case "Error"
    assert!(output.contains("Error: message"));
    // Unless case insensitivity is added, these shouldn't match
    let count = output.matches("Error: message").count();
    assert_eq!(count, 1);
}

#[test]
fn test_regex_pattern_wildcard() {
    let file = create_test_file(&["user123 logged in", "admin456 logged in", "guest789 logged out"]);
    let output = run_sel(&["-e", r"user\d+", file.path().to_str().unwrap()]);

    assert!(output.contains("user123 logged in"));
    assert!(!output.contains("admin456"));
}

#[test]
fn test_regex_pattern_or() {
    let file = create_test_file(&["ERROR: fatal", "WARN: caution", "INFO: info", "CRITICAL: crash"]);
    let output = run_sel(&["-e", r"(ERROR|CRITICAL)", file.path().to_str().unwrap()]);

    assert!(output.contains("ERROR: fatal"));
    assert!(output.contains("CRITICAL: crash"));
    assert!(!output.contains("WARN: caution"));
}

#[test]
fn test_regex_pattern_start_of_line() {
    let file = create_test_file(&["test line", "another test", "test again"]);
    let output = run_sel(&["-e", r"^test", file.path().to_str().unwrap()]);

    assert!(output.contains("test line"));
    assert!(output.contains("test again"));
    assert!(!output.contains("another test"));
}

#[test]
fn test_regex_pattern_end_of_line() {
    let file = create_test_file(&["line end", "line middle", "end line"]);
    let output = run_sel(&["-e", r"end$", file.path().to_str().unwrap()]);

    assert!(output.contains("line end"));
    assert!(!output.contains("line middle"));
}

#[test]
fn test_regex_pattern_any_character() {
    let file = create_test_file(&["abc", "axc", "a_c", "ac"]);
    let output = run_sel(&["-e", r"a.c", file.path().to_str().unwrap()]);

    assert!(output.contains("abc"));
    assert!(output.contains("axc"));
    assert!(output.contains("a_c"));
    assert!(!output.contains("ac"));
}

#[test]
fn test_regex_pattern_escaped_chars() {
    let file = create_test_file(&["price: $100", "price: $200", "cost: 100"]);
    let output = run_sel(&["-e", r"price: \$\d+", file.path().to_str().unwrap()]);

    assert!(output.contains("price: $100"));
    assert!(output.contains("price: $200"));
    assert!(!output.contains("cost: 100"));
}

#[test]
fn test_regex_pattern_character_class() {
    let file = create_test_file(&["test123", "testABC", "test!@#", "test"]);
    let output = run_sel(&["-e", r"test[0-9]+", file.path().to_str().unwrap()]);

    assert!(output.contains("test123"));
    assert!(!output.contains("testABC"));
    assert!(!output.contains("test!@#"));
}

#[test]
fn test_regex_pattern_negated_class() {
    let file = create_test_file(&["a1b", "a2b", "axb", "a3b"]);
    let output = run_sel(&["-e", r"a[^0-9]b", file.path().to_str().unwrap()]);

    assert!(output.contains("axb"));
    assert!(!output.contains("a1b"));
    assert!(!output.contains("a2b"));
}

#[test]
fn test_regex_no_match() {
    let file = create_test_file(&["line1", "line2", "line3"]);
    let output = run_sel(&["-e", "nomatch", file.path().to_str().unwrap()]);

    assert!(output.is_empty());
}

#[test]
fn test_regex_empty_pattern() {
    let file = create_test_file(&["a", "b", "c"]);
    let output = run_sel(&["-e", "", file.path().to_str().unwrap()]);

    // Empty pattern matches everything
    assert!(output.contains("a"));
    assert!(output.contains("b"));
    assert!(output.contains("c"));
}

#[test]
fn test_regex_special_characters() {
    let file = create_test_file(&[
        r"function(arg1, arg2)",
        r"function()",
        r"function(x)",
    ]);
    let output = run_sel(&["-e", r"function\(.*\)", file.path().to_str().unwrap()]);

    assert!(output.contains(r"function(arg1, arg2)"));
    assert!(output.contains(r"function()"));
}

#[test]
fn test_regex_unicode_pattern() {
    let file = create_test_file(&["Hello 世界", "Hello World", "世界 Hello"]);
    let output = run_sel(&["-e", "世界", file.path().to_str().unwrap()]);

    assert!(output.contains("Hello 世界"));
    assert!(output.contains("世界 Hello"));
    assert!(!output.contains("Hello World"));
}

#[test]
fn test_regex_with_no_line_numbers() {
    let file = create_test_file(&["ERROR: bad", "INFO: good"]);
    let output = run_sel(&["-e", "ERROR", "-l", file.path().to_str().unwrap()]);

    assert!(output.contains("ERROR: bad"));
    assert!(!output.contains("1:ERROR"));
    assert!(!output.contains("INFO: good"));
}

#[test]
fn test_regex_line_numbers_visible() {
    let file = create_test_file(&["match this line", "don't match", "also match this"]);
    let output = run_sel(&["-e", "match", file.path().to_str().unwrap()]);

    // Line numbers should be visible by default
    assert!(output.contains("1:"));
    assert!(output.contains("3:"));
}

#[test]
fn test_regex_multiple_files() {
    let file1 = create_test_file(&["ERROR in file1", "INFO in file1"]);
    let file2 = create_test_file(&["ERROR in file2", "DEBUG in file2"]);

    let output = run_sel(&["-e", "ERROR", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Both files should show ERROR lines
    assert!(output.contains("ERROR in file1"));
    assert!(output.contains("ERROR in file2"));
    assert!(!output.contains("INFO"));
    assert!(!output.contains("DEBUG"));
}

#[test]
fn test_regex_anchor_both_ends() {
    let file = create_test_file(&["exact", "not exact match", "exact but more"]);
    let output = run_sel(&["-e", r"^exact$", file.path().to_str().unwrap()]);

    assert!(output.contains("exact"));
    assert!(!output.contains("not exact"));
    assert!(!output.contains("exact but more"));
}

#[test]
fn test_regex_repetition_operators() {
    let file = create_test_file(&["a", "aa", "aaa", "aaaa", "b"]);
    let output = run_sel(&["-e", r"a{3}", file.path().to_str().unwrap()]);

    // a{3} matches "aaa" and also matches within "aaaa"
    assert!(output.contains("aaa"));
    assert!(output.contains("aaaa"));
    // Note: "aaa" contains "aa" as substring, so we check line prefixes
    assert!(output.contains("3:aaa") || output.contains("4:aaaa"));
    assert!(!output.contains("1:a"));
    assert!(!output.contains("2:aa"));
    assert!(!output.contains("5:b"));
}

#[test]
fn test_regex_word_boundary() {
    let file = create_test_file(&["test", "testing", "pretest", "test case", "tested"]);
    let output = run_sel(&["-e", r"\btest\b", file.path().to_str().unwrap()]);

    assert!(output.contains("test"));
    assert!(output.contains("test case"));
    assert!(!output.contains("testing"));
    assert!(!output.contains("pretest"));
    assert!(!output.contains("tested"));
}

#[test]
fn test_regex_digit_class() {
    let file = create_test_file(&["123", "abc", "a1b2c3", "no digits"]);
    let output = run_sel(&["-e", r"\d+", file.path().to_str().unwrap()]);

    assert!(output.contains("123"));
    assert!(output.contains("a1b2c3"));
    assert!(!output.contains("abc"));
    assert!(!output.contains("no digits"));
}

#[test]
fn test_regex_whitespace_class() {
    let file = create_test_file(&["no_spaces", "has spaces", "tab\there", "new\nline"]);
    let output = run_sel(&["-e", r"\s", file.path().to_str().unwrap()]);

    // \s matches any whitespace (spaces, tabs, newlines)
    assert!(output.contains("has spaces"));
    assert!(output.contains("tab\there"));
    assert!(!output.contains("no_spaces"));
}

#[test]
fn test_regex_hex_color_pattern() {
    let file = create_test_file(&["color: #FF5733", "color: #00FF00", "color: invalid"]);
    let output = run_sel(&["-e", r"#[0-9A-Fa-f]{6}", file.path().to_str().unwrap()]);

    assert!(output.contains("#FF5733"));
    assert!(output.contains("#00FF00"));
    assert!(!output.contains("invalid"));
}

#[test]
fn test_regex_email_pattern() {
    let file = create_test_file(&[
        "Contact: user@example.com",
        "Contact: admin@test.org",
        "Contact: not-an-email",
        "Contact: another@domain.co.uk",
    ]);
    let output = run_sel(&["-e", r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", file.path().to_str().unwrap()]);

    assert!(output.contains("user@example.com"));
    assert!(output.contains("admin@test.org"));
    assert!(output.contains("another@domain.co.uk"));
    assert!(!output.contains("not-an-email"));
}

#[test]
fn test_regex_with_filename_flag() {
    let file = create_test_file(&["ERROR: test", "INFO: test"]);
    let output = run_sel(&["-H", "-e", "ERROR", file.path().to_str().unwrap()]);

    // With -H, filename should be shown even for single file
    let filename = file.path().file_name().unwrap().to_string_lossy();
    assert!(output.contains(filename.as_ref()));
}

#[test]
fn test_regex_long_line_match() {
    let long_line = "a".repeat(1000) + "TARGET" + &"b".repeat(1000);
    let file = create_test_file(&[&long_line]);
    let output = run_sel(&["-e", "TARGET", file.path().to_str().unwrap()]);

    assert!(output.contains("TARGET"));
}
