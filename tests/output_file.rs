use std::process::{Command, Stdio};
use tempfile::tempdir;

fn sel_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sel"))
}

#[test]
fn writes_to_output_file() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.txt");
    let output = dir.path().join("out.txt");
    std::fs::write(&input, "a\nb\nc\n").unwrap();

    let status = sel_bin()
        .args(["2", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "2:b\n");
}

#[test]
fn refuses_to_overwrite_by_default() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.txt");
    let output = dir.path().join("out.txt");
    std::fs::write(&input, "x\n").unwrap();
    std::fs::write(&output, "do not clobber").unwrap();

    let out = sel_bin()
        .args(["1", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let msg = String::from_utf8(out.stderr).unwrap();
    assert!(msg.contains("already exists"));
    assert!(msg.contains("--force"));
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "do not clobber");
}

#[test]
fn force_overwrites() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.txt");
    let output = dir.path().join("out.txt");
    std::fs::write(&input, "new\n").unwrap();
    std::fs::write(&output, "old").unwrap();

    let status = sel_bin()
        .args([
            "1",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--force",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "1:new\n");
}

#[test]
fn dash_output_is_stdout() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.txt");
    std::fs::write(&input, "stdout\n").unwrap();

    let out = sel_bin()
        .args(["1", input.to_str().unwrap(), "-o", "-"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "1:stdout\n");
}
