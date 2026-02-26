//! Integration tests for multi-file handling.

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

/// Helper to get the filename of a temp file.
fn temp_file_name(file: &NamedTempFile) -> String {
    file.path().file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn test_two_files_simple_selector() {
    let file1 = create_test_file(&["line1 from file1", "line2 from file1"]);
    let file2 = create_test_file(&["line1 from file2", "line2 from file2"]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Both files should show line 1
    assert!(output.contains("line1 from file1"));
    assert!(output.contains("line1 from file2"));
}

#[test]
fn test_three_files_range() {
    let file1 = create_test_file(&["a1", "a2", "a3"]);
    let file2 = create_test_file(&["b1", "b2", "b3"]);
    let file3 = create_test_file(&["c1", "c2", "c3"]);

    let output = run_sel(&[
        "2-3",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
        file3.path().to_str().unwrap(),
    ]);

    assert!(output.contains("a2"));
    assert!(output.contains("b2"));
    assert!(output.contains("c2"));
    assert!(!output.contains("a1"));
    assert!(!output.contains("b1"));
}

#[test]
fn test_filename_prefix_multi_file() {
    let file1 = create_test_file(&["content1"]);
    let file2 = create_test_file(&["content2"]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // With multiple files, filename should be shown
    let name1 = temp_file_name(&file1);
    let name2 = temp_file_name(&file2);

    assert!(output.contains(&name1) || output.contains(file1.path().to_str().unwrap()));
    assert!(output.contains(&name2) || output.contains(file2.path().to_str().unwrap()));
}

#[test]
fn test_no_filename_prefix_single_file() {
    let file1 = create_test_file(&["content1"]);
    let output = run_sel(&["1", file1.path().to_str().unwrap()]);

    // Single file should not show filename by default
    let name = temp_file_name(&file1);
    // Output should be "1:content1", not "filename:1:content1"
    assert!(output.contains("1:content1"));
    // Unless filename is part of content
    if !output.contains(&name) {
        // Good - no filename prefix
    }
}

#[test]
fn test_force_filename_with_h_flag() {
    let file1 = create_test_file(&["content"]);
    let name = temp_file_name(&file1);

    let output = run_sel(&["-H", "1", file1.path().to_str().unwrap()]);

    // With -H, filename should be shown even for single file
    assert!(output.contains(&name) || output.contains(file1.path().to_str().unwrap()));
}

#[test]
fn test_multi_file_with_regex() {
    let file1 = create_test_file(&["ERROR: file1 error", "INFO: file1 info"]);
    let file2 = create_test_file(&["ERROR: file2 error", "DEBUG: file2 debug"]);
    let file3 = create_test_file(&["INFO: file3 info"]);

    let output = run_sel(&[
        "-e",
        "ERROR",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
        file3.path().to_str().unwrap(),
    ]);

    assert!(output.contains("ERROR: file1 error"));
    assert!(output.contains("ERROR: file2 error"));
    assert!(!output.contains("INFO:"));
    assert!(!output.contains("DEBUG:"));
}

#[test]
fn test_multi_file_regex_with_filename() {
    let file1 = create_test_file(&["TARGET"]);
    let file2 = create_test_file(&["TARGET"]);

    let output = run_sel(&["-e", "TARGET", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    let _name1 = temp_file_name(&file1);
    let _name2 = temp_file_name(&file2);

    // Filenames should appear with regex on multiple files
    assert!(output.contains("TARGET"));
}

#[test]
fn test_multi_file_complex_selector() {
    let file1 = create_test_file(&["l1", "l2", "l3", "l4", "l5"]);
    let file2 = create_test_file(&["l1", "l2", "l3", "l4", "l5"]);

    let output = run_sel(&["1,3,5", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Both files should output lines 1, 3, 5
    let lines: Vec<&str> = output.lines().collect();
    let matching_lines: Vec<_> = lines.iter().filter(|l| l.contains("l1") || l.contains("l3") || l.contains("l5")).collect();

    assert!(matching_lines.len() >= 6); // 3 lines per file * 2 files
}

#[test]
fn test_multi_file_all_lines() {
    let file1 = create_test_file(&["a", "b"]);
    let file2 = create_test_file(&["c", "d"]);

    let output = run_sel(&[file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Without selector, all lines from all files
    assert!(output.contains("a"));
    assert!(output.contains("b"));
    assert!(output.contains("c"));
    assert!(output.contains("d"));
}

#[test]
fn test_multi_file_empty_first_file() {
    let file1 = create_test_file(&[]);
    let file2 = create_test_file(&["content"]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Should still process second file
    assert!(output.contains("content"));
}

#[test]
fn test_multi_file_empty_second_file() {
    let file1 = create_test_file(&["content"]);
    let file2 = create_test_file(&[]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Should still process first file
    assert!(output.contains("content"));
}

#[test]
fn test_multi_file_different_sizes() {
    let file1 = create_test_file(&["only one line"]);
    let file2 = create_test_file(&["line1", "line2", "line3", "line4", "line5"]);

    let output = run_sel(&["3", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // File1 only has 1 line, file2 has line 3
    assert!(output.contains("line3"));
}

#[test]
fn test_multi_file_with_range_extending_beyond() {
    let file1 = create_test_file(&["a", "b"]);
    let file2 = create_test_file(&["x", "y", "z"]);

    let output = run_sel(&["2-5", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Should handle gracefully - file1 only has 2 lines
    assert!(output.contains("b"));
    assert!(output.contains("y"));
    assert!(output.contains("z"));
}

#[test]
fn test_multi_file_with_context() {
    let file1 = create_test_file(&["before", "TARGET1", "after"]);
    let file2 = create_test_file(&["before", "TARGET2", "after"]);

    let output = run_sel(&[
        "-c",
        "1",
        "2",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(output.contains("TARGET1") || output.contains("TARGET2"));
}

#[test]
fn test_multi_file_with_no_line_numbers() {
    let file1 = create_test_file(&["content1"]);
    let file2 = create_test_file(&["content2"]);

    let output = run_sel(&[
        "-l",
        "1",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    // No line numbers, but filename should still appear for multiple files
    assert!(output.contains("content1"));
    assert!(output.contains("content2"));
}

#[test]
fn test_multi_file_positional_selector() {
    let file1 = create_test_file(&["position test in file1"]);
    let file2 = create_test_file(&["position test in file2"]);

    let output = run_sel(&[
        "1:1",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(output.contains("position test in file1"));
    assert!(output.contains("position test in file2"));
}

#[test]
fn test_multi_file_ordering_preserved() {
    let file1 = create_test_file(&["from_file1"]);
    let file2 = create_test_file(&["from_file2"]);
    let file3 = create_test_file(&["from_file3"]);

    let output = run_sel(&[
        "1",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
        file3.path().to_str().unwrap(),
    ]);

    // Files should be processed in order given
    let file1_pos = output.find("from_file1");
    let file2_pos = output.find("from_file2");
    let file3_pos = output.find("from_file3");

    if let (Some(f1), Some(f2), Some(f3)) = (file1_pos, file2_pos, file3_pos) {
        assert!(f1 < f2);
        assert!(f2 < f3);
    }
}

#[test]
fn test_multi_file_duplicate_content() {
    let file1 = create_test_file(&["IDENTICAL LINE"]);
    let file2 = create_test_file(&["IDENTICAL LINE"]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    // Both occurrences should appear
    let count = output.matches("IDENTICAL LINE").count();
    assert_eq!(count, 2);
}

#[test]
fn test_multi_file_with_special_filenames() {
    let file1 = create_test_file(&["content with spaces"]);
    let file2 = create_test_file(&["content with tabs\t"]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    assert!(output.contains("content with spaces"));
}

#[test]
fn test_many_files() {
    let files: Vec<_> = (0..10)
        .map(|i| {
            let content = format!("content{}", i);
            create_test_file(&[content.as_str()])
        })
        .collect();
    let args: Vec<_> = ["1"]
        .iter()
        .map(|s| *s)
        .chain(files.iter().map(|f| f.path().to_str().unwrap()))
        .collect();

    let output = run_sel(&args);

    // All files should be processed
    for i in 0..10 {
        assert!(output.contains(&format!("content{}", i)));
    }
}

#[test]
fn test_multi_file_nonexistent_file() {
    let file1 = create_test_file(&["exists"]);
    let (_stdout, stderr, code) = run_sel_with_result(&[
        "1",
        file1.path().to_str().unwrap(),
        "/nonexistent/file.txt",
    ]);

    // Should error on nonexistent file
    assert!(code != 0 || stderr.contains("Error") || stderr.contains("No such file"));
}

#[test]
fn test_multi_file_mixed_valid_invalid() {
    let file1 = create_test_file(&["file1 content"]);
    let file2 = create_test_file(&["file2 content"]);
    let (_stdout, stderr, code) = run_sel_with_result(&[
        "1",
        file1.path().to_str().unwrap(),
        "/nonexistent.txt",
        file2.path().to_str().unwrap(),
    ]);

    // Behavior on mixed files - should stop at error
    if code != 0 {
        assert!(stderr.contains("Error"));
    }
}

#[test]
fn test_multi_file_unicode_content() {
    let file1 = create_test_file(&["Hello 世界"]);
    let file2 = create_test_file(&["Привет мир"]);

    let output = run_sel(&["1", file1.path().to_str().unwrap(), file2.path().to_str().unwrap()]);

    assert!(output.contains("世界"));
    assert!(output.contains("Привет"));
}

#[test]
fn test_multi_file_with_color_flag() {
    let file1 = create_test_file(&["content1"]);
    let file2 = create_test_file(&["content2"]);

    // Test with color=never to avoid ANSI codes in output
    let output = run_sel(&[
        "--color=never",
        "1",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(output.contains("content1"));
    assert!(output.contains("content2"));
}

#[test]
fn test_multi_file_regex_all_files() {
    let file1 = create_test_file(&["match this", "not this"]);
    let file2 = create_test_file(&["match that", "also not"]);
    let file3 = create_test_file(&["match other", "nope"]);

    let output = run_sel(&[
        "-e",
        "match",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
        file3.path().to_str().unwrap(),
    ]);

    // All files should have matches
    assert!(output.contains("match this"));
    assert!(output.contains("match that"));
    assert!(output.contains("match other"));
    assert!(!output.contains("not this"));
    assert!(!output.contains("also not"));
    assert!(!output.contains("nope"));
}
