use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

fn sel_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sel"))
}

#[test]
fn invert_emits_non_matching_lines() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "keep").unwrap();
    writeln!(f, "drop ERROR").unwrap();
    writeln!(f, "keep too").unwrap();

    let out = sel_bin()
        .args(["-v", "-e", "ERROR", f.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("keep"));
    assert!(stdout.contains("keep too"));
    assert!(!stdout.contains("drop ERROR"));
}

#[test]
fn invert_without_regex_is_error() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "whatever").unwrap();
    let out = sel_bin()
        .args(["-v", f.path().to_str().unwrap()])
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--invert-match requires --regex"));
}
