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

//! Differential tests for `f5 enrich-wireshark` and `f5 enrich-pcapng`.
//!
//! Runs the built `f5-query` binary against committed bigip.conf fixtures and
//! asserts byte-for-byte equality with the captured golden output for
//! `enrich-wireshark` / `enrich-pcapng`:
//!
//! - `enrich-wireshark`: every generated profile file (`hosts` / `subnets` /
//!   `vlans` / `dfilters` / `services` / `ethers` / `colorfilters` /
//!   `preferences` / `README.md`) for two configs, plus the stderr summary and
//!   the not-a-directory / already-exists refusals.
//! - `enrich-pcapng`: the `NameIndex`-derived `--dry-run` dump, the
//!   observed-mode / `--all` / `--keylog` PCAPNG annotation byte-output (direct
//!   PCAPNG→PCAPNG write, no tshark), and the not-a-file refusal.
//!
//! Self-contained: no external tool runs at test time. The tshark/editcap-driven libpcap
//! conversion path is out of scope here (its bytes depend on the external
//! tool's version); the deterministic direct-write paths are covered.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> ScratchDir {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("f5-enrich-parity-{pid}-{n}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        ScratchDir(path)
    }
    fn path(&self) -> &Path {
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

fn run(verb: &str, args: &[&std::ffi::OsStr]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg(verb)
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

// enrich-wireshark

/// Generate a profile from `config_fixture` into a directory named
/// `profile_name`, then assert every file matches the golden directory.
fn assert_wireshark_profile(config_fixture: &str, profile_name: &str) {
    let scratch = ScratchDir::new();
    let out_dir = scratch.path().join(profile_name);
    let cfg = osv("-c");
    let config = fixture(config_fixture);
    let r = run(
        "enrich-wireshark",
        &[cfg.as_os_str(), config.as_os_str(), out_dir.as_os_str()],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);

    let golden = golden_dir().join(profile_name);
    let mut golden_names: Vec<String> = std::fs::read_dir(&golden)
        .expect("read golden dir")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    golden_names.sort();
    let mut got_names: Vec<String> = std::fs::read_dir(&out_dir)
        .expect("read output dir")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    got_names.sort();
    assert_eq!(got_names, golden_names, "profile file set must match");

    for name in &golden_names {
        let got = std::fs::read(out_dir.join(name)).unwrap();
        let want = std::fs::read(golden.join(name)).unwrap();
        assert_eq!(
            got, want,
            "profile file `{name}` for {config_fixture} must be byte-identical"
        );
    }
}

#[test]
fn wireshark_rich_profile_byte_identical() {
    assert_wireshark_profile("enrich-rich.conf", "enrich-ws-rich");
}

#[test]
fn wireshark_bigip_profile_byte_identical() {
    assert_wireshark_profile("bigip.conf", "enrich-ws-bigip");
}

#[test]
fn wireshark_summary_stderr_matches() {
    let scratch = ScratchDir::new();
    let out_dir = scratch.path().join("prof");
    let cfg = osv("-c");
    let config = fixture("enrich-rich.conf");
    let r = run(
        "enrich-wireshark",
        &[cfg.as_os_str(), config.as_os_str(), out_dir.as_os_str()],
    );
    assert_eq!(r.code, 0);
    assert_eq!(
        r.stderr,
        format!(
            "enrich-wireshark: 12 host(s), 2 subnet(s), 2 vlan(s), 6 display-filter(s), \
             22 service(s), 1 ether(s), 5 colour rule(s) written to {}\n",
            out_dir.display()
        )
    );
}

#[test]
fn wireshark_refuses_nonempty_dir_without_force() {
    let scratch = ScratchDir::new();
    let out_dir = scratch.path().join("existing");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(out_dir.join("keep.txt"), b"hi").unwrap();
    let cfg = osv("-c");
    let config = fixture("enrich-rich.conf");
    let r = run(
        "enrich-wireshark",
        &[cfg.as_os_str(), config.as_os_str(), out_dir.as_os_str()],
    );
    assert_eq!(r.code, 2);
    assert_eq!(
        r.stderr,
        format!(
            "error: {} already exists and is not empty; pass --force to overwrite\n",
            out_dir.display()
        )
    );
}

#[test]
fn wireshark_refuses_non_directory_output() {
    let scratch = ScratchDir::new();
    let out = scratch.path().join("afile");
    std::fs::write(&out, b"x").unwrap();
    let cfg = osv("-c");
    let config = fixture("enrich-rich.conf");
    let r = run(
        "enrich-wireshark",
        &[cfg.as_os_str(), config.as_os_str(), out.as_os_str()],
    );
    assert_eq!(r.code, 2);
    assert_eq!(
        r.stderr,
        format!("error: not a directory: {}\n", out.display())
    );
}

// enrich-pcapng

#[test]
fn pcapng_dry_run_name_index_matches() {
    let cfg = osv("-c");
    let config = fixture("enrich-rich.conf");
    let dry = osv("--dry-run");
    let devnull = osv("/dev/null");
    let r = run(
        "enrich-pcapng",
        &[
            cfg.as_os_str(),
            config.as_os_str(),
            dry.as_os_str(),
            devnull.as_os_str(),
            devnull.as_os_str(),
        ],
    );
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let want = "\
10.0.0.250\tself-common-external-self
10.0.0.5\tvs-common-web-vs
10.0.0.5\tirule-common-redir-rule
10.0.0.5\twideip-common-www-example-com
10.0.0.6\tvs-common-dns-vs
10.0.1.10\tpool-common-web1-80
10.0.1.10\tnode-common-web1
10.0.1.11\tpool-common-web2-80
10.0.1.11\tnode-common-web2
10.0.1.250\tself-common-internal-self
10.0.2.50\tsnat-common-snat1
10.0.2.51\tsnat-common-snat1
10.0.0.0/24\tnet-common-external-self
10.0.1.0/24\tnet-common-internal-self
";
    assert_eq!(r.stdout, want);
    assert_eq!(
        r.stderr,
        "enrich-pcapng (dry-run): 8 v4 / 0 v6 address(es), 2 subnet(s), 14 label(s)\n"
    );
}

/// Run `enrich-pcapng` writing `out`, asserting exit 0 and the byte output.
fn enrich_pcapng_to(extra: &[&std::ffi::OsStr], out: &Path, golden: &str) -> Run {
    let cfg = osv("-c");
    let config = fixture("enrich-rich.conf");
    let input = fixture("enrich-sample.pcapng");
    let mut args: Vec<&std::ffi::OsStr> = vec![cfg.as_os_str(), config.as_os_str()];
    args.extend_from_slice(extra);
    args.push(input.as_os_str());
    args.push(out.as_os_str());
    let r = run("enrich-pcapng", &args);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(
        std::fs::read(out).unwrap(),
        read_fixture(golden),
        "pcapng output must match golden ({golden})"
    );
    r
}

#[test]
fn pcapng_observed_annotation_byte_identical() {
    let scratch = ScratchDir::new();
    let out = scratch.path().join("obs.pcapng");
    let r = enrich_pcapng_to(&[], &out, "enrich-sample.observed.pcapng.golden");
    assert_eq!(
        r.stderr,
        "enrich-pcapng: 4 v4 / 0 v6 record(s), 10 label(s) out of 4 v4 / 0 v6 observed\n"
    );
}

#[test]
fn pcapng_all_annotation_byte_identical() {
    let scratch = ScratchDir::new();
    let out = scratch.path().join("all.pcapng");
    let all = osv("--all");
    let r = enrich_pcapng_to(&[all.as_os_str()], &out, "enrich-sample.all.pcapng.golden");
    assert_eq!(
        r.stderr,
        "enrich-pcapng: 8 v4 / 0 v6 record(s), 12 label(s) (--all: every inventory entry)\n"
    );
}

#[test]
fn pcapng_keylog_dsb_byte_identical() {
    let scratch = ScratchDir::new();
    let out = scratch.path().join("kl.pcapng");
    let all = osv("--all");
    let kflag = osv("--keylog");
    let keylog = fixture("enrich-keys.log");
    let r = enrich_pcapng_to(
        &[all.as_os_str(), kflag.as_os_str(), keylog.as_os_str()],
        &out,
        "enrich-sample.keylog.pcapng.golden",
    );
    assert_eq!(
        r.stderr,
        "enrich-pcapng: 8 v4 / 0 v6 record(s), 12 label(s) (--all: every inventory entry), \
         keylog 22 byte(s) embedded\n"
    );
}

#[test]
fn pcapng_refuses_missing_input_file() {
    let scratch = ScratchDir::new();
    let out = scratch.path().join("nope.pcapng");
    let cfg = osv("-c");
    let config = fixture("enrich-rich.conf");
    let missing = scratch.path().join("does-not-exist.pcapng");
    let r = run(
        "enrich-pcapng",
        &[
            cfg.as_os_str(),
            config.as_os_str(),
            missing.as_os_str(),
            out.as_os_str(),
        ],
    );
    assert_eq!(r.code, 2);
    assert_eq!(
        r.stderr,
        format!("error: not a file: {}\n", missing.display())
    );
    assert!(!out.exists());
}
