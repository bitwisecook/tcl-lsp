//! Differential parity tests for the `f5 redact` / `f5 unredact` verbs — secret
//! stripping + the reversible IP-remap engine (`tcl-bigip::redact`).
//!
//! Runs the built `f5-query` binary against the committed `redact.conf` fixture
//! and asserts the redacted config, the sidecar `.map` file, and the stderr
//! summary line all match the captured golden output for `redact` /
//! `unredact`. Self-contained: no external tool runs at test time.
//!
//! Paths that vary per machine are normalised: the temp map path the summary
//! line carries is replaced with `__MAP__`, and the fixtures dir (which never
//! actually appears in these goldens) with `__FIXTURES__`.
//!
//! `--shuffle` reaches byte-parity here: `tcl-bigip::redact` reproduces the
//! `MT19937` Fisher-Yates shuffle permutation exactly, so the shuffled map
//! + config are byte-identical to the captured golden output.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A unique, freshly-created scratch directory under the system temp dir.
/// Returned [`ScratchDir`] removes the tree on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> ScratchDir {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("f5-redact-parity-{tag}-{pid}-{n}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        ScratchDir(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixtures_path() -> String {
    std::fs::canonicalize(fixtures_dir())
        .expect("canonicalize fixtures dir")
        .to_string_lossy()
        .into_owned()
}

fn conf() -> String {
    std::fs::canonicalize(fixtures_dir().join("redact.conf"))
        .expect("canonicalize redact.conf")
        .to_string_lossy()
        .into_owned()
}

/// Read a golden, expanding `__FIXTURES__` to the real fixtures path.
fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
        .replace("__FIXTURES__", &fixtures_path())
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_f5-query"))
}

/// Run `f5-query redact <conf> -o <out> --map-file <map> [extra…]` and return
/// `(stdout, stderr, redacted_config, map_contents)`. The summary line's temp
/// map path is normalised to `__MAP__`.
fn run_redact(extra: &[&str]) -> (String, String, String, String) {
    let dir = ScratchDir::new("default");
    let out_path = dir.path().join("out.conf");
    let map_path = dir.path().join("out.map");
    let conf = conf();
    let mut args: Vec<String> = vec![
        "redact".into(),
        conf,
        "-o".into(),
        out_path.to_string_lossy().into_owned(),
        "--map-file".into(),
        map_path.to_string_lossy().into_owned(),
    ];
    for e in extra {
        args.push((*e).to_owned());
    }
    let output = bin().args(&args).output().expect("spawn f5-query");
    assert!(
        output.status.success(),
        "redact {extra:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .replace(&map_path.to_string_lossy().into_owned(), "__MAP__");
    let config = std::fs::read_to_string(&out_path).expect("read redacted config");
    let map = std::fs::read_to_string(&map_path).expect("read map file");
    (stdout, stderr, config, map)
}

/// Assert a default-style redact (config + map + summary) matches the goldens.
fn assert_redact(base: &str, extra: &[&str]) {
    let (stdout, stderr, config, map) = run_redact(extra);
    assert_eq!(
        stdout, "",
        "{base}: redact writes the config to -o, not stdout"
    );
    assert_eq!(
        config,
        golden(&format!("redact-{base}.conf.golden")),
        "{base}: redacted config does not match the golden"
    );
    assert_eq!(
        map,
        golden(&format!("redact-{base}.map.golden")),
        "{base}: sidecar map file does not match the golden"
    );
    assert_eq!(
        stderr,
        golden(&format!("redact-{base}.err.golden")),
        "{base}: summary line does not match the golden"
    );
}

#[test]
fn default_redact_secrets_and_direct_remap() {
    // Secrets + PEM stripped, public IPs remapped into RFC1918 (direct mode),
    // private 10.0.1.10 left alone. Both config + map byte-identical.
    assert_redact("default", &[]);
}

#[test]
fn remap_private_also_scrubs_rfc1918() {
    assert_redact("remap-private", &["--remap-private"]);
}

#[test]
fn custom_target_cidr_pool() {
    assert_redact("custom", &["--target-cidr", "172.20.0.0/16"]);
}

#[test]
fn shuffle_mode_byte_parity() {
    // MT19937 shuffle reproduced exactly: shuffled host bits + the
    // per-CIDR shuffle keys in the map match the golden byte-for-byte.
    assert_redact("shuffle", &["--shuffle", "--seed", "42"]);
}

#[test]
fn keep_ips_redacts_secrets_only() {
    // `--keep-ips` strips secrets + PEM but leaves every IP untouched and
    // writes no map file. The config goes to -o; the summary reports 0 remaps.
    let dir = ScratchDir::new("keepips");
    let out_path = dir.path().join("out.conf");
    let conf = conf();
    let output = bin()
        .args([
            "redact",
            &conf,
            "--keep-ips",
            "-o",
            &out_path.to_string_lossy(),
        ])
        .output()
        .expect("spawn f5-query");
    assert!(output.status.success());
    let config = std::fs::read_to_string(&out_path).expect("read config");
    assert_eq!(
        config,
        golden("redact-keepips.conf.golden"),
        "keep-ips: config does not match the golden"
    );
    // No map file is derived (no --map-file, but -o derives one) — but with
    // --keep-ips a map file is never written. Assert it was not created.
    let derived = format!("{}.redact.toml", out_path.to_string_lossy());
    assert!(
        !std::path::Path::new(&derived).exists(),
        "keep-ips must not write a sidecar map"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).replace(&derived, "__MAP__");
    assert_eq!(
        stderr,
        golden("redact-keepips.err.golden"),
        "keep-ips: summary line does not match the golden"
    );
}

/// Run a stdout-producing redact/unredact invocation (`--format tmsh` etc.)
/// and return `(stdout, stderr)`. No `-o` / `--map-file`, so no sidecar map is
/// written and the summary line carries no per-machine path.
fn run_to_stdout(args: &[&str]) -> (String, String) {
    let output = bin().args(args).output().expect("spawn f5-query");
    assert!(
        output.status.success(),
        "{args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn redact_format_tmsh_byte_parity() {
    // `--format tmsh` re-renders the redacted config as a `tmsh modify` script
    // (in-place rewriters use `modify`). With no `-o`/`--map-file` the script
    // goes to stdout and no map is written.
    let conf = conf();
    let (out, err) = run_to_stdout(&["redact", &conf, "--format", "tmsh"]);
    assert_eq!(
        out,
        golden("redact-tmsh.conf.golden"),
        "redact --format tmsh"
    );
    assert_eq!(err, golden("redact-tmsh.err.golden"), "redact tmsh summary");
}

#[test]
fn redact_format_tmsh_transaction_byte_parity() {
    // `--transaction` wraps the `tmsh modify` script in a `cli transaction`
    // envelope.
    let conf = conf();
    let (out, err) = run_to_stdout(&["redact", &conf, "--format", "tmsh", "--transaction"]);
    assert_eq!(
        out,
        golden("redact-tmsh-txn.conf.golden"),
        "redact --format tmsh --transaction"
    );
    assert_eq!(
        err,
        golden("redact-tmsh-txn.err.golden"),
        "redact tmsh transaction summary"
    );
}

#[test]
fn unredact_format_tmsh_byte_parity() {
    // `unredact --format tmsh` re-renders the recovered config as a `tmsh
    // modify` script. Drives committed `unredact-input.{conf,map}` (a frozen
    // redaction) so the run is deterministic with no external tool.
    let map = fixtures_dir()
        .join("unredact-input.map")
        .to_string_lossy()
        .into_owned();
    let red = fixtures_dir()
        .join("unredact-input.conf")
        .to_string_lossy()
        .into_owned();
    let (out, err) = run_to_stdout(&["unredact", &map, &red, "--format", "tmsh"]);
    assert_eq!(
        out,
        golden("unredact-tmsh.conf.golden"),
        "unredact --format tmsh"
    );
    assert_eq!(
        err,
        golden("unredact-tmsh.err.golden"),
        "unredact tmsh summary"
    );
}

#[test]
fn redact_unredact_round_trip() {
    // redact (default direct remap) -> unredact restores every remapped IP.
    let dir = ScratchDir::new("roundtrip");
    let red_path = dir.path().join("red.conf");
    let map_path = dir.path().join("red.map");
    let back_path = dir.path().join("back.conf");
    let conf = conf();

    let r = bin()
        .args([
            "redact",
            &conf,
            "-o",
            &red_path.to_string_lossy(),
            "--map-file",
            &map_path.to_string_lossy(),
        ])
        .output()
        .expect("spawn redact");
    assert!(r.status.success());

    let u = bin()
        .args([
            "unredact",
            &map_path.to_string_lossy(),
            &red_path.to_string_lossy(),
            "-o",
            &back_path.to_string_lossy(),
        ])
        .output()
        .expect("spawn unredact");
    assert!(u.status.success());

    let back = std::fs::read_to_string(&back_path).expect("read unredacted");
    assert_eq!(
        back,
        golden("redact-roundtrip.conf.golden"),
        "round-trip: unredacted config does not match the golden"
    );
    let uerr = String::from_utf8_lossy(&u.stderr).into_owned();
    assert_eq!(
        uerr,
        golden("redact-roundtrip.uerr.golden"),
        "round-trip: unredact summary does not match the golden"
    );
}
