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

//! Differential tests for UCS handling against the captured reference.
//!
//! Fixtures are captured once (a plain UCS, an encrypted UCS produced by
//! `gpg --symmetric`, and the golden SCF text from `ucs_to_scf`). The tests
//! themselves are self-contained — no external reference, no gpg —
//! so they run under plain `cargo test`.

use std::path::PathBuf;

use tcl_bigip_io::paths::PassphraseOptions;
use tcl_bigip_io::ucs::{is_pgp_bytes, is_ucs_bytes, ucs_archive_to_scf, ucs_to_scf};
use tcl_bigip_io::{decrypt_symmetric, read_path};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name)).expect("read fixture")
}

fn read_text(name: &str) -> String {
    String::from_utf8(read(name)).expect("utf-8 fixture")
}

fn passphrase() -> String {
    read_text("sample.passphrase")
}

fn no_passphrase() -> PassphraseOptions {
    PassphraseOptions {
        explicit: None,
        env_var: String::new(),
        allow_prompt: false,
        ..PassphraseOptions::default()
    }
}

fn with_passphrase() -> PassphraseOptions {
    PassphraseOptions {
        explicit: Some(passphrase()),
        ..PassphraseOptions::default()
    }
}

#[test]
fn detects_plain_and_encrypted_magic() {
    let plain = read("sample.ucs");
    let enc = read("sample-encrypted.ucs");
    assert!(is_ucs_bytes(&plain) && !is_pgp_bytes(&plain));
    assert!(is_pgp_bytes(&enc) && !is_ucs_bytes(&enc));
    // F5 encrypted UCS starts with a SKESK packet (tag 3 → 0x8C).
    assert_eq!(enc[0], 0x8C);
}

#[test]
fn plain_ucs_to_scf() {
    let plain = read("sample.ucs");
    assert_eq!(
        ucs_to_scf(&plain, false).unwrap(),
        read_text("sample.scf.golden")
    );
    assert_eq!(
        ucs_to_scf(&plain, true).unwrap(),
        read_text("sample.extras.scf.golden")
    );
}

#[test]
fn pure_rust_decrypt_round_trips_gpg_output() {
    // The Rust OpenPGP decryptor reproduces the gpg-produced ciphertext's
    // plaintext exactly — same bytes as the committed plain UCS.
    let enc = read("sample-encrypted.ucs");
    let plain = read("sample.ucs");
    let got = decrypt_symmetric(&enc, passphrase().as_bytes()).expect("decrypt");
    assert_eq!(got, plain, "decrypted bytes differ from the plain UCS");
}

#[test]
fn encrypted_ucs_to_scf() {
    let enc = read("sample-encrypted.ucs");
    let opts = with_passphrase();
    let provider = || tcl_bigip_io::resolve_passphrase(&opts);
    let scf = ucs_archive_to_scf(&enc, &provider, false, "sample-encrypted.ucs").unwrap();
    assert_eq!(scf, read_text("sample.scf.golden"));
}

#[test]
fn pure_rust_decrypt_handles_aes256_uncompressed() {
    // AES-256 session key (32 bytes) + no inner compression (algo 0) — the
    // other ends of the cipher / decompress dispatch.
    let enc = read("sample-aes256.ucs");
    let plain = read("sample.ucs");
    let got = decrypt_symmetric(&enc, passphrase().as_bytes()).expect("decrypt aes256");
    assert_eq!(got, plain);
}

#[test]
fn wrong_passphrase_is_rejected() {
    let enc = read("sample-encrypted.ucs");
    let err = decrypt_symmetric(&enc, b"not the passphrase").unwrap_err();
    // The quick-check or MDC catches a bad passphrase before any plaintext.
    let msg = err.to_string();
    assert!(
        msg.contains("passphrase") || msg.contains("integrity"),
        "unexpected error: {msg}"
    );
}

#[test]
fn read_path_extracts_plain_ucs_file() {
    let path = fixtures().join("sample.ucs");
    let (_uri, source) = read_path(path.to_str().unwrap(), false, &no_passphrase()).unwrap();
    assert_eq!(source, read_text("sample.scf.golden"));
}

#[test]
fn read_path_extracts_encrypted_ucs_file() {
    let path = fixtures().join("sample-encrypted.ucs");
    let (_uri, source) = read_path(path.to_str().unwrap(), false, &with_passphrase()).unwrap();
    assert_eq!(source, read_text("sample.scf.golden"));
}

// Passphrase resolution order (explicit → env → prompt → error)

// The `Result` is required to match the `PassphraseOptions::prompt` fn-pointer
// type, so the always-`Ok` wrap is intentional.
#[allow(clippy::unnecessary_wraps)]
fn prompt_sentinel() -> Result<String, String> {
    Ok("FROM_PROMPT".to_owned())
}

#[test]
fn env_var_takes_precedence_over_prompt() {
    // Even with a TTY-style prompt wired in, a set env var wins and the prompt
    // is never consulted. `temp_env` scopes the env mutation (and serialises it
    // against other env-touching tests) so the crate stays unsafe-free.
    temp_env::with_var("F5_UCS_PRECEDENCE_TEST", Some("FROM_ENV"), || {
        let opts = PassphraseOptions {
            explicit: None,
            env_var: "F5_UCS_PRECEDENCE_TEST".to_owned(),
            allow_prompt: true,
            prompt: Some(prompt_sentinel),
        };
        assert_eq!(
            tcl_bigip_io::resolve_passphrase(&opts).expect("env passphrase"),
            "FROM_ENV"
        );
    });
}

#[test]
fn prompt_used_only_when_env_absent() {
    let opts = PassphraseOptions {
        explicit: None,
        env_var: "F5_UCS_UNSET_FOR_PROMPT_TEST".to_owned(),
        allow_prompt: true,
        prompt: Some(prompt_sentinel),
    };
    assert_eq!(
        tcl_bigip_io::resolve_passphrase(&opts).expect("prompt passphrase"),
        "FROM_PROMPT"
    );
}

#[test]
fn prompt_skipped_when_disabled() {
    let opts = PassphraseOptions {
        explicit: None,
        env_var: "F5_UCS_UNSET_FOR_DISABLED_TEST".to_owned(),
        allow_prompt: false,
        prompt: Some(prompt_sentinel),
    };
    assert!(tcl_bigip_io::resolve_passphrase(&opts).is_err());
}

fn original_member_archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    for (name, bytes) in members {
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(bytes.len()).unwrap());
        header.set_mode(0o644);
        header.set_cksum();
        archive.append_data(&mut header, name, *bytes).unwrap();
    }
    archive.into_inner().unwrap().finish().unwrap()
}

#[test]
fn original_selected_archive_members_control_source_fidelity() {
    use tcl_bigip_io::ucs_to_scf_source;
    let invalid = b"ltm pool /Common/\xff { }\n";
    let replacement = "ltm pool /Common/\u{fffd} { }\n".as_bytes();
    let source = ucs_to_scf_source(
        &original_member_archive(&[("config/bigip.conf", invalid)]),
        false,
    )
    .unwrap();
    assert!(source.text().contains('\u{fffd}'));
    assert!(
        source
            .require_faithful()
            .unwrap_err()
            .to_string()
            .contains("config/bigip.conf")
    );
    let literal = ucs_to_scf_source(
        &original_member_archive(&[("config/bigip.conf", replacement)]),
        false,
    )
    .unwrap();
    assert_eq!(source.text(), literal.text());
    literal.require_faithful().unwrap();
    let selected = original_member_archive(&[
        ("config/bigip.conf", invalid),
        ("config/bigip.conf", replacement),
        ("config/unselected.conf", invalid),
    ]);
    ucs_to_scf_source(&selected, false)
        .unwrap()
        .require_faithful()
        .unwrap();
    assert!(
        ucs_to_scf_source(&selected, true)
            .unwrap()
            .require_faithful()
            .is_err()
    );
}

#[test]
fn strict_archive_path_reads_validate_selected_member_bytes() {
    let dir = std::env::temp_dir().join(format!("f5-ucs-fidelity-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("invalid.ucs");
    std::fs::write(
        &path,
        original_member_archive(&[("config/bigip.conf", b"ltm pool /Common/\xff { }\n")]),
    )
    .unwrap();
    assert!(read_path(path.to_str().unwrap(), false, &no_passphrase()).is_ok());
    assert!(read_path(path.to_str().unwrap(), true, &no_passphrase()).is_err());
    let input = tcl_bigip_io::read_path_source(path.to_str().unwrap(), &no_passphrase()).unwrap();
    assert!(input.source().contains('\u{fffd}'));
    assert!(input.require_faithful().is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn archive_path_preview_collisions_cannot_certify_source_selection() {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    let bytes = b"ltm pool /Common/p { }\n";
    for name in [
        b"config/partitions/\xff/bigip.conf".as_slice(),
        "config/partitions/\u{fffd}/bigip.conf".as_bytes(),
    ] {
        let mut header = tar::Header::new_gnu();
        header.as_mut_bytes()[..name.len()].copy_from_slice(name);
        header.set_size(u64::try_from(bytes.len()).unwrap());
        header.set_mode(0o644);
        header.set_cksum();
        archive.append(&header, bytes.as_slice()).unwrap();
    }
    let archive = archive.into_inner().unwrap().finish().unwrap();
    let source = tcl_bigip_io::ucs_to_scf_source(&archive, false).unwrap();
    assert!(source.text().contains("/Common/p"));
    assert!(
        source
            .require_faithful()
            .unwrap_err()
            .to_string()
            .contains("member path")
    );
}

#[test]
fn selected_member_sources_keep_first_match_bytes_and_completed_absence_separate() {
    use tcl_bigip_io::{find_ucs_member_source, read_ucs_member_source};
    let archive =
        original_member_archive(&[("etc/donor", b"\xff"), ("etc/donor", "\u{fffd}".as_bytes())]);
    let source =
        read_ucs_member_source(&archive, "etc/donor", &|| Ok(String::new()), "fixture").unwrap();
    assert_eq!(source.path(), "etc/donor");
    assert_eq!(source.bytes(), b"\xff");
    assert!(source.require_faithful().is_err());
    assert!(
        find_ucs_member_source(&archive, "absent", &|| Ok(String::new()), "fixture")
            .unwrap()
            .is_none()
    );
    assert!(
        find_ucs_member_source(
            b"invalid archive",
            "absent",
            &|| Ok(String::new()),
            "fixture"
        )
        .is_err()
    );
    let archive =
        original_member_archive(&[("etc/donor", "\u{fffd}".as_bytes()), ("etc/donor", b"\xff")]);
    read_ucs_member_source(&archive, "etc/donor", &|| Ok(String::new()), "fixture")
        .unwrap()
        .require_faithful()
        .unwrap();
}

#[test]
fn archive_inventory_reporting_labels_cannot_replace_original_path_owners() {
    use tcl_bigip_io::{MemberScope, list_ucs_members};
    let archive = original_member_archive(&[("etc/donor", b"content")]);
    let mut entries = list_ucs_members(
        &archive,
        &|| Ok(String::new()),
        "fixture",
        MemberScope::FullTree,
    )
    .unwrap();
    entries[0].require_faithful_path().unwrap();
    entries[0].path = "etc/other".to_owned();
    assert!(entries[0].require_faithful_path().is_err());
}
