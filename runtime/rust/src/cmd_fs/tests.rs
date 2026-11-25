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

use super::file_permissions;
use crate::interp::{Code, Interp};
#[cfg(unix)]
use crate::obj;
use tcl_dialect::TclVersion;

#[test]
fn file_worker_runtime_headers_match_all_190_original_native_snapshots() {
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
    let fixtures = [
        (
            TclVersion::V8_4,
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_file_compilers/8.4.20.tsv"
            ),
        ),
        (
            TclVersion::V8_5,
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_file_compilers/8.5.19.tsv"
            ),
        ),
        (
            TclVersion::V8_6,
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_file_compilers/8.6.18.tsv"
            ),
        ),
        (
            TclVersion::V9_0,
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_file_compilers/9.0.4.tsv"
            ),
        ),
        (
            TclVersion::V9_1,
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_file_compilers/9.1.0.tsv"
            ),
        ),
    ];
    let mut checked = 0;
    for (version, fixture) in fixtures {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        for row in fixture.lines().skip(1) {
            let (identity, expected) = tcl_test_support::fixture::tsv_split_once(row).unwrap();
            let namespaces = interp.namespaces();
            let actual = match namespaces
                .resolve_generation(crate::namespace::GLOBAL, identity.as_bytes())
            {
                None => -1,
                Some(generation) => match namespaces.native_compiler_hook(generation) {
                    Some(Hook::Absent) => 0,
                    Some(Hook::Present) => 1,
                    value => panic!(
                        "unknown original file registration: {version:?}/{identity}: {value:?}"
                    ),
                },
            };
            assert_eq!(
                actual,
                expected.parse::<i32>().unwrap(),
                "{version:?}/{identity}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 190);
}

/// `file` on names the releases read apart, as tclsh 8.4.20, 8.5.19,
/// 8.6.18 and 9.0.4 (9.1.0 as 9.0.4) answer: a leading `//` (#2420), a
/// `~` (#2421), the zipfs volume and 8.5's as-given `join`. An error
/// reads `ERR: message`.
const RELEASE_PATHS: &[(&str, [&str; 4])] = &[
    ("split //a", ["/ a", "/ a", "/ a", "//a"]),
    ("dirname //a", ["/", "/", "/", "//a"]),
    ("tail //a", ["a", "a", "a", ""]),
    ("join //h b", ["/h/b", "/h/b", "/h/b", "//h/b"]),
    ("join a //b", ["/b", "//b", "/b", "//b"]),
    ("split a/~b", ["a ./~b", "a ./~b", "a ./~b", "a ~b"]),
    ("join a ~b", ["~b", "~b", "~b", "a/~b"]),
    ("tail a/~b", ["./~b", "./~b", "./~b", "~b"]),
    ("dirname ~b/c", ["~b", "~b", "~b", "~b"]),
    (
        "pathtype ~b",
        ["absolute", "absolute", "absolute", "relative"],
    ),
    ("join /./~b", ["/~b", "/./~b", "/./~b", "/./~b"]),
    (
        "split //zipfs:/x/y",
        [
            "/ zipfs: x y",
            "/ zipfs: x y",
            "/ zipfs: x y",
            "//zipfs:/ x y",
        ],
    ),
    (
        "pathtype //zipfsxy",
        ["absolute", "absolute", "absolute", "volumerelative"],
    ),
    (
        "nativename //zipfsxy",
        ["/zipfsxy", "//zipfsxy", "/zipfsxy", "//zipfs:y"],
    ),
    (
        "normalize /./no_such_dir_x",
        [
            "/no_such_dir_x",
            "//no_such_dir_x",
            "//no_such_dir_x",
            "//no_such_dir_x",
        ],
    ),
    (
        "normalize //no_such_dir_x/b/..",
        [
            "/no_such_dir_x",
            "//no_such_dir_x",
            "/no_such_dir_x",
            "//no_such_dir_x",
        ],
    ),
    (
        "dirname ~no_such_user_x",
        [
            "ERR: user \"no_such_user_x\" doesn't exist",
            "ERR: user \"no_such_user_x\" doesn't exist",
            "ERR: user \"no_such_user_x\" doesn't exist",
            ".",
        ],
    ),
];

#[test]
fn path_names_read_as_the_emulated_release_reads_them() {
    for (column, release) in TclVersion::ALL.into_iter().enumerate() {
        let column = column.min(3);
        let mut i = Interp::new();
        i.set_runtime_version(release);
        for &(call, answers) in RELEASE_PATHS {
            let script = format!("file {call}");
            let got = match i.eval_str(script.as_bytes()) {
                Code::Ok => String::from_utf8_lossy(&i.result_bytes()).into_owned(),
                _ => format!("ERR: {}", String::from_utf8_lossy(&i.result_bytes())),
            };
            assert_eq!(got, answers[column], "{release:?} {script}");
        }
    }
}

fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(
        i.eval_str(src),
        Code::Ok,
        "eval {:?}: {}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes())
    );
    i.result_bytes()
}

/// `file`'s ensemble miss sentence — the
/// `unknown or ambiguous subcommand "` prefix and the comma-before-`or`
/// join — is `tcl_cmd_core::ensemble`'s, not this module's. The visible
/// name set still comes from the registry, so it stays release-gated.
///
/// tclsh 9.0.4:
///   file {}   -> unknown or ambiguous subcommand "": must be atime,
///                attributes, channels, copy, delete, dirname, executable,
///                exists, extension, home, isdirectory, isfile, join,
///                link, lstat, mkdir, mtime, nativename, normalize, owned,
///                pathtype, readable, readlink, rename, rootname,
///                separator, size, split, stat, system, tail, tempdir,
///                tempfile, tildeexpand, type, volumes, or writable
///   file ex / -> unknown or ambiguous subcommand "ex": must be <same>
///                (exists/executable/extension all start with `ex`)
///   file ext /a.b -> .b   (a unique prefix still resolves)
#[test]
fn file_ensemble_miss_carries_the_full_option_list() {
    const MUST: &str = "must be atime, attributes, channels, copy, delete, dirname, \
                            executable, exists, extension, home, isdirectory, isfile, join, \
                            link, lstat, mkdir, mtime, nativename, normalize, owned, pathtype, \
                            readable, readlink, rename, rootname, separator, size, split, \
                            stat, system, tail, tempdir, tempfile, tildeexpand, type, volumes, \
                            or writable";
    let mut i = Interp::new();
    assert_eq!(i.eval_str(b"file {}"), Code::Error);
    assert_eq!(
        i.result_bytes(),
        format!("unknown or ambiguous subcommand \"\": {MUST}").as_bytes()
    );
    assert_eq!(i.eval_str(b"file ex /"), Code::Error);
    assert_eq!(
        i.result_bytes(),
        format!("unknown or ambiguous subcommand \"ex\": {MUST}").as_bytes()
    );
    assert_eq!(ok(&mut i, b"file ext /a.b"), b".b");
}

#[test]
fn file_mkdir_size_type_delete() {
    let mut i = Interp::new();
    let d = format!("/tmp/rtfs_{}", std::process::id());
    ok(&mut i, format!("file delete -force {d}").as_bytes());
    ok(&mut i, format!("file mkdir {d}/sub").as_bytes());
    assert_eq!(
        ok(&mut i, format!("file isdirectory {d}/sub").as_bytes()),
        b"1"
    );
    assert_eq!(
        ok(&mut i, format!("file type {d}").as_bytes()),
        b"directory"
    );
    assert_eq!(ok(&mut i, b"file pathtype /x"), b"absolute");
    assert_eq!(ok(&mut i, b"file pathtype rel"), b"relative");
    // Deleting a missing path is a silent no-op; size of a missing path errors.
    ok(&mut i, format!("file delete {d}/nope").as_bytes());
    assert_eq!(
        i.eval_str(format!("file size {d}/nope").as_bytes()),
        Code::Error
    );
    ok(&mut i, format!("file delete -force {d}").as_bytes());
    assert_eq!(ok(&mut i, format!("file exists {d}").as_bytes()), b"0");
}

#[test]
fn file_path_ops() {
    let mut i = Interp::new();
    assert_eq!(ok(&mut i, b"file dirname /a/b/c"), b"/a/b");
    assert_eq!(ok(&mut i, b"file dirname a"), b".");
    assert_eq!(ok(&mut i, b"file dirname /a"), b"/");
    assert_eq!(ok(&mut i, b"file tail /a/b/c"), b"c");
    assert_eq!(ok(&mut i, b"file tail c"), b"c");
    assert_eq!(ok(&mut i, b"file join /a b c"), b"/a/b/c");
    assert_eq!(ok(&mut i, b"file join a /b"), b"/b");
    assert_eq!(ok(&mut i, b"file extension foo.tcl"), b".tcl");
    assert_eq!(ok(&mut i, b"file rootname /x/foo.tcl"), b"/x/foo");
    assert_eq!(ok(&mut i, b"file split /a/b/c"), b"/ a b c");
    assert_eq!(ok(&mut i, b"file normalize /a/./b/../c"), b"/a/c");
}

#[test]
fn file_surface_and_permissions_follow_the_runtime_profile() {
    let mut i = Interp::new();
    i.set_runtime_version(TclVersion::V8_6);
    assert_eq!(i.eval_str(b"file tempdir"), Code::Error);
    let old_error = i.result_bytes();
    assert_eq!(
        old_error
            .windows(b"tempdir".len())
            .filter(|w| *w == b"tempdir")
            .count(),
        1,
        "8.6 visible subcommands must exclude tempdir: {}",
        String::from_utf8_lossy(&old_error)
    );
    assert_eq!(file_permissions(&i, 0o100755), b"0100755");

    i.set_runtime_version(TclVersion::V9_0);
    assert_eq!(file_permissions(&i, 0o100755), b"0o100755");
}

#[test]
fn tempdir_template_honours_its_parent_and_requires_it_to_exist() {
    let mut i = Interp::new();
    let root = std::env::temp_dir().join(format!("tclrt_tempdir_{}", std::process::id()));
    let missing = root.join("missing").join("prefix");
    assert_eq!(
        i.eval_str(format!("file tempdir {}/", missing.display()).as_bytes()),
        Code::Error
    );
    assert_eq!(
        i.result_bytes(),
        b"can't create temporary directory: no such file or directory"
    );

    std::fs::create_dir_all(&root).unwrap();
    let created = ok(
        &mut i,
        format!("file tempdir {}/gorp", root.display()).as_bytes(),
    );
    let created = std::path::PathBuf::from(String::from_utf8(created).unwrap());
    assert_eq!(created.parent(), Some(root.as_path()));
    assert!(created
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("gorp_"));
    std::fs::remove_dir(&created).unwrap();
    std::fs::remove_dir(&root).unwrap();
}

#[test]
fn file_extended_subcommands_cover_native_host_surface() {
    let mut i = Interp::new();
    // These pure arms otherwise default a missing operand to an empty
    // path; the runtime arity guard must reject them like Tcl does.
    assert_eq!(i.eval_str(b"file join"), Code::Error);
    assert_eq!(i.eval_str(b"file dirname"), Code::Error);
    let root = std::env::temp_dir().join(format!("tclrt_file_{}", std::process::id()));
    let src = root.join("src.txt");
    let copy = root.join("copy.txt");
    let moved = root.join("moved.txt");
    let src2 = root.join("src2.txt");
    let copied_dir = root.join("copied");
    let moved_dir = root.join("moved_dir");
    let link = root.join("link.txt");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&src, b"hello").unwrap();
    std::fs::write(&src2, b"world").unwrap();
    let s = |p: &std::path::Path| p.to_string_lossy().into_owned();
    let src_s = s(&src);
    let copy_s = s(&copy);
    let moved_s = s(&moved);
    let link_s = s(&link);
    assert_eq!(
        ok(&mut i, format!("file system {src_s}").as_bytes()),
        b"native"
    );
    assert_eq!(ok(&mut i, format!("file owned {src_s}").as_bytes()), b"1");
    assert!(
        String::from_utf8(ok(&mut i, format!("file atime {src_s}").as_bytes()))
            .unwrap()
            .parse::<i64>()
            .is_ok()
    );
    assert!(
        String::from_utf8(ok(&mut i, format!("file mtime {src_s}").as_bytes()))
            .unwrap()
            .parse::<i64>()
            .is_ok()
    );
    assert!(ok(&mut i, format!("file attributes {src_s}").as_bytes()).contains(&b'-'));
    assert_eq!(
        ok(&mut i, format!("file copy {src_s} {copy_s}").as_bytes()),
        b""
    );
    assert_eq!(
        ok(&mut i, format!("file rename {copy_s} {moved_s}").as_bytes()),
        b""
    );
    ok(
        &mut i,
        format!("file mkdir {}", copied_dir.display()).as_bytes(),
    );
    ok(
        &mut i,
        format!(
            "file copy {src_s} {} {}",
            src2.display(),
            copied_dir.display()
        )
        .as_bytes(),
    );
    assert_eq!(
        ok(
            &mut i,
            format!("file exists {}/src2.txt", copied_dir.display()).as_bytes()
        ),
        b"1"
    );
    ok(
        &mut i,
        format!("file mkdir {}", moved_dir.display()).as_bytes(),
    );
    ok(
        &mut i,
        format!(
            "file rename {}/src.txt {}/",
            copied_dir.display(),
            moved_dir.display()
        )
        .as_bytes(),
    );
    assert_eq!(
        ok(
            &mut i,
            format!("file exists {}/src.txt", moved_dir.display()).as_bytes()
        ),
        b"1"
    );
    assert_eq!(
        ok(
            &mut i,
            format!("file link -symbolic {link_s} {src_s}").as_bytes()
        ),
        src_s.as_bytes()
    );
    assert_eq!(
        ok(&mut i, format!("file readlink {link_s}").as_bytes()),
        src_s.as_bytes()
    );
    assert_eq!(
        ok(
            &mut i,
            format!("file stat {src_s} st; set st(size)").as_bytes()
        ),
        b"5"
    );
    assert_eq!(
        ok(
            &mut i,
            format!("file lstat {link_s} lst; set lst(type)").as_bytes()
        ),
        b"link"
    );
    assert_eq!(ok(&mut i, b"file tildeexpand plain"), b"plain");
    let home = ok(&mut i, b"file home");
    assert!(!home.is_empty());
    let tempdir = ok(&mut i, b"file tempdir tclrt-test");
    assert_eq!(
        ok(
            &mut i,
            format!("file isdirectory {}", String::from_utf8_lossy(&tempdir)).as_bytes()
        ),
        b"1"
    );
    let tempfile = ok(&mut i, b"file tempfile tmpname tclrt-test");
    assert!(String::from_utf8_lossy(&tempfile).starts_with("file"));
    assert!(!ok(&mut i, b"file channels").is_empty());
    ok(
        &mut i,
        format!("close {}", String::from_utf8_lossy(&tempfile)).as_bytes(),
    );
    assert!(!ok(&mut i, b"file volumes").is_empty());
    ok(
        &mut i,
        format!("file delete -force {}", root.display()).as_bytes(),
    );
    ok(
        &mut i,
        format!("file delete -force {}", String::from_utf8_lossy(&tempdir)).as_bytes(),
    );
}

#[test]
fn source_evaluates_a_file() {
    let mut i = Interp::new();
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tclrt_src_{}.tcl", std::process::id()));
    std::fs::write(&path, b"set sourced 42\nreturn done\n").unwrap();
    let cmd = format!("source {}", path.display());
    assert_eq!(ok(&mut i, cmd.as_bytes()), b"done"); // top-level return → ok
    assert_eq!(ok(&mut i, b"set sourced"), b"42");
    std::fs::remove_file(&path).ok();
}

#[test]
fn source_uses_actual_argument_and_return_protocols() {
    let jim = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    )));
    let path =
        std::env::temp_dir().join(format!("tclrt_source_protocol_{}.tcl", std::process::id()));
    std::fs::write(&path, b"return -level 2 -code error BOOM\n").unwrap();
    for (profile, expected_code) in [
        (
            crate::environment::profile_for_dialect("tcl8.6"),
            b"2".as_slice(),
        ),
        (jim, b"0".as_slice()),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(profile);
        let script = format!("catch {{source {}}} result", path.display());
        assert_eq!(ok(&mut interp, script.as_bytes()), expected_code);
        assert_eq!(ok(&mut interp, b"set result"), b"BOOM");
        let invalid = format!("catch {{source -encoding utf-8 {}}}", path.display());
        if std::ptr::eq(profile, jim) {
            assert_eq!(ok(&mut interp, invalid.as_bytes()), b"1");
        }
    }
    std::fs::remove_file(&path).unwrap();
}

#[cfg(unix)]
struct GlobTypeFixture(std::path::PathBuf);

#[cfg(unix)]
impl Drop for GlobTypeFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[cfg(unix)]
fn glob_type_fixture(label: &str) -> (Interp, GlobTypeFixture) {
    use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
    let directory =
        std::env::temp_dir().join(format!("tclrt_glob_types_{}_{label}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let fixture = GlobTypeFixture(directory);
    for (name, mode) in [
        ("plain.txt", 0o600),
        ("executable.txt", 0o700),
        ("readonly.txt", 0o400),
    ] {
        let path = fixture.0.join(name);
        std::fs::write(&path, b"ordinary fixture\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    let child = fixture.0.join("directory");
    std::fs::create_dir(&child).unwrap();
    std::fs::set_permissions(child, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let path = obj::Owned::fresh(obj::new_string_bytes(fixture.0.as_os_str().as_bytes()));
    interp.var_set(b"fixture", path.as_ptr()).unwrap();
    (interp, fixture)
}

#[cfg(unix)]
fn assert_glob_type_row(interp: &mut Interp, capture: &str, label: &str, types: &[u8]) {
    let prefix = format!("ROW {label} ");
    let row = capture
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap();
    let fields = tcl_syntax::list::split_native_list_bytes(
        row.as_bytes(),
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
    )
    .unwrap();
    assert_eq!(fields[2].as_ref(), b"0");
    let selected = obj::Owned::fresh(obj::new_string_bytes(types));
    interp.var_set(b"types", selected.as_ptr()).unwrap();
    assert_eq!(
        interp.eval_str(b"lsort [glob -nocomplain -tails -directory $fixture -types $types -- *]"),
        Code::Ok,
        "{label}: {:?}",
        interp.result_bytes()
    );
    assert_eq!(interp.result_bytes(), fields[3].as_ref(), "{label}");
}

#[cfg(unix)]
const GLOB_TYPE_C86_ROWS: &str = include_str!(
    "../../../../experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-question/capture-01/8.6.18/stdout.txt"
);

#[cfg(unix)]
#[test]
fn glob_type_regular_file_directory_unions_match_original_c86_rows() {
    let (mut interp, _fixture) = glob_type_fixture("ordinary_union");
    for (label, types) in [
        ("file", b"f".as_slice()),
        ("directory", b"d".as_slice()),
        ("file-directory", b"f d".as_slice()),
        ("directory-file", b"d f".as_slice()),
        ("duplicated-union", b"f d f".as_slice()),
    ] {
        assert_glob_type_row(&mut interp, GLOB_TYPE_C86_ROWS, label, types);
    }
}

#[cfg(unix)]
#[test]
fn glob_type_original_tcltest_selector_keeps_supported_kind_alternatives() {
    let (mut interp, _fixture) = glob_type_fixture("tcltest_union");
    for (label, types) in [
        ("union-with-unrepresented", b"b c f p s".as_slice()),
        ("directory-with-unrepresented", b"b c d p s".as_slice()),
        ("unrepresented-only", b"b c p s".as_slice()),
    ] {
        assert_glob_type_row(&mut interp, GLOB_TYPE_C86_ROWS, label, types);
    }
}

#[cfg(unix)]
#[test]
fn glob_type_kind_union_still_requires_executable_qualification() {
    let (mut interp, _fixture) = glob_type_fixture("qualified_union");
    for (label, types) in [
        ("file-executable", b"f x".as_slice()),
        ("union-executable", b"f d x".as_slice()),
    ] {
        assert_glob_type_row(&mut interp, GLOB_TYPE_C86_ROWS, label, types);
    }
}

#[cfg(unix)]
#[test]
fn glob_type_readable_writable_executable_qualifiers_remain_conjunctive() {
    let (mut interp, _fixture) = glob_type_fixture("rwx_qualifiers");
    let capture = include_str!(
        "../../../../experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-rwx-question/capture-01/8.6.18/stdout.txt"
    );
    assert_glob_type_row(
        &mut interp,
        capture,
        "readable-writable-executable",
        b"r w x",
    );
}

#[test]
fn glob_finds_files() {
    let mut i = Interp::new();
    let dir = std::env::temp_dir().join(format!("tclrt_glob_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.tcl"), b"").unwrap();
    std::fs::write(dir.join("b.txt"), b"").unwrap();
    let cmd = format!("glob -nocomplain -tails -directory {} *.tcl", dir.display());
    assert_eq!(ok(&mut i, cmd.as_bytes()), b"a.tcl");
    let none = format!("glob -nocomplain -directory {} *.zzz", dir.display());
    assert_eq!(ok(&mut i, none.as_bytes()), b"");
    std::fs::remove_dir_all(&dir).ok();
}

/// `glob`'s option words are a
/// `Tcl_GetIndexFromObj(…, "option", 0)` table (`globOptions[]`,
/// `tclFileName.c`). This scan already rejected an unknown option exactly
/// as C does — it just matched every name exactly, so nothing abbreviated
/// and the lone `-` was `bad` rather than `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   glob -x a  -> bad option "-x": must be -directory, -join, -nocomplain,
///                 -path, -tails, -types, or --
///   glob - a   -> ambiguous option "-": must be <same>
///   glob -t …  -> ambiguous option "-t": must be <same>  (-tails/-types)
///   glob {}    -> .    (the empty word is a pattern, not an option)
///   glob -n /nonexistent* -> {}      (`-n` → -nocomplain)
///   glob -nocomplain -d <dir> -ta *.tcl -> the tails    (-d, -ta)
#[test]
fn glob_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -directory, -join, -nocomplain, -path, -tails, -types, or --";
    let mut i = Interp::new();
    let err_of = |i: &mut Interp, src: &[u8]| {
        assert_eq!(i.eval_str(src), Code::Error, "expected an error");
        String::from_utf8_lossy(&i.result_bytes()).into_owned()
    };
    assert_eq!(
        err_of(&mut i, b"glob -d"),
        r#"missing argument to "-directory""#
    );
    assert_eq!(
        err_of(&mut i, b"glob -types bogus /dev/null"),
        r#"bad argument to "-types": bogus"#
    );
    assert_eq!(
        err_of(&mut i, b"glob -types [format %c 123] /dev/null"),
        "unmatched open brace in list"
    );
    // Only a `-`-leading word reaches the table.
    assert_eq!(
        ok(
            &mut i,
            b"set r [glob -nocomplain {}]; list [llength $r] [lindex $r 0]"
        ),
        b"1 ."
    );
    // Unique prefixes resolve: `-n` is `-nocomplain`.
    assert_eq!(ok(&mut i, b"glob -n /nonexistent-zz/*"), b"");
    let dir = std::env::temp_dir().join(format!("tclrt_globopt_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.tcl"), b"").unwrap();
    // `-d` is `-directory`, `-ta` is `-tails`, `-ty` is `-types`.
    let cmd = format!("glob -n -d {} -ta -ty f *.tcl", dir.display());
    assert_eq!(ok(&mut i, cmd.as_bytes()), b"a.tcl");
    assert_eq!(
        err_of(&mut i, b"glob -x a"),
        format!("bad option \"-x\": {MUST}")
    );
    assert_eq!(
        err_of(&mut i, b"glob - a"),
        format!("ambiguous option \"-\": {MUST}")
    );
    // `-t` prefixes both `-tails` and `-types`.
    assert_eq!(
        err_of(&mut i, b"glob -t d a"),
        format!("ambiguous option \"-t\": {MUST}")
    );
    let brace = format!(r"glob -d {} -ta -- {{{{a.tcl,a.tcl}}}}", dir.display());
    assert_eq!(ok(&mut i, brace.as_bytes()), b"a.tcl a.tcl");
    let escaped = format!(r"glob -d {} -ta -- {{a\.tcl}}", dir.display());
    assert_eq!(ok(&mut i, escaped.as_bytes()), b"a.tcl");
    // Each occurrence must be a list. C interprets only the last list's members.
    let repeated = format!("glob -d {} -ta -ty bogus -ty f -- a.tcl", dir.display());
    assert_eq!(ok(&mut i, repeated.as_bytes()), b"a.tcl");
    // `-j` is `-join`, and `--` still ends the scan.
    let joined = format!("glob -n -d {} -j -- *.tcl", dir.display());
    assert_eq!(
        ok(&mut i, joined.as_bytes()),
        dir.join("a.tcl").display().to_string().as_bytes()
    );
    std::fs::remove_dir_all(&dir).ok();
}
