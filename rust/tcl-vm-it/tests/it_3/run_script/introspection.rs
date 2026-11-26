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

//! `info`, `namespace`, `file` and `interp` queries over the shared command cores, and the autoloader's `unknown`.

use super::*;

/// `namespace path` sets a per-namespace command resolution path consulted
/// after the current namespace and before the global one, so e.g. the math
/// operators resolve unqualified. Regression for mathop / cmdIL (which crashed
/// on the unimplemented subcommand). `namespace path` with no list reads it back.
#[test]
fn namespace_path_resolves_commands() {
    let (ok, result, _) = run(
        "namespace eval foo {\n  namespace path ::tcl::mathop\n  set r [+ 2 3]\n  lappend r [namespace path]\n  set r\n}",
    );
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "5 ::tcl::mathop");
}

/// `init_auto_load` installs the `unknown`/`auto_load` procs so an unresolved
/// command is looked up on demand. With an empty `auto_path` (no library scan)
/// a genuinely unknown command still raises the standard `invalid command name`
/// error — confirming the proc is installed and the miss path matches C.
/// (word.test, driven through the real library, is the positive integration
/// test.)
#[test]
fn autoloader_installs_unknown_and_reports_miss() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm.init_auto_load();
    // Decouple from any on-disk library so the test is environment-independent.
    let _ = vm.eval_source("set ::auto_path {}");

    let c = vm.eval_source("info commands unknown").expect("compiles");
    assert_eq!(&*c.result.to_str(), "unknown", "unknown proc is installed");

    let c = vm
        .eval_source("catch {nonexistent_xyz_cmd} m; set m")
        .expect("compiles");
    assert_eq!(
        &*c.result.to_str(),
        "invalid command name \"nonexistent_xyz_cmd\"",
    );
}

/// Ensemble subcommands resolve an unambiguous prefix, matching C's
/// `Tcl_GetIndexFromObj`: `info command` → `commands`, `file ext` →
/// `extension`, `file dir` → `dirname`. (cmdAH.test's constraint setup uses
/// `info command`; the absence of prefix matching aborted the whole file.)
#[test]
fn info_and_file_subcommand_prefix_abbreviation() {
    assert_eq!(run("info command set").1, "set");
    assert_eq!(run("file ext a.tar.gz").1, ".gz");
    assert_eq!(run("file dir /a/b/c").1, "/a/b");
}

/// `info level` runs through the shared Family-B core
/// (`tcl_cmd_core::info::level`, over the `Introspect` role trait): the current
/// depth with no argument, and the correct coercion error for a non-integer
/// argument: a local "bad level" message here would diverge from the runtime
/// and from real Tcl.
#[test]
fn info_level_shared_core() {
    assert_eq!(run("info level").1, "0"); // global scope: depth 0
    let (ok, result, _out) = run("info level foo");
    assert!(!ok);
    assert_eq!(result, "expected integer but got \"foo\"");
}

/// `info exists` runs through the shared core (`VarStore::exists`). A
/// current-frame existence check that is scalar-only
/// (`var_exists`) would report arrays like `::env` / `a` as not existing.
#[test]
fn info_exists_shared_core() {
    assert_eq!(run("info exists nope").1, "0");
    assert_eq!(run("set x 1\ninfo exists x").1, "1");
    // Arrays and array elements (the cases the old scalar-only check missed).
    assert_eq!(run("set a(k) v\ninfo exists a").1, "1");
    assert_eq!(run("set a(k) v\ninfo exists a(k)").1, "1");
    assert_eq!(run("set a(k) v\ninfo exists a(nope)").1, "0");
}

/// `namespace tail`/`qualifiers` run through the shared pure core
/// (`tcl_cmd_core::namespace`). A run
/// of 3+ colons is one separator (C semantics); `rsplit("::")` alone
/// would yield a stray `:`.
#[test]
fn namespace_tail_qualifiers_colon_runs() {
    assert_eq!(run("namespace tail ::a::b::c").1, "c");
    assert_eq!(run("namespace qualifiers ::a::b::c").1, "::a::b");
    assert_eq!(run("namespace tail foo:::").1, ""); // not a stray `:`
    assert_eq!(run("namespace qualifiers foo:::").1, "foo");
}

/// `info complete` runs through the shared core (`tcl_cmd_core::info::complete`,
/// C's `Tcl_CommandComplete`). A counter that tracks
/// brackets even inside `{braces}` (where `[` is literal) would call `{[}` incomplete.
#[test]
fn info_complete_shared_core() {
    assert_eq!(run("info complete {set x 1}").1, "1");
    assert_eq!(run("info complete {set x [}").1, "0"); // unclosed bracket
    assert_eq!(run("info complete {{[}}").1, "1"); // `{[}` — brackets inside braces are literal
}

/// `namespace current`/`which` route through the shared `Namespaces` cores
/// (`current`/`name` and `find_command`/`command_name`).
#[test]
fn namespace_current_which_shared_core() {
    assert_eq!(run("namespace current").1, "::");
    assert_eq!(run("namespace eval foo {namespace current}").1, "::foo");
    assert_eq!(run("namespace which -command set").1, "::set");
    assert_eq!(run("namespace which -command no_such_cmd_xyz").1, "");
}

/// `file dirname`/`tail`/`extension`/`rootname` run through the shared
/// `/`-based byte path core (platform-independent), not `std::path::Path`,
/// which is not platform-independent.
#[test]
fn file_path_ops_shared_core() {
    assert_eq!(run("file tail /a/b/c").1, "c");
    assert_eq!(run("file dirname /a/b/c").1, "/a/b");
    assert_eq!(run("file extension a/b.txt").1, ".txt");
    assert_eq!(run("file rootname a/b.txt").1, "a/b");
    assert_eq!(run("file tail /a/b/").1, "b"); // trailing slash ignored
    assert_eq!(run("file dirname foo").1, ".");
}

/// `interp eval {} script …` — only the current interpreter (empty path) exists;
/// its scripts evaluate in the current frame, like `eval` (verified vs tclsh 9.0).
/// A non-empty path is an unknown child interpreter.
#[test]
fn interp_eval_current() {
    assert_eq!(run("interp eval {} {expr 1+1}").1, "2");
    assert_eq!(run("set y 0; interp eval {} {set y 5}; puts $y").2, "5\n");
    // Runs in the current frame: a proc local is visible.
    assert_eq!(
        run("proc p {} { set loc 1; interp eval {} {info exists loc} }; puts [p]").2,
        "1\n"
    );
    // Multiple script args are concatenated.
    let (ok, msg, _) = run("interp eval {} {string length hello} {append}");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"string length string\"");
    // A named (non-empty) path is an unknown interpreter.
    let (ok, msg, _) = run("interp eval foo {set x 1}");
    assert!(!ok);
    assert_eq!(msg, "could not find interpreter \"foo\"");
}
