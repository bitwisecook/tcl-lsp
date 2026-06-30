//! Unit tests plus **differential tests against a real `tclsh`** for the
//! package / auto-load resolver.
//!
//! The differential tests generate genuine `pkgIndex.tcl` / `tclIndex` files
//! with C Tcl's own `pkg_mkIndex` / `auto_mkindex`, and query C Tcl's own
//! `auto_qualify`, then assert the Rust port produces the same result. They
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
    assert_eq!(resolver.resolve("dup", None), vec![first.join("dup.tcl")]);
}
