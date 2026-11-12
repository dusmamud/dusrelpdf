//! Regression tests for `dusrelpdf rotate --angle`.
//!
//! The angle accepts negative values, and both `--angle -90` (space form)
//! and `--angle=-90` (equals form) must work and normalise to /Rotate 270.
//! A non-multiple of 90 must fail cleanly with INVALID_ANGLE, exit 1 —
//! never a clap parse error (exit 2) or a panic.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_dusrelpdf"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("failed to run dusrelpdf binary")
}

fn mock_pdf(dir: &std::path::Path, pages: u32) -> PathBuf {
    let input = dir.join("in.pdf");
    let out = run(&[
        "mock",
        "--pages",
        &pages.to_string(),
        "-o",
        input.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "mock failed: {out:?}");
    input
}

fn rotate(input: &std::path::Path, dir: &std::path::Path, angle_args: &[&str]) -> PathBuf {
    let output = dir.join("out.pdf");
    let mut full: Vec<&str> = vec!["rotate", input.to_str().unwrap()];
    full.extend_from_slice(angle_args);
    full.extend_from_slice(&["-o", output.to_str().unwrap()]);
    let out = run(&full);
    assert!(
        out.status.success(),
        "rotate {angle_args:?} failed (exit {:?}): stdout={} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    output
}

#[test]
fn rotate_space_form_negative_angle_works() {
    let dir = std::env::temp_dir().join("dusrelpdf-cli-rotate-space");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let input = mock_pdf(&dir, 2);
    let out = rotate(&input, &dir, &["--angle", "-90"]);
    let bytes = fs::read(out).unwrap();
    assert!(
        bytes.windows(11).any(|w| w == b"/Rotate 270"),
        "expected /Rotate 270 in output for --angle -90"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rotate_equals_form_negative_angle_works() {
    let dir = std::env::temp_dir().join("dusrelpdf-cli-rotate-equals");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let input = mock_pdf(&dir, 2);
    let out = rotate(&input, &dir, &["--angle=-90"]);
    let bytes = fs::read(out).unwrap();
    assert!(
        bytes.windows(11).any(|w| w == b"/Rotate 270"),
        "expected /Rotate 270 in output for --angle=-90"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rotate_space_form_positive_angle_works() {
    let dir = std::env::temp_dir().join("dusrelpdf-cli-rotate-pos");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let input = mock_pdf(&dir, 2);
    let out = rotate(&input, &dir, &["--angle", "90"]);
    let bytes = fs::read(out).unwrap();
    assert!(
        bytes.windows(10).any(|w| w == b"/Rotate 90"),
        "expected /Rotate 90 in output for --angle 90"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rotate_non_multiple_of_90_fails_cleanly() {
    let dir = std::env::temp_dir().join("dusrelpdf-cli-rotate-bad");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let input = mock_pdf(&dir, 2);
    for angle in ["45", "-45"] {
        let output = dir.join("bad.pdf");
        let out = run(&[
            "rotate",
            input.to_str().unwrap(),
            "--angle",
            angle,
            "-o",
            output.to_str().unwrap(),
        ]);
        assert_eq!(
            out.status.code(),
            Some(1),
            "--angle {angle} must exit 1, got {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        );
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("INVALID_ANGLE"),
            "--angle {angle} must report INVALID_ANGLE: {}",
            String::from_utf8_lossy(&out.stderr),
        );
    }
    let _ = fs::remove_dir_all(&dir);
}
