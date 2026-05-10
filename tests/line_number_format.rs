use std::process::Command;
use tempfile::tempdir;

fn sel_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sel"))
}

fn run_sel(args: &[&str]) -> String {
    let output = sel_bin().args(args).output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn normal_output_uses_minimum_width_four_with_separator_space() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.txt");
    std::fs::write(&input, "alpha\nbeta\n").unwrap();

    let output = run_sel(&["2", input.to_str().unwrap()]);

    assert_eq!(output, "   2: beta\n");
}

#[test]
fn context_output_keeps_marker_outside_padded_number_field() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.txt");
    std::fs::write(&input, "alpha\nbeta\ngamma\n").unwrap();

    let output = run_sel(&["-c", "1", "2", input.to_str().unwrap()]);

    assert_eq!(output, "   1: alpha\n>    2: beta\n   3: gamma\n");
}

#[test]
fn regex_output_uses_padded_prefix() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.txt");
    std::fs::write(&input, "INFO ok\nERROR bad\n").unwrap();

    let output = run_sel(&["-e", "ERROR", input.to_str().unwrap()]);

    assert_eq!(output, "   2: ERROR bad\n");
}

#[test]
fn fragment_output_aligns_caret_after_padded_prefix() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.txt");
    std::fs::write(&input, "abcdef\n").unwrap();

    let output = run_sel(&["-n", "2", "1:5", input.to_str().unwrap()]);

    assert_eq!(output, "   1: cdef\n        ^\n");
}

#[test]
fn selector_estimates_width_from_largest_selected_line() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.txt");
    let mut contents = "\n".repeat(9999);
    contents.push_str("target\n");
    std::fs::write(&input, contents).unwrap();

    let output = run_sel(&["10000", input.to_str().unwrap()]);

    assert_eq!(output, "10000: target\n");
}

#[test]
fn whole_file_output_grows_width_while_streaming() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.txt");
    let contents = (1..=10000)
        .map(|line| format!("line{line}\n"))
        .collect::<String>();
    std::fs::write(&input, contents).unwrap();

    let output = run_sel(&[input.to_str().unwrap()]);

    assert!(output.contains("   1: line1\n"));
    assert!(output.contains("9999: line9999\n"));
    assert!(output.contains("10000: line10000\n"));
}
