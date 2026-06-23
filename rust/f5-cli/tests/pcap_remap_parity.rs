//! Differential parity tests for the `f5 pcap-remap` verb — the PCAP IP-remap
//! engine (`tcl-bigip::pcap_remap`).
//!
//! Runs the built `f5-query` binary against committed, hand-emitted libpcap and
//! PCAPNG fixtures plus a redaction `.map`, and asserts the OUTPUT capture is
//! byte-for-byte identical to the captured golden output. Also covers the
//! stderr summary line, the `--list-schemas` stdout, and the
//! `--on-unknown error/preserve/sweep` policies (incl. the exit-2
//! unknown-trailer error and its message). Self-contained: no external tool
//! runs at test time.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

/// A unique scratch directory; removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> ScratchDir {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("f5-pcap-remap-parity-{pid}-{n}"));
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

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&std::ffi::OsStr]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("pcap-remap")
        .args(args)
        .output()
        .expect("spawn f5-query");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn osv(s: &str) -> std::ffi::OsString {
    std::ffi::OsString::from(s)
}

#[test]
fn libpcap_forward_byte_identical() {
    let scratch = ScratchDir::new();
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-sample.pcap");
    let output = scratch.path().join("out.pcap");
    let r = run(&[map.as_os_str(), input.as_os_str(), output.as_os_str()]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let got = std::fs::read(&output).expect("read output");
    let golden = read_fixture("pcap-remap-sample.forward.pcap.golden");
    assert_eq!(
        got, golden,
        "forward libpcap output must match golden"
    );
    assert_eq!(
        r.stderr,
        "pcap-remap: 6/7 packet(s) rewritten, 14 address(es) changed; \
         trailer TLVs: 1 rewritten / 0 unknown / 1 total\n"
    );
}

#[test]
fn libpcap_round_trip_identity() {
    // forward then --reverse must reproduce the original input byte-for-byte.
    let scratch = ScratchDir::new();
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-sample.pcap");
    let fwd = scratch.path().join("fwd.pcap");
    let rt = scratch.path().join("rt.pcap");
    let r1 = run(&[map.as_os_str(), input.as_os_str(), fwd.as_os_str()]);
    assert_eq!(r1.code, 0, "{}", r1.stderr);
    let rev = osv("--reverse");
    let r2 = run(&[
        map.as_os_str(),
        fwd.as_os_str(),
        rt.as_os_str(),
        rev.as_os_str(),
    ]);
    assert_eq!(r2.code, 0, "{}", r2.stderr);
    assert_eq!(
        std::fs::read(&rt).unwrap(),
        read_fixture("pcap-remap-sample.pcap"),
        "round-trip must restore the original capture"
    );
}

#[test]
fn pcapng_forward_byte_identical() {
    let scratch = ScratchDir::new();
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-sample.pcapng");
    let output = scratch.path().join("out.pcapng");
    let r = run(&[map.as_os_str(), input.as_os_str(), output.as_os_str()]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(
        std::fs::read(&output).unwrap(),
        read_fixture("pcap-remap-sample.pcapng.golden"),
        "pcapng output must match golden"
    );
    assert_eq!(
        r.stderr,
        "pcap-remap: 2/2 packet(s) rewritten, 4 address(es) changed; \
         trailer TLVs: 0 rewritten / 0 unknown / 0 total\n"
    );
}

#[test]
fn unknown_trailer_error_exits_2_and_writes_nothing() {
    let scratch = ScratchDir::new();
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-unknown.pcap");
    let output = scratch.path().join("nope.pcap");
    let r = run(&[map.as_os_str(), input.as_os_str(), output.as_os_str()]);
    assert_eq!(r.code, 2);
    assert!(!output.exists(), "output must not be written on error");
    assert_eq!(
        r.stderr,
        "error: f5 trailer entry has no registered schema: \
         format=dpt type=3 version=99 length=16\n  -> rerun with \
         --on-unknown=preserve|sweep, or supply a matching --schema file \
         to add the missing layout.\n"
    );
}

#[test]
fn unknown_trailer_preserve_byte_identical() {
    let scratch = ScratchDir::new();
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-unknown.pcap");
    let output = scratch.path().join("up.pcap");
    let policy = osv("preserve");
    let flag = osv("--on-unknown");
    let r = run(&[
        map.as_os_str(),
        input.as_os_str(),
        output.as_os_str(),
        flag.as_os_str(),
        policy.as_os_str(),
    ]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(
        std::fs::read(&output).unwrap(),
        read_fixture("pcap-remap-unknown.preserve.pcap.golden")
    );
    assert_eq!(
        r.stderr,
        "pcap-remap: 1/1 packet(s) rewritten, 2 address(es) changed; \
         trailer TLVs: 0 rewritten / 1 unknown / 1 total\n"
    );
}

#[test]
fn unknown_trailer_sweep_byte_identical() {
    let scratch = ScratchDir::new();
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-unknown.pcap");
    let output = scratch.path().join("us.pcap");
    let policy = osv("sweep");
    let flag = osv("--on-unknown");
    let r = run(&[
        map.as_os_str(),
        input.as_os_str(),
        output.as_os_str(),
        flag.as_os_str(),
        policy.as_os_str(),
    ]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(
        std::fs::read(&output).unwrap(),
        read_fixture("pcap-remap-unknown.sweep.pcap.golden")
    );
    // sweep rewrites the embedded IP inside the unknown TLV (3 addrs, 1 TLV).
    assert_eq!(
        r.stderr,
        "pcap-remap: 1/1 packet(s) rewritten, 3 address(es) changed; \
         trailer TLVs: 1 rewritten / 1 unknown / 1 total\n"
    );
}

#[test]
fn list_schemas_matches_python() {
    let x = osv("x");
    let y = osv("y");
    let z = osv("z");
    let flag = osv("--list-schemas");
    let r = run(&[
        x.as_os_str(),
        y.as_os_str(),
        z.as_os_str(),
        flag.as_os_str(),
    ]);
    assert_eq!(r.code, 0);
    let expected = "\
legacy:
  type=1 version=0 length=22 fields=0
  type=1 version=0 length=35 fields=0
  type=2 version=0 length=8 fields=0
  type=2 version=0 length=21 fields=0
  type=2 version=0 length=29 fields=0
  type=3 version=0 length=42 fields=2
dpt:
  provider=1 type=1 version=1 fields=0
  provider=1 type=1 version=2 fields=0
  provider=1 type=1 version=3 fields=0
  provider=1 type=1 version=4 fields=0
  provider=1 type=2 version=1 fields=0
  provider=1 type=2 version=4 fields=0
  provider=1 type=3 version=1 fields=2
  provider=4 type=0 version=0 fields=0
  provider=4 type=0 version=1 fields=0
  provider=4 type=1 version=0 fields=0
  provider=4 type=1 version=1 fields=0
  provider=4 type=2 version=0 fields=0
  provider=4 type=2 version=1 fields=0
  provider=4 type=3 version=0 fields=0
  provider=4 type=3 version=1 fields=0
  provider=5 type=1 version=0 fields=0
  provider=5 type=1 version=1 fields=0
";
    assert_eq!(r.stdout, expected);
}

#[test]
fn list_schemas_with_overlay_byte_identical() {
    // A `--schema` overlay adds an unknown legacy layout + a new DPT provider
    // and overrides a built-in field count; `--list-schemas` reflects the
    // merged, re-sorted registry. Map/input/output are unused on this path.
    let scratch = ScratchDir::new();
    let dummy = scratch.path().join("unused");
    let overlay = fixture("pcap-remap-schema-overlay.toml");
    let r = run(&[
        dummy.as_os_str(),
        dummy.as_os_str(),
        dummy.as_os_str(),
        std::ffi::OsStr::new("--list-schemas"),
        std::ffi::OsStr::new("--schema"),
        overlay.as_os_str(),
    ]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(
        r.stdout.into_bytes(),
        read_fixture("pcap-remap-list-schemas-overlay.golden"),
        "list-schemas-with-overlay must match the golden"
    );
}

#[test]
fn overlay_makes_unknown_trailer_known() {
    // Without a schema the unknown DPT TLV (type=3 version=99) errors under the
    // default policy; a `--schema` overlay teaching its layout makes the remap
    // succeed (0 unknown TLVs) instead of bailing.
    let scratch = ScratchDir::new();
    let overlay_path = scratch.path().join("fix.toml");
    std::fs::write(
        &overlay_path,
        "[[dpt]]\nprovider = 1\ntype = 3\nversion = 99\nip_fields = [ { offset = 8, kind = \"v4\" } ]\n",
    )
    .expect("write overlay");
    let map = fixture("pcap-remap.map.toml");
    let input = fixture("pcap-remap-unknown.pcap");
    let output = scratch.path().join("out.pcap");
    let r = run(&[
        map.as_os_str(),
        input.as_os_str(),
        output.as_os_str(),
        std::ffi::OsStr::new("--schema"),
        overlay_path.as_os_str(),
    ]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(
        r.stderr.contains("0 unknown"),
        "overlay should make the TLV schema-known, got: {}",
        r.stderr
    );
    assert!(output.is_file(), "output should be written");
}
