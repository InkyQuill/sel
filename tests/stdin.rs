use std::io::Write;
use std::process::{Command, Stdio};

fn sel_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sel"))
}

fn run_with_stdin(args: &[&str], stdin: &str) -> (String, String, i32) {
    let mut child = sel_bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn no_args_reads_stdin_as_cat_n() {
    let (stdout, _, code) = run_with_stdin(&[], "alpha\nbeta\ngamma\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "   1: alpha\n   2: beta\n   3: gamma\n");
}

#[test]
fn dash_is_stdin() {
    let (stdout, _, code) = run_with_stdin(&["2", "-"], "one\ntwo\nthree\n");
    assert_eq!(code, 0);
    assert_eq!(stdout, "   2: two\n");
}

#[test]
fn positional_selector_with_stdin_errors() {
    let (_, stderr, code) = run_with_stdin(&["1:5"], "hello world\n");
    assert_ne!(code, 0);
    assert!(stderr.contains("stdin"));
}
