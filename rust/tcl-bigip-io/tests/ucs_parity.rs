//! Differential parity tests for UCS handling against the Python reference.
//!
//! Fixtures are captured once from `tooling.f5.f5_remote.ucs` (a plain UCS, an
//! encrypted UCS produced by `gpg --symmetric`, and the golden SCF text from
//! `ucs_to_scf`). The tests themselves are self-contained — no Python, no gpg —
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
fn plain_ucs_to_scf_matches_python() {
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
fn encrypted_ucs_to_scf_matches_python() {
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

// --- Passphrase resolution order (explicit → env → prompt → error) ----------

fn prompt_sentinel() -> Result<String, String> {
    Ok("FROM_PROMPT".to_owned())
}

#[test]
fn env_var_takes_precedence_over_prompt() {
    // Even with a TTY-style prompt wired in, a set env var wins and the prompt
    // is never consulted.
    std::env::set_var("F5_UCS_PRECEDENCE_TEST", "FROM_ENV");
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
    std::env::remove_var("F5_UCS_PRECEDENCE_TEST");
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
