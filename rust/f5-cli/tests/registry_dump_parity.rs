//! Differential parity tests for the `f5 registry-dump` verb.
//!
//! Runs the built `f5-query` binary and asserts stdout matches a golden
//! captured from `python -m tooling.f5.main registry-dump …`. Self-contained:
//! no Python at test time.
//!
//! Only the byte-parity sections (`profiles`, `objects`) are asserted against a
//! golden. The `commands` / `events` / `all` sections are deferred in the Rust
//! port (they embed the event-validity cross-product and hover prose
//! catalogue), so they are asserted to fail cleanly with exit code 2.

use std::path::PathBuf;
use std::process::Command;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn run_f5(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary")
}

fn assert_section_matches(section: &str, golden: &str) {
    let expected = std::fs::read(golden_dir().join(golden)).expect("read golden");
    let output = run_f5(&["registry-dump", "--section", section]);
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "registry-dump --section {section} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&expected),
        "registry-dump --section {section} stdout did not match golden {golden}"
    );
}

#[test]
fn profiles_section_matches_golden() {
    assert_section_matches("profiles", "registry_dump_profiles.golden");
}

#[test]
fn objects_section_matches_golden() {
    assert_section_matches("objects", "registry_dump_objects.golden");
}

#[test]
fn profiles_section_to_file_matches_golden() {
    // `--output FILE` writes the same canonical JSON plus a trailing newline,
    // exactly like the Python verb's `fh.write(text + "\n")`.
    let expected =
        std::fs::read(golden_dir().join("registry_dump_profiles.golden")).expect("read golden");
    let tmp = std::env::temp_dir().join(format!("registry_dump_{}.json", std::process::id()));
    let output = run_f5(&[
        "registry-dump",
        "--section",
        "profiles",
        "-o",
        tmp.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let written = std::fs::read(&tmp).expect("read written file");
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(
        String::from_utf8_lossy(&written),
        String::from_utf8_lossy(&expected),
    );
}

#[test]
fn deferred_sections_fail_cleanly() {
    for section in ["commands", "events", "all"] {
        let output = run_f5(&["registry-dump", "--section", section]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "registry-dump --section {section} should exit 2 (deferred)"
        );
        assert!(
            output.stdout.is_empty(),
            "deferred section {section} should emit no stdout"
        );
    }
}

#[test]
fn default_section_is_all_and_deferred() {
    // The default `--section all` contains the deferred commands/events
    // snapshots, so the bare verb is deferred too.
    let output = run_f5(&["registry-dump"]);
    assert_eq!(output.status.code(), Some(2));
}
