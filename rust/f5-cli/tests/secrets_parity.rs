//! Behaviour tests for the `f5 encrypt-secrets` / `decrypt-secrets` verbs.
//!
//! Drives the built `f5-query` binary the way `tests/test_f5_secrets.py` drives
//! the Python CLI: a known F5 master key, a forced salt for deterministic
//! output, and the round-trip / idempotency / key-source / edge-case
//! assertions. The crypto known-answer + round-trip vectors themselves live in
//! the `f5mku` unit tests; here we assert the verbs' observable behaviour.
//!
//! Self-contained: no Python at test time.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// A real F5 master key (from the original f5mkupy docstrings).
const KEY: &str = "BHDLd0bbao1VlwpTk1sioQ==";

const CONF: &str = "ltm monitor https /Common/mon {\n    password clearpw\n    username admin\n}\nauth radius-server /Common/rad {\n    secret \"my radius secret\"\n}\nsys snmp {\n    communities {\n        public { community-name public }\n    }\n}\n";

/// A unique scratch dir, removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> ScratchDir {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("f5-secrets-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        ScratchDir(path)
    }
    fn write(&self, name: &str, content: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, content).expect("write fixture");
        p
    }
    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Run `f5-query` with `args`, clearing `$F5MKU` so the env never leaks a key
/// into a test that means to omit one. `env` adds extra environment.
fn run(args: &[&str], env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_f5-query"));
    cmd.args(args).env_remove("F5MKU");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn f5-query");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn round_trip() {
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);
    let sealed = dir.join("sealed.conf");

    let r = run(
        &[
            "encrypt-secrets",
            conf.to_str().unwrap(),
            "-k",
            KEY,
            "--salt",
            "ab",
            "-o",
            sealed.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(
        r.stderr.contains("encrypted: 2 secret(s)"),
        "stderr: {}",
        r.stderr
    );
    let sealed_text = std::fs::read_to_string(&sealed).unwrap();
    assert!(!sealed_text.contains("clearpw"));
    assert!(sealed_text.contains("$M$ab$"));

    let r = run(
        &["decrypt-secrets", sealed.to_str().unwrap(), "-k", KEY],
        &[],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(
        r.stderr.contains("decrypted: 2 secret(s)"),
        "stderr: {}",
        r.stderr
    );
    assert!(r.stdout.contains("password clearpw"));
    assert!(r.stdout.contains("my radius secret"));
}

#[test]
fn encrypt_is_idempotent() {
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);

    let r = run(&["encrypt-secrets", conf.to_str().unwrap(), "-k", KEY], &[]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let sealed = dir.write("sealed.conf", &r.stdout);

    let r = run(
        &["encrypt-secrets", sealed.to_str().unwrap(), "-k", KEY],
        &[],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(
        r.stderr
            .contains("encrypted: 0 secret(s); 2 left untouched"),
        "stderr: {}",
        r.stderr
    );
}

#[test]
fn encrypt_skips_existing_crypt_hash() {
    // A crypt(3) hash sitting under a secret key must not be re-wrapped.
    let dir = ScratchDir::new();
    let conf = dir.write(
        "bigip.conf",
        "ltm monitor http /Common/m {\n    password $6$salt$hash\n}\n",
    );
    let r = run(&["encrypt-secrets", conf.to_str().unwrap(), "-k", KEY], &[]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(
        r.stderr
            .contains("encrypted: 0 secret(s); 1 left untouched"),
        "stderr: {}",
        r.stderr
    );
    assert!(r.stdout.contains("password $6$salt$hash"));
}

#[test]
fn alias_encrypt_decrypt() {
    // The `encrypt` / `decrypt` aliases resolve to the same verbs.
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);
    let r = run(
        &["encrypt", conf.to_str().unwrap(), "-k", KEY, "--salt", "ab"],
        &[],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("$M$ab$"));
}

#[test]
fn tmsh_delta_does_not_recreate_existing_objects() {
    // With --format tmsh-delta the pre-rewrite source is the delta baseline, so
    // existing objects emit `modify`, not a spurious `create`.
    let dir = ScratchDir::new();
    let conf = dir.write(
        "bigip.conf",
        "ltm monitor https /Common/mon {\n    password clearpw\n}\n",
    );
    let r = run(
        &[
            "encrypt-secrets",
            conf.to_str().unwrap(),
            "-k",
            KEY,
            "--salt",
            "ab",
            "--format",
            "tmsh-delta",
        ],
        &[],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(!r.stdout.contains("create"), "stdout: {}", r.stdout);
}

#[test]
fn key_from_env() {
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);
    let r = run(
        &["encrypt-secrets", conf.to_str().unwrap()],
        &[("F5MKU", KEY)],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("$M$"));
}

#[test]
fn key_from_file() {
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);
    // A trailing newline (as `f5mku -K > key.txt` leaves) must be trimmed.
    let keyfile = dir.write("key.txt", &format!("{KEY}\n"));
    let r = run(
        &[
            "encrypt-secrets",
            conf.to_str().unwrap(),
            "--f5mku-file",
            keyfile.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("$M$"));
}

#[test]
fn missing_key_errors() {
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);
    let r = run(
        &["encrypt-secrets", conf.to_str().unwrap(), "--no-key-prompt"],
        &[],
    );
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("no master key"), "stderr: {}", r.stderr);
}

#[test]
fn bad_key_errors() {
    let dir = ScratchDir::new();
    let conf = dir.write("bigip.conf", CONF);
    let r = run(
        &[
            "encrypt-secrets",
            conf.to_str().unwrap(),
            "-k",
            "not base64!!!",
        ],
        &[],
    );
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("f5mku"), "stderr: {}", r.stderr);
}
