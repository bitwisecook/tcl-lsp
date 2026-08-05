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

//! Unit tests plus **differential tests against a real `tclsh`** for the
//! package / auto-load resolver.
//!
//! The differential tests generate genuine `pkgIndex.tcl` / `tclIndex` files
//! with C Tcl's own `pkg_mkIndex` / `auto_mkindex`, and query C Tcl's own
//! `auto_qualify`, then assert we produce the same result. They
//! skip cleanly when no `tclsh` is on `PATH` (so CI without Tcl still passes),
//! while the pure unit tests always run.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

// ---------------------------------------------------------------------------
// Test scaffolding.
// ---------------------------------------------------------------------------

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A throwaway directory under the system temp dir, removed on drop.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("tcl-lsp-pkgres-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, content).expect("write file");
}

/// The first usable `tclsh` on `PATH`, or `None` (tests then skip).
fn find_tclsh() -> Option<&'static str> {
    ["tclsh9.0", "tclsh8.6", "tclsh"]
        .into_iter()
        .find(|&sh| run_tclsh(sh, "puts ok").as_deref().map(str::trim) == Some("ok"))
        .map(|v| v as _)
}

/// Run `script` through `tclsh` (fed on stdin), returning trimmed stdout.
fn run_tclsh(sh: &str, script: &str) -> Option<String> {
    use std::io::Write as _;
    let mut child = Command::new(sh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(script.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

fn always_exists(_: &Path) -> bool {
    true
}

fn no_tcl_files(_: &Path) -> Vec<PathBuf> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// auto_qualify — pure cases + differential against real tclsh.
// ---------------------------------------------------------------------------

#[test]
fn auto_qualify_matches_c_tcl_documented_cases() {
    // The exact examples from the C Tcl proc's own comments
    // (library/init.tcl:488).
    assert_eq!(auto_qualify("::foo::bar", "::ignored"), vec!["::foo::bar"]);
    assert_eq!(auto_qualify("::global", "::ignored"), vec!["global"]);
    assert_eq!(auto_qualify("nocolons", "::"), vec!["nocolons"]);
    assert_eq!(
        auto_qualify("nocolons", "::sub"),
        vec!["::sub::nocolons", "nocolons"]
    );
    assert_eq!(auto_qualify("foo::bar", "::"), vec!["::foo::bar"]);
    assert_eq!(
        auto_qualify("foo::bar", "::sub"),
        vec!["::sub::foo::bar", "::foo::bar"]
    );
    // Colon-run collapsing: `foo:::::bar` normalises to `foo::bar` (one run).
    assert_eq!(auto_qualify("foo:::::bar", "::"), vec!["::foo::bar"]);
}

#[test]
fn auto_qualify_differential_against_tclsh() {
    let Some(sh) = find_tclsh() else {
        eprintln!("skipping auto_qualify differential: no tclsh on PATH");
        return;
    };
    let cases = [
        ("::foo::bar", "::"),
        ("::global", "::"),
        ("nocolons", "::"),
        ("nocolons", "::sub"),
        ("foo::bar", "::"),
        ("foo::bar", "::sub"),
        ("foo:::::bar", "::sub"),
        ("::a::b::c", "::x"),
        ("::single", "::deep::ns"),
        ("a", "::deep::ns"),
        ("ns::cmd", "::deep::ns"),
    ];
    for (cmd, ns) in cases {
        let script = format!("puts [auto_qualify {{{cmd}}} {{{ns}}}]");
        let Some(reference) = run_tclsh(sh, &script) else {
            panic!("tclsh failed for auto_qualify {cmd} {ns}");
        };
        let ours = auto_qualify(cmd, ns).join(" ");
        assert_eq!(
            ours,
            reference.trim(),
            "auto_qualify({cmd:?}, {ns:?}) diverged from tclsh",
        );
    }
}

// ---------------------------------------------------------------------------
// pkgIndex.tcl parsing — pure cases + differential against pkg_mkIndex.
// ---------------------------------------------------------------------------

/// Regression coverage for issue #996: `collect_source_targets` recurses
/// once per `[...]`/`{...}`/`"..."` wrapper word, with no depth cap
/// before this fix (`MAX_SOURCE_TARGET_SCAN_DEPTH`). 2000 nested `[list
/// ...]` wrappers is comfortably past that cap (256); the assertion is
/// that parsing returns at all, not what it returns.
#[test]
fn parse_pkg_index_survives_deeply_nested_source_wrapper() {
    const DEPTH: usize = 2000;
    let mut body = String::from("source $dir/x.tcl");
    for _ in 0..DEPTH {
        body = format!("[list {body}]");
    }
    let content = format!("package ifneeded p 1.0 {body}");
    let dir = Path::new("/pkg");
    let _ = parse_pkg_index(
        &content,
        dir,
        &dir.join("pkgIndex.tcl"),
        &always_exists,
        &no_tcl_files,
    );
}

#[test]
fn parse_pkg_index_standard_form() {
    // The canonical `pkg_mkIndex` output shape.
    let content = "package ifneeded http 1.0 [list source [file join $dir http.tcl]]";
    let dir = Path::new("/pkg/http1.0");
    let infos = parse_pkg_index(
        content,
        dir,
        &dir.join("pkgIndex.tcl"),
        &always_exists,
        &no_tcl_files,
    );
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].name, "http");
    assert_eq!(infos[0].version, "1.0");
    assert_eq!(infos[0].source_files, vec![dir.join("http.tcl")]);
}

#[test]
fn parse_pkg_index_multi_component_file_join_uses_slash() {
    // `[file join $dir src impl.tcl]` — the components after `$dir` join with
    // the directory separator, so the resolved file is `<dir>/src/impl.tcl`,
    // not `<dir>/"src impl.tcl"` (issue 177).
    let content = "package ifneeded p 1.0 [list source [file join $dir src impl.tcl]]";
    let dir = Path::new("/pkg/p1.0");
    let infos = parse_pkg_index(
        content,
        dir,
        &dir.join("pkgIndex.tcl"),
        &always_exists,
        &no_tcl_files,
    );
    assert_eq!(infos.len(), 1);
    assert_eq!(
        infos[0].source_files,
        vec![dir.join("src").join("impl.tcl")]
    );
}

#[test]
fn parse_pkg_index_quoted_and_dir_slash_forms() {
    // `"source $dir/x.tcl"` and `$dir/x.tcl` are both reached.
    let content = "package ifneeded a 1 \"source $dir/a.tcl\"\n\
                   package ifneeded b 2 [list source $dir/b.tcl]";
    let dir = Path::new("/pkg");
    let infos = parse_pkg_index(
        content,
        dir,
        &dir.join("pkgIndex.tcl"),
        &always_exists,
        &no_tcl_files,
    );
    let by_name: std::collections::HashMap<_, _> =
        infos.iter().map(|i| (i.name.as_str(), i)).collect();
    assert_eq!(by_name["a"].source_files, vec![dir.join("a.tcl")]);
    assert_eq!(by_name["b"].source_files, vec![dir.join("b.tcl")]);
}

#[test]
fn parse_pkg_index_rejects_bad_version_and_falls_back_to_dir_listing() {
    // Unconventional version → skipped.
    let bad = "package ifneeded x notaversion [list source [file join $dir x.tcl]]";
    let dir = Path::new("/pkg");
    assert!(
        parse_pkg_index(
            bad,
            dir,
            &dir.join("pkgIndex.tcl"),
            &always_exists,
            &no_tcl_files
        )
        .is_empty()
    );

    // No explicit source → fall back to the directory's `*.tcl` listing.
    let body = "package ifneeded y 1.2 [list tclPkgSetup $dir y 1.2 {{y.tcl source {a}}}]";
    let list = |_: &Path| vec![dir.join("y.tcl"), dir.join("pkgIndex.tcl")];
    let infos = parse_pkg_index(body, dir, &dir.join("pkgIndex.tcl"), &always_exists, &list);
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].source_files, vec![dir.join("y.tcl")]);
}

#[test]
fn parse_pkg_index_differential_against_pkg_mkindex() {
    let Some(sh) = find_tclsh() else {
        eprintln!("skipping pkg_mkIndex differential: no tclsh on PATH");
        return;
    };
    let td = TempDir::new("pkgmk");
    let pkg_dir = td.path().join("foo");
    write(
        &pkg_dir.join("foo.tcl"),
        "package provide foo 2.3\nproc foo {} { return foo }\n",
    );
    // Generate a real pkgIndex.tcl with C Tcl's own pkg_mkIndex.
    let script = format!("pkg_mkIndex {{{}}} *.tcl", pkg_dir.display());
    assert!(run_tclsh(sh, &script).is_some(), "pkg_mkIndex failed");
    let index_path = pkg_dir.join("pkgIndex.tcl");
    let content = std::fs::read_to_string(&index_path).expect("read generated pkgIndex.tcl");

    let infos = parse_pkg_index(
        &content,
        &pkg_dir,
        &index_path,
        &|p| p.is_file(),
        &super::list_tcl_files,
    );
    assert_eq!(infos.len(), 1, "parsed {content:?}");
    assert_eq!(infos[0].name, "foo");
    assert_eq!(infos[0].version, "2.3");
    assert_eq!(infos[0].source_files, vec![pkg_dir.join("foo.tcl")]);
}

// ---------------------------------------------------------------------------
// tclIndex parsing — pure cases + differential against auto_mkindex.
// ---------------------------------------------------------------------------

#[test]
fn parse_tcl_index_v2_and_old_formats() {
    let dir = Path::new("/lib");
    // Version 2.0 with the `-encoding` flag and a namespaced + global key.
    let v2 = "# Tcl autoload index file, version 2.0\n\
              set auto_index(::http::geturl) [list source -encoding utf-8 [file join $dir http.tcl]]\n\
              set auto_index(topcmd) [list source [file join $dir top.tcl]]\n";
    let entries = parse_tcl_index(v2, dir, &always_exists);
    let map: std::collections::HashMap<_, _> = entries
        .iter()
        .map(|e| (e.proc_name.as_str(), e.source_file.clone()))
        .collect();
    assert_eq!(map["::http::geturl"], dir.join("http.tcl"));
    assert_eq!(map["topcmd"], dir.join("top.tcl"));

    // Old line-based format.
    let old = "# Tcl autoload index file: each line identifies a Tcl\n\
               myProc myfile.tcl\n\
               # a comment\n\
               other other.tcl\n";
    let entries = parse_tcl_index(old, dir, &always_exists);
    let map: std::collections::HashMap<_, _> = entries
        .iter()
        .map(|e| (e.proc_name.as_str(), e.source_file.clone()))
        .collect();
    assert_eq!(map["myProc"], dir.join("myfile.tcl"));
    assert_eq!(map["other"], dir.join("other.tcl"));
}

#[test]
fn parse_tcl_index_differential_against_auto_mkindex() {
    let Some(sh) = find_tclsh() else {
        eprintln!("skipping auto_mkindex differential: no tclsh on PATH");
        return;
    };
    let td = TempDir::new("automk");
    let dir = td.path();
    write(
        &dir.join("lib.tcl"),
        "proc ::myns::helper {} {}\nproc globalcmd {} {}\n",
    );
    let script = format!("auto_mkindex {{{}}} *.tcl", dir.display());
    assert!(run_tclsh(sh, &script).is_some(), "auto_mkindex failed");
    let content = std::fs::read_to_string(dir.join("tclIndex")).expect("read tclIndex");

    let entries = parse_tcl_index(&content, dir, &|p| p.is_file());
    let map: std::collections::HashMap<_, _> = entries
        .iter()
        .map(|e| (e.proc_name.as_str(), e.source_file.clone()))
        .collect();
    // C Tcl keys a global command with NO leading `::` and a namespaced one
    // with its full path (matching auto_qualify); both point at lib.tcl.
    assert_eq!(map.get("globalcmd"), Some(&dir.join("lib.tcl")));
    assert_eq!(map.get("::myns::helper"), Some(&dir.join("lib.tcl")));
}

// ---------------------------------------------------------------------------
// PackageResolver — search-path scanning + resolution.
// ---------------------------------------------------------------------------

#[test]
fn resolver_scans_immediate_subdirs_like_tcl_pkg_unknown() {
    // C Tcl scans `$dir/*/pkgIndex.tcl` as well as `$dir/pkgIndex.tcl`, so a
    // package in an immediate subdirectory is found from the parent path.
    let td = TempDir::new("scan");
    let root = td.path();
    let pkg_dir = root.join("mylib2.1");
    write(&pkg_dir.join("mylib.tcl"), "proc mylib::init {} {}\n");
    write(
        &pkg_dir.join("pkgIndex.tcl"),
        "package ifneeded mylib 2.1 [list source [file join $dir mylib.tcl]]\n",
    );

    let mut resolver = PackageResolver::new();
    resolver.scan_path(root);
    assert!(resolver.provides("mylib"));
    assert_eq!(
        resolver.resolve("mylib", None),
        vec![pkg_dir.join("mylib.tcl")]
    );
    assert_eq!(
        resolver.resolve("mylib", Some("2.1")),
        vec![pkg_dir.join("mylib.tcl")]
    );
    assert!(resolver.resolve("mylib", Some("9.9")).is_empty());
    assert!(resolver.resolve("nonexistent", None).is_empty());
}

/// A version constraint selects the release Tcl would load, with `package
/// vsatisfies` semantics — not the first-discovered one, and not a string
/// prefix.
///
/// Oracle (`tclsh8.6` 8.6.14 and `tclsh9.0` 9.0.4, two `pkgIndex.tcl`
/// directories declaring `widget` 1.5 and 2.3 on `auto_path`):
///
/// ```text
/// package vsatisfies 1.5 2.0     -> 0      package vsatisfies 2.9 2.0 -> 1
/// package vsatisfies 2.3 2.0     -> 1      package vsatisfies 3.0 2.0 -> 0
/// package vsatisfies 2.3 2.0-    -> 1      package vsatisfies 2.3 2.0-2.2 -> 0
/// package require widget 2.0     -> 2.3
/// ```
#[test]
fn resolve_picks_the_highest_release_satisfying_the_constraint() {
    let td = TempDir::new("vsat");
    let root = td.path();
    for (dir, ver, file) in [("v1", "1.5", "widget15.tcl"), ("v2", "2.3", "widget23.tcl")] {
        let pkg_dir = root.join(dir);
        write(&pkg_dir.join(file), "proc widget::init {} {}\n");
        write(
            &pkg_dir.join("pkgIndex.tcl"),
            &format!("package ifneeded widget {ver} [list source [file join $dir {file}]]\n"),
        );
    }
    let mut resolver = PackageResolver::new();
    resolver.scan_path(root);

    // TP: `2.0` means [2.0, 3) — 2.3 satisfies it, 1.5 does not.
    assert_eq!(
        resolver.resolve("widget", Some("2.0")),
        vec![root.join("v2").join("widget23.tcl")],
        "`package require widget 2.0` loads 2.3 in real tclsh",
    );
    // A constraint only 1.5 satisfies picks 1.5, whichever was scanned first.
    assert_eq!(
        resolver.resolve("widget", Some("1.0")),
        vec![root.join("v1").join("widget15.tcl")],
    );
    // Open-ended and ranged forms.
    assert_eq!(
        resolver.resolve("widget", Some("2.0-")),
        vec![root.join("v2").join("widget23.tcl")],
    );
    assert!(
        resolver.resolve("widget", Some("2.0-2.2")).is_empty(),
        "`2.0-2.2` excludes its upper bound, so neither release satisfies it",
    );
    // TN: a constraint no release satisfies resolves to nothing — never a
    // silent fallback to the first provider.
    assert!(resolver.resolve("widget", Some("9.9")).is_empty());
    assert!(resolver.resolve("widget", Some("3.0")).is_empty());

    // An **unconstrained** require picks the highest, not the first
    // discovered — `package require widget` loads 2.3 on 8.6.14 and 9.0.4
    // alike, whichever order the two directories were scanned in (#1090).
    assert_eq!(
        resolver.resolve("widget", None),
        vec![root.join("v2").join("widget23.tcl")],
    );
}

/// `package require -exact NAME VERSION` selects **that** release or nothing.
///
/// Oracle (`tclsh8.6` 8.6.14 / `tclsh9.0` 9.0.4, byte-identical; providers
/// registered as `package ifneeded widget V …` and then required):
///
/// ```text
/// avail {1.5 2.3}     -exact widget 2.0  -> can't find package widget exactly 2.0
/// avail {1.5 2.0 2.3} -exact widget 2.0  -> 2.0
/// avail {1.5 2.0 2.3}         widget 2.0 -> 2.3      (the ranged reading)
/// avail {2.0a1}       -exact widget 2.0  -> can't find package widget exactly 2.0
/// avail {2.0a1}               widget 2.0 -> 2.0a1
/// ```
///
/// Before #1090 the flag was parsed and dropped, so the first row resolved
/// 2.3 — a navigation jump into a release the script provably never loads.
#[test]
fn exact_require_selects_that_release_or_nothing() {
    let td = TempDir::new("exact");
    let root = td.path();
    for (dir, ver) in [("v1", "1.5"), ("v2", "2.0"), ("v3", "2.3"), ("v4", "2.0a1")] {
        let pkg_dir = root.join(dir);
        let file = format!("widget{dir}.tcl");
        write(&pkg_dir.join(&file), "proc widget::init {} {}\n");
        write(
            &pkg_dir.join("pkgIndex.tcl"),
            &format!("package ifneeded widget {ver} [list source [file join $dir {file}]]\n"),
        );
    }
    let impl_of = |dir: &str| vec![root.join(dir).join(format!("widget{dir}.tcl"))];

    let mut all = PackageResolver::new();
    all.scan_path(root);
    // TP — the exact release is present, so it is the one selected, where the
    // ranged reading of the same version would have taken 2.3.
    assert_eq!(
        all.resolve_require("widget", Some("2.0"), true, PackagePrefer::default()),
        impl_of("v2")
    );
    assert_eq!(
        all.resolve_require("widget", Some("2.0"), false, PackagePrefer::default()),
        impl_of("v3")
    );
    // TP — the trailing-zero spelling of the same release still satisfies
    // `-exact` (`package vcompare 2.0 2.0.0` is 0).
    assert_eq!(
        all.resolve_require("widget", Some("2.0.0"), true, PackagePrefer::default()),
        impl_of("v2"),
    );

    // FN guard — with 2.0 absent, `-exact 2.0` resolves *nothing* rather than
    // falling through to the next-highest release.
    let mut without_exact = PackageResolver::new();
    without_exact.scan_path(&root.join("v1"));
    without_exact.scan_path(&root.join("v3"));
    assert!(
        without_exact
            .resolve_require("widget", Some("2.0"), true, PackagePrefer::default())
            .is_empty(),
        "`-exact 2.0` must not resolve 2.3",
    );
    assert_eq!(
        without_exact.resolve_require("widget", Some("2.0"), false, PackagePrefer::default()),
        impl_of("v3"),
        "the ranged form still resolves 2.3, so the difference is the flag",
    );

    // FP guard — an alpha of the required release does not satisfy `-exact`,
    // though it does satisfy the ranged form.
    let mut alpha_only = PackageResolver::new();
    alpha_only.scan_path(&root.join("v4"));
    assert!(
        alpha_only
            .resolve_require("widget", Some("2.0"), true, PackagePrefer::default())
            .is_empty(),
    );
    assert_eq!(
        alpha_only.resolve_require("widget", Some("2.0"), false, PackagePrefer::default()),
        impl_of("v4"),
    );

    // TN — `-exact` with no version is `package require -exact NAME`, a
    // syntax error in real Tcl; treated as unconstrained, so it picks the
    // best release rather than nothing.
    assert_eq!(
        all.resolve_require("widget", None, true, PackagePrefer::default()),
        impl_of("v3")
    );
}

/// An unconstrained require picks the highest **stable** release, which is
/// `package prefer`'s default (`package prefer` answers `stable` on 8.6.14 and
/// 9.0.4 with no `TCL_PKG_PREFER_LATEST` in the environment).
///
/// Oracle: with 1.2 and 1.3b1 both on `auto_path`, `package require widget`
/// loads **1.2**; `package prefer latest` first makes it load 1.3b1. With
/// 1.3b1 the only provider, it loads 1.3b1 either way.
#[test]
fn unconstrained_require_prefers_the_highest_stable_release() {
    let td = TempDir::new("prefer");
    let root = td.path();
    for (dir, ver) in [("s", "1.2"), ("u", "1.3b1")] {
        let pkg_dir = root.join(dir);
        let file = format!("w{dir}.tcl");
        write(&pkg_dir.join(&file), "proc w {} {}\n");
        write(
            &pkg_dir.join("pkgIndex.tcl"),
            &format!("package ifneeded w {ver} [list source [file join $dir {file}]]\n"),
        );
    }
    let mut both = PackageResolver::new();
    both.scan_path(root);
    // TP — the prerelease is the numerically higher version but not the one
    // Tcl loads; picking `max` alone would have taken 1.3b1.
    assert_eq!(both.resolve("w", None), vec![root.join("s").join("ws.tcl")]);
    // …and a version constraint both satisfy resolves the same way.
    assert_eq!(
        both.resolve("w", Some("1.2")),
        vec![root.join("s").join("ws.tcl")],
    );

    // TN — with only the prerelease available there is nothing stable to
    // prefer, so it loads.
    let mut unstable_only = PackageResolver::new();
    unstable_only.scan_path(&root.join("u"));
    assert_eq!(
        unstable_only.resolve("w", None),
        vec![root.join("u").join("wu.tcl")],
    );
    assert_eq!(
        unstable_only.resolve("w", Some("1.2")),
        vec![root.join("u").join("wu.tcl")],
        "`package vsatisfies 1.3b1 1.2` is 1 — the prerelease is acceptable",
    );
}

/// `package prefer latest` flips the same corpus onto the prerelease
/// (issue #1126 item 1).
///
// tclsh-proof: tclsh8.6 (8.6.14), with both registered by hand —
//   package ifneeded widget 1.2   {package provide widget 1.2}
//   package ifneeded widget 1.3b1 {package provide widget 1.3b1}
//   package require widget          → 1.2
// and with `package prefer latest` evaluated first → 1.3b1.
// `package prefer` itself answers `stable` by default, `latest` after the
// raise, and a following `package prefer stable` returns `latest` with no
// error — the latch never falls back.
#[test]
fn prefer_latest_selects_the_prerelease() {
    let td = TempDir::new("preferlatest");
    let root = td.path();
    for (dir, ver) in [("s", "1.2"), ("u", "1.3b1")] {
        let pkg_dir = root.join(dir);
        let file = format!("w{dir}.tcl");
        write(&pkg_dir.join(&file), "proc w {} {}\n");
        write(
            &pkg_dir.join("pkgIndex.tcl"),
            &format!("package ifneeded w {ver} [list source [file join $dir {file}]]\n"),
        );
    }
    let mut both = PackageResolver::new();
    both.scan_path(root);
    // TP — the raise moves the answer onto the prerelease.
    assert_eq!(
        both.resolve_require("w", None, false, PackagePrefer::Latest),
        vec![root.join("u").join("wu.tcl")],
    );
    // FP guard — the default is untouched.
    assert_eq!(
        both.resolve_require("w", None, false, PackagePrefer::Stable),
        vec![root.join("s").join("ws.tcl")],
    );
    // TN — with no prerelease in the acceptable set the mode changes nothing.
    let mut stable_only = PackageResolver::new();
    stable_only.scan_path(&root.join("s"));
    assert_eq!(
        stable_only.resolve_require("w", None, false, PackagePrefer::Latest),
        stable_only.resolve_require("w", None, false, PackagePrefer::Stable),
    );
}

/// `package_prefer_at` reads the document's own raise, ordered against the
/// `package require` that asks (issue #1126 item 1).
#[test]
fn package_prefer_state_is_ordered_against_the_require() {
    use tcl_compiler::analyser::Analyser;
    let analyse = |src: &str| Analyser::new().analyse(src, "tcl");
    let at = |src: &str, needle: &str| {
        u32::try_from(src.find(needle).expect("needle")).expect("offset fits")
    };

    // TP — a raise above the require is in effect.
    let src = "package prefer latest\npackage require w\n";
    let analysis = analyse(src);
    assert_eq!(
        crate::package_resolver::package_prefer_at(&analysis, at(src, "package require")),
        PackagePrefer::Latest,
    );

    // FP guard — a raise written *below* a top-level require has not run yet.
    let src = "package require w\npackage prefer latest\n";
    let analysis = analyse(src);
    assert_eq!(
        crate::package_resolver::package_prefer_at(&analysis, at(src, "package require")),
        PackagePrefer::Stable,
    );

    // TP — …but a require inside a proc body *does* see a raise written
    // later at load level, because the whole file loads before any body runs
    // (the `in_effect` rule the import family already shares).
    let src = "proc load {} {\n    package require w\n}\npackage prefer latest\n";
    let analysis = analyse(src);
    assert_eq!(
        crate::package_resolver::package_prefer_at(&analysis, at(src, "    package require")),
        PackagePrefer::Latest,
    );

    // FP guard — a conditional raise is not taken as a fact.
    let src = "if {$::tcl_platform(platform) eq \"unix\"} {\n    package prefer latest\n}\npackage require w\n";
    let analysis = analyse(src);
    assert_eq!(
        crate::package_resolver::package_prefer_at(&analysis, at(src, "package require")),
        PackagePrefer::Stable,
    );

    // TN — `package prefer stable` and the query form change nothing; a
    // dynamic mode word is skipped rather than guessed at.
    for src in [
        "package prefer stable\npackage require w\n",
        "package prefer\npackage require w\n",
        "package prefer $mode\npackage require w\n",
    ] {
        let analysis = analyse(src);
        assert_eq!(
            crate::package_resolver::package_prefer_at(&analysis, at(src, "package require")),
            PackagePrefer::Stable,
            "{src}",
        );
    }
}

/// Two providers whose versions compare **equal** both contribute their files
/// (issue #1126 item 2).
///
// tclsh-proof: tclsh8.6 (8.6.14) —
//   package ifneeded w 1.0   {puts A}
//   package ifneeded w 1.0.0 {puts B}
//   package versions w   → 1.0        (one entry, the *first* version string)
//   package ifneeded w 1.0 → puts B   (the *last* script)
// The registration order is `glob` order, i.e. filesystem order, so which
// script survives is machine-dependent and cannot be pinned.
#[test]
fn equal_comparing_providers_all_contribute_their_files() {
    let td = TempDir::new("dupe");
    let root = td.path();
    for (dir, ver) in [("a", "1.0"), ("b", "1.0.0"), ("c", "0.9")] {
        let pkg_dir = root.join(dir);
        let file = format!("w{dir}.tcl");
        write(&pkg_dir.join(&file), "proc w {} {}\n");
        write(
            &pkg_dir.join("pkgIndex.tcl"),
            &format!("package ifneeded w {ver} [list source [file join $dir {file}]]\n"),
        );
    }
    let mut resolver = PackageResolver::new();
    resolver.scan_path(root);
    // TP — `1.0` and `1.0.0` are one release to `package vcompare`, so both
    // copies are indexed; discovery order (sorted by directory name) puts the
    // one whose version string C Tcl keeps first.
    assert_eq!(
        resolver.resolve("w", None),
        vec![root.join("a").join("wa.tcl"), root.join("b").join("wb.tcl"),],
    );
    // FP guard — a *lower* release is not dragged in with them.
    assert_eq!(
        resolver
            .select_provider("w", None, false, PackagePrefer::default())
            .map(|i| i.version.as_str()),
        Some("1.0"),
    );
    // FP guard — a constraint that only the lower release satisfies still
    // answers just that one.
    assert_eq!(
        resolver.resolve_require("w", Some("0.9"), true, PackagePrefer::default()),
        vec![root.join("c").join("wc.tcl")],
    );
}

/// `select_provider` reports the whole chosen declaration, not just its files
/// — the version actually selected and the `pkgIndex.tcl` that declared it.
#[test]
fn select_provider_reports_the_chosen_declaration() {
    let td = TempDir::new("chosen");
    let root = td.path();
    for (dir, ver) in [("a", "1.5"), ("b", "2.3")] {
        let pkg_dir = root.join(dir);
        write(&pkg_dir.join("impl.tcl"), "proc p {} {}\n");
        write(
            &pkg_dir.join("pkgIndex.tcl"),
            &format!("package ifneeded p {ver} [list source [file join $dir impl.tcl]]\n"),
        );
    }
    let mut resolver = PackageResolver::new();
    resolver.scan_path(root);
    let chosen = resolver
        .select_provider("p", None, false, PackagePrefer::default())
        .expect("a provider");
    assert_eq!(chosen.version, "2.3");
    assert_eq!(chosen.pkg_index_path, root.join("b").join("pkgIndex.tcl"));
    // TN — nothing acceptable, and an unknown package, both answer `None`.
    assert!(
        resolver
            .select_provider("p", Some("9.9"), false, PackagePrefer::default())
            .is_none()
    );
    assert!(
        resolver
            .select_provider("absent", None, false, PackagePrefer::default())
            .is_none()
    );
}

#[test]
fn resolver_auto_command_uses_auto_qualify_candidates() {
    let td = TempDir::new("autocmd");
    let dir = td.path();
    write(&dir.join("h.tcl"), "proc ::http::geturl {u} {}\n");
    write(
        &dir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(::http::geturl) [list source [file join $dir h.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(dir);
    // `geturl` used inside namespace `::http` qualifies to `::http::geturl`.
    assert_eq!(
        resolver.resolve_auto_command("geturl", "::http"),
        vec![dir.join("h.tcl")]
    );
    // A bare global `nope` resolves to nothing.
    assert!(resolver.resolve_auto_command("nope", "::").is_empty());
}

#[test]
fn package_requires_in_collects_require_and_provide_names() {
    let content = "package require Tk\n\
                   package require -exact Tcl 8.6\n\
                   package provide myTkPackage 1.0\n\
                   package require $dynamic\n\
                   set x 1\n";
    let mut names = package_requires_in(content);
    names.sort();
    assert_eq!(names, vec!["Tcl", "Tk", "myTkPackage"]);
}

#[test]
fn transitive_closure_pulls_in_tk_through_a_wrapper_package() {
    // Regression model for #723: `package require myTkPackage`, whose
    // implementation does `package require Tk`, makes Tk transitively
    // available — exactly what C Tcl's ifneeded script would do.
    let td = TempDir::new("trans");
    let pkg_dir = td.path().join("mytk1.0");
    write(
        &pkg_dir.join("mytk.tcl"),
        "package provide myTkPackage 1.0\npackage require Tk\nproc mytk::go {} {}\n",
    );
    write(
        &pkg_dir.join("pkgIndex.tcl"),
        "package ifneeded myTkPackage 1.0 [list source [file join $dir mytk.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(td.path());

    let available = resolver.transitive_available_packages(&["myTkPackage".to_owned()], &|p| {
        std::fs::read_to_string(p).ok()
    });
    assert!(available.contains("myTkPackage"));
    assert!(
        available.contains("Tk"),
        "Tk must be transitively available: {available:?}"
    );

    // A package that does NOT pull in Tk leaves Tk unavailable.
    let other = td.path().join("plain1.0");
    write(
        &other.join("plain.tcl"),
        "package provide plain 1.0\nproc p {} {}\n",
    );
    write(
        &other.join("pkgIndex.tcl"),
        "package ifneeded plain 1.0 [list source [file join $dir plain.tcl]]\n",
    );
    let mut resolver2 = PackageResolver::new();
    resolver2.scan_path(td.path());
    let avail2 = resolver2
        .transitive_available_packages(&["plain".to_owned()], &|p| std::fs::read_to_string(p).ok());
    assert!(avail2.contains("plain"));
    assert!(
        !avail2.contains("Tk"),
        "plain must not provide Tk: {avail2:?}"
    );
}

/// Two providers declaring versions that compare *equal*: the first scanned
/// is the selected declaration, deterministically — while the *files* are the
/// union of both (issue #1126 item 2, and
/// [`super::PackageResolver::resolve_require`]'s doc).
///
/// Real Tcl collapses them into one `package ifneeded` entry — first
/// registration's version string, last registration's script — and the
/// registration order is `glob` order, i.e. filesystem order, which differs
/// per machine (verified: on this container `glob -directory … -join *
/// pkgIndex.tcl` returned `b` before `a`). There is no stable oracle for
/// *which script survives*, so navigation reads both copies rather than
/// betting; the deterministic half — which declaration is "the" one, and so
/// which version string is reported — stays discovery order, sorted by name.
#[test]
fn resolver_first_provider_wins() {
    // Two providers of the same package: the first scanned keeps the head.
    let td = TempDir::new("dup");
    let first = td.path().join("a");
    let second = td.path().join("b");
    write(&first.join("dup.tcl"), "proc dup {} {}\n");
    write(
        &first.join("pkgIndex.tcl"),
        "package ifneeded dup 1.0 [list source [file join $dir dup.tcl]]\n",
    );
    write(&second.join("dup.tcl"), "proc dup {} {}\n");
    write(
        &second.join("pkgIndex.tcl"),
        "package ifneeded dup 1.0 [list source [file join $dir dup.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(&first);
    resolver.scan_path(&second);
    assert_eq!(
        resolver
            .select_provider("dup", None, false, PackagePrefer::default())
            .map(|i| i.pkg_index_path.clone()),
        Some(first.join("pkgIndex.tcl")),
        "the first-scanned declaration is the one C Tcl reports the version of",
    );
    assert_eq!(
        resolver.resolve("dup", None),
        vec![first.join("dup.tcl"), second.join("dup.tcl")],
        "…and both copies are indexed, because which script survives is a \
         filesystem-order fact",
    );
}

// ---------------------------------------------------------------------------
// `auto_loads_command` — the W123 "command defined in library path" oracle
// (issue #832).  A library that ships a `tclIndex` makes its procs auto-loadable
// by bare name with no `package require`, exactly the Rbc_* / BLT idiom in the
// issue.  These pin the TP / FP / TN / FN arms of that resolvability check.
// ---------------------------------------------------------------------------

/// TN control — the resolvability oracle answers *yes* for a command the scanned
/// `tclIndex` genuinely provides, so the W123 that would fire on it is a true
/// negative (correctly not flagged).
#[test]
fn auto_loads_command_true_for_tclindex_global_proc() {
    let td = TempDir::new("autoloads-tn");
    let dir = td.path();
    // A BLT/Rbc-style library dir: a global proc registered by an auto_mkindex
    // `tclIndex` (global names stored without a leading `::`, per auto_qualify).
    write(&dir.join("graph.tcl"), "proc Rbc_ActiveLegend {graph} {}\n");
    write(
        &dir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(dir);
    assert!(
        resolver.auto_loads_command("Rbc_ActiveLegend", "::"),
        "a global proc in a scanned tclIndex must be reported auto-loadable"
    );
}

/// TP control — a genuinely-unknown command (no index maps it) must *not* be
/// reported resolvable, so its W123 correctly stands (a true positive).
#[test]
fn auto_loads_command_false_for_unknown_name() {
    let td = TempDir::new("autoloads-tp");
    let dir = td.path();
    write(&dir.join("graph.tcl"), "proc Rbc_ActiveLegend {graph} {}\n");
    write(
        &dir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(dir);
    // A typo of the real command is not in the index → not resolvable.
    assert!(
        !resolver.auto_loads_command("Rbc_ActveLegend", "::"),
        "a name no tclIndex declares must not be reported auto-loadable"
    );
    // An empty resolver knows nothing.
    assert!(!PackageResolver::new().auto_loads_command("Rbc_ActiveLegend", "::"));
}

/// A namespaced auto-load key resolves for a bare call inside that namespace but
/// not for the same tail in the global namespace — the `auto_qualify` candidate
/// order is honoured, not a blanket tail match.
#[test]
fn auto_loads_command_respects_namespace_qualification() {
    let td = TempDir::new("autoloads-ns");
    let dir = td.path();
    write(&dir.join("h.tcl"), "proc ::http::geturl {u} {}\n");
    write(
        &dir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(::http::geturl) [list source [file join $dir h.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(dir);
    assert!(resolver.auto_loads_command("geturl", "::http"));
    assert!(!resolver.auto_loads_command("geturl", "::"));
}

// ---------------------------------------------------------------------------
// `package_defined_commands` — the secondary W123 oracle for a `pkgIndex`-only
// package (no `tclIndex`) whose implementation source files define the command.
// The extractor is injected; here a stub stands in for the analyser.
// ---------------------------------------------------------------------------

#[test]
fn package_defined_commands_unions_available_package_sources() {
    let td = TempDir::new("pkgcmds");
    let dir = td.path();
    write(&dir.join("mylib.tcl"), "proc mylib::draw {} {}\n");
    write(
        &dir.join("pkgIndex.tcl"),
        "package ifneeded mylib 1.0 [list source [file join $dir mylib.tcl]]\n",
    );
    let mut resolver = PackageResolver::new();
    resolver.scan_path(dir);
    // Stub extractor: report the tail `draw` for the resolved implementation
    // file (standing in for the registry-driven analyser extraction).
    let extract = |path: &Path| -> Vec<String> {
        if path.file_name().and_then(|n| n.to_str()) == Some("mylib.tcl") {
            vec!["draw".to_owned()]
        } else {
            Vec::new()
        }
    };
    let cmds = resolver.package_defined_commands(&["mylib".to_owned()], None, &extract);
    assert!(
        cmds.contains("draw"),
        "available package's command surfaced"
    );
    // A package the paths don't know contributes nothing (no source files).
    let none = resolver.package_defined_commands(&["absent".to_owned()], None, &extract);
    assert!(none.is_empty());
    // No available packages ⇒ empty, extractor never consulted.
    let empty = resolver.package_defined_commands(&[], None, &extract);
    assert!(empty.is_empty());
}
