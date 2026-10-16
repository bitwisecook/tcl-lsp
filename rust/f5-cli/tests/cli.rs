// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Differential tests for the native `f5-query` CLI.
//!
//! Runs the built binary on committed fixtures and asserts stdout matches the
//! captured golden output. Only verbs whose engine is implemented
//! (currently file-I/O only) are asserted byte-for-byte.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run f5-query, returning stdout. `diff` exits 1 when there are changes, so
/// callers pass the set of acceptable exit codes.
fn run_f5_codes(args: &[&str], ok_codes: &[i32]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        ok_codes.contains(&code),
        "f5-query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn run_f5(args: &[&str]) -> Vec<u8> {
    run_f5_codes(args, &[0])
}

fn assert_diff_matches(args: &[&str], golden: &str) {
    let expected = std::fs::read(fixtures_dir().join(golden)).expect("read golden");
    let actual = run_f5_codes(args, &[0, 1]);
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "f5 diff output does not match the golden ({golden})"
    );
}

#[test]
fn merge() {
    let dir = fixtures_dir();
    let a = dir.join("part-a.conf");
    let b = dir.join("part-b.conf");
    let expected = std::fs::read(dir.join("merge-ab.golden")).expect("read golden");
    let actual = run_f5(&["merge", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "f5 merge output does not match the golden"
    );
}

#[cfg(unix)]
#[test]
fn merge_refuses_opaque_path_without_opening_replacement_sibling() {
    use std::os::unix::ffi::OsStringExt;

    let dir = std::env::temp_dir().join(format!("f5-merge-native-path-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let opaque_name = std::ffi::OsString::from_vec(b"input-\xff.conf".to_vec());
    let opaque_path = dir.join(opaque_name);
    let replacement_path = dir.join("input-\u{fffd}.conf");
    let marker = "# Unicode replacement sibling sentinel\n";
    std::fs::write(&opaque_path, "# opaque original file\n").expect("write opaque input");
    std::fs::write(&replacement_path, marker).expect("write replacement sibling");

    // This is the concrete alias that the former lossy conversion created.
    assert_eq!(
        PathBuf::from(opaque_path.to_string_lossy().as_ref()),
        replacement_path
    );
    assert!(replacement_path.is_file());

    let refused = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("merge")
        .arg(&opaque_path)
        .output()
        .expect("run merge with opaque path");
    assert_eq!(refused.status.code(), Some(2));
    assert!(
        refused.stdout.is_empty(),
        "refusal must not emit merged data"
    );
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("input path is not representable as Unicode"),
        "unexpected diagnostic: {}",
        String::from_utf8_lossy(&refused.stderr)
    );

    let accepted = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("merge")
        .arg(&replacement_path)
        .output()
        .expect("run merge with Unicode replacement sibling");
    assert_eq!(accepted.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&accepted.stdout).contains(marker));
    assert!(accepted.stderr.is_empty());

    std::fs::remove_dir_all(&dir).expect("remove temp dir");
}

#[test]
fn split_then_merge() {
    // split a multi-partition fixture into a temp dir, then merge it back, and
    // assert the round-trip equals the captured golden output.
    let dir = fixtures_dir();
    let input = dir.join("split-multi.conf");
    let expected = std::fs::read(dir.join("split-multi.roundtrip.golden")).expect("read golden");

    let out_dir = std::env::temp_dir().join(format!("f5-split-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    run_f5(&["split", input.to_str().unwrap(), out_dir.to_str().unwrap()]);
    let actual = run_f5(&["merge", out_dir.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&out_dir);

    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "f5 split→merge round-trip does not match the golden"
    );
}

#[test]
fn diff_add_remove_text() {
    let dir = fixtures_dir();
    let a = dir.join("part-a.conf");
    let b = dir.join("part-b.conf");
    assert_diff_matches(
        &["diff", a.to_str().unwrap(), b.to_str().unwrap()],
        "diff-addrm.text.golden",
    );
}

#[test]
fn diff_add_remove_json() {
    let dir = fixtures_dir();
    let a = dir.join("part-a.conf");
    let b = dir.join("part-b.conf");
    assert_diff_matches(
        &["diff", a.to_str().unwrap(), b.to_str().unwrap(), "--json"],
        "diff-addrm.json.golden",
    );
}

#[test]
fn diff_tmsh_input() {
    // tmsh-script input is normalised to SCF before parsing (the `to_scf` path),
    // so `tmsh create/modify` headers compare against the real objects.
    let dir = fixtures_dir();
    let a = dir.join("diff-a.tmsh");
    let b = dir.join("diff-b.tmsh");
    assert_diff_matches(
        &["diff", a.to_str().unwrap(), b.to_str().unwrap()],
        "diff-tmsh.text.golden",
    );
}

#[test]
fn explain_virtual_text() {
    let conf = fixtures_dir().join("bigip.conf");
    assert_diff_matches(
        &[
            "explain",
            "virtual",
            "/Common/www_vs",
            conf.to_str().unwrap(),
        ],
        "explain-virtual.text.golden",
    );
}

#[test]
fn explain_virtual_json() {
    let conf = fixtures_dir().join("bigip.conf");
    assert_diff_matches(
        &[
            "explain",
            "virtual",
            "/Common/www_vs",
            conf.to_str().unwrap(),
            "--json",
        ],
        "explain-virtual.json.golden",
    );
}

/// Assert that `f5 extract`'s stdout matches the captured golden output,
/// optionally with `F5_UCS_PASSPHRASE` set for an encrypted archive.
fn assert_extract_matches(args: &[&str], passphrase: Option<&str>, golden: &str) {
    let expected = std::fs::read(fixtures_dir().join(golden)).expect("read golden");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_f5-query"));
    cmd.args(args);
    if let Some(p) = passphrase {
        cmd.env("F5_UCS_PASSPHRASE", p);
    }
    let output = cmd.output().expect("failed to spawn f5-query binary");
    assert!(
        output.status.success(),
        "f5-query {args:?} failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&expected),
        "f5 extract output does not match the golden ({golden})"
    );
}

#[test]
fn extract_plain_ucs() {
    let ucs = fixtures_dir().join("sample.ucs");
    assert_extract_matches(
        &["extract", ucs.to_str().unwrap()],
        None,
        "extract-default.scf.golden",
    );
}

#[test]
fn extract_plain_ucs_include_extras() {
    let ucs = fixtures_dir().join("sample.ucs");
    assert_extract_matches(
        &["extract", "--include-extras", ucs.to_str().unwrap()],
        None,
        "extract-extras.scf.golden",
    );
}

#[test]
fn extract_encrypted_ucs() {
    // OpenPGP-symmetric (gpg --symmetric) archive decrypted purely in Rust;
    // passphrase supplied via the F5_UCS_PASSPHRASE environment variable.
    let ucs = fixtures_dir().join("sample-encrypted.ucs");
    assert_extract_matches(
        &["extract", ucs.to_str().unwrap()],
        Some("correct horse battery staple"),
        "extract-encrypted.scf.golden",
    );
}

#[test]
fn explain_reads_ucs() {
    // The UCS input layer is wired into the model-reading verbs: `f5 explain`
    // accepts a `.ucs` (transparently extracted to SCF) exactly like a `.conf`.
    let ucs = fixtures_dir().join("sample.ucs");
    assert_diff_matches(
        &["explain", "auto", "/Common/vs_web", ucs.to_str().unwrap()],
        "explain-ucs.text.golden",
    );
}

#[test]
fn diff_reads_ucs() {
    // `f5 diff` accepts `.ucs` inputs on either side; diffing an archive
    // against itself yields no changes, byte-identical to the captured golden output.
    let ucs = fixtures_dir().join("sample.ucs");
    assert_diff_matches(
        &["diff", ucs.to_str().unwrap(), ucs.to_str().unwrap()],
        "diff-ucs-self.text.golden",
    );
}

#[test]
fn diff_scalar_modify() {
    // Scalar-field modification (load-balancing-mode) — object-list fields
    // (members/records) are excluded here since their display diverges.
    let dir = fixtures_dir();
    let before = dir.join("diff-mod-before.conf");
    let after = dir.join("diff-mod-after.conf");
    assert_diff_matches(
        &["diff", before.to_str().unwrap(), after.to_str().unwrap()],
        "diff-mod.text.golden",
    );
}

#[test]
fn source_transforms_refuse_invalid_name_bytes_before_artifacts() {
    let dir = std::env::temp_dir().join(format!("f5-source-fidelity-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.conf");
    let output = dir.join("output.conf");
    let split = dir.join("split");
    let original = b"ltm pool /Common/old { description \"\xff\" }\n";
    std::fs::write(&input, original).unwrap();
    let input_arg = input.to_str().unwrap();
    let output_arg = output.to_str().unwrap();
    let expression = r#"rename("/Common/old", "/Common/new")"#;
    for args in [
        vec!["rename", "/Common/old", "/Common/new", input_arg],
        vec!["rename", "/Common/old", "/Common/new", input_arg, "--write"],
        vec![
            "rename",
            "/Common/old",
            "/Common/new",
            input_arg,
            "--output",
            output_arg,
        ],
        vec![
            "rename",
            "/Common/old",
            "/Common/new",
            input_arg,
            "--in-place",
        ],
        vec!["query", expression, input_arg],
        vec!["query", "--write", expression, input_arg],
        vec!["query", "--in-place", expression, input_arg],
        vec!["split", input_arg, split.to_str().unwrap()],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_f5-query"))
            .args(&args)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("invalid UTF-8"),
            "{args:?}: {:?}",
            result.stderr
        );
        assert_eq!(std::fs::read(&input).unwrap(), original);
        assert!(!output.exists());
        assert!(!split.exists());
    }
    let report = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(["query", ".ltm.pool[].name", input_arg])
        .output()
        .unwrap();
    assert!(report.status.success(), "{:?}", report.stderr);
    assert!(
        String::from_utf8(report.stdout)
            .unwrap()
            .contains("/Common/old")
    );
    std::fs::write(
        &input,
        "ltm pool /Common/old { description \"\u{fffd}\" }\n",
    )
    .unwrap();
    let literal = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(["rename", "/Common/old", "/Common/new", input_arg, "--write"])
        .output()
        .unwrap();
    assert!(literal.status.success(), "{:?}", literal.stderr);
    let text = String::from_utf8(literal.stdout).unwrap();
    assert!(text.contains("/Common/new"));
    assert!(text.contains('\u{fffd}'));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mutating_queries_require_faithful_named_and_structured_donors() {
    let dir = std::env::temp_dir().join(format!("f5-query-donor-fidelity-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("target.conf");
    let donor = dir.join("donor.conf");
    let json = dir.join("donor.json");
    let original = "ltm pool /Common/old { }\n";
    std::fs::write(&target, original).unwrap();
    std::fs::write(&donor, b"ltm pool /Common/\xff { }\n").unwrap();
    std::fs::write(&json, b"{\"name\":\"/Common/\xff\"}").unwrap();
    for (flag, binding, expression, extra) in [
        (
            "--name",
            format!("donor={}", donor.display()),
            r#"rename("/Common/old", $donor.ltm.pool[].name)"#,
            Some(donor.as_path()),
        ),
        (
            "--input-json",
            format!("donor={}", json.display()),
            r#"rename("/Common/old", $donor.name)"#,
            None,
        ),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_f5-query"));
        command
            .args(["query", "--write", flag, &binding, expression])
            .arg(&target);
        if let Some(extra) = extra {
            command.arg(extra);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("invalid UTF-8"),
            "{:?}",
            output.stderr
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), original);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn query_archive_file_donors_keep_original_bytes_before_mutation() {
    let dir = std::env::temp_dir().join(format!("f5-query-file-donor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.ucs");
    for (donor, faithful) in [
        (b"new\xff".as_slice(), false),
        ("new\u{fffd}".as_bytes(), true),
    ] {
        let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(gzip);
        for (name, bytes) in [
            (
                "config/bigip.conf",
                b"ltm pool /Common/old { description old }\n".as_slice(),
            ),
            ("etc/donor", donor),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(u64::try_from(bytes.len()).unwrap());
            header.set_mode(0o644);
            header.set_cksum();
            archive.append_data(&mut header, name, bytes).unwrap();
        }
        std::fs::write(&path, archive.into_inner().unwrap().finish().unwrap()).unwrap();
        let preview = Command::new(env!("CARGO_BIN_EXE_f5-query"))
            .args(["query", r#"file("etc/donor") | .content"#])
            .arg(&path)
            .output()
            .unwrap();
        assert!(preview.status.success(), "{:?}", preview.stderr);
        assert!(
            String::from_utf8(preview.stdout)
                .unwrap()
                .contains('\u{fffd}')
        );
        let result = Command::new(env!("CARGO_BIN_EXE_f5-query"))
            .args([
                "query",
                "--write",
                r#".ltm.pool[] | .description = (file("etc/donor") | .content)"#,
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(result.status.success(), faithful, "{:?}", result.stderr);
        if faithful {
            assert!(
                String::from_utf8(result.stdout)
                    .unwrap()
                    .contains('\u{fffd}')
            );
        } else {
            assert!(result.stdout.is_empty());
            assert!(String::from_utf8_lossy(&result.stderr).contains("invalid UTF-8"));
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}
