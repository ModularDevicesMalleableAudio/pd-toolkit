use assert_cmd::Command;
use std::path::{Path, PathBuf};

pub fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

pub fn handcrafted(name: &str) -> PathBuf {
    fixture_path(&format!("handcrafted/{name}"))
}

pub fn corpus_dir() -> PathBuf {
    fixture_path("corpus")
}

pub fn run_pdtk(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("pdtk")
        .expect("pdtk binary should build")
        .args(args)
        .output()
        .expect("pdtk command should run")
}

pub fn run_pdtk_with_path(args: &[&str], path_arg: &Path) -> std::process::Output {
    Command::cargo_bin("pdtk")
        .expect("pdtk binary should build")
        .args(args)
        .arg(path_arg)
        .output()
        .expect("pdtk command should run")
}

/// Run pdtk, assert success, return stdout as String.
pub fn pdtk_output(args: &[&str]) -> String {
    let out = run_pdtk(args);
    assert!(
        out.status.success(),
        "pdtk {:?} failed (exit {:?}):\nstderr: {}",
        args,
        out.status.code(),
        stderr_string(&out)
    );
    stdout_string(&out)
}

pub fn stdout_string(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn stderr_string(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}
