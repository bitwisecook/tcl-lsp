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

//! End-to-end coverage for the `tcl::prefix`, `info`, and `namespace`
//! ensembles. Each script is compiled as real Tcl via `tcl-compiler` and run
//! through `tcl-vm`; every expectation is pinned against real `tclsh`
//! (8.6 + 9.0) — cited in `// tclsh:` comments. The VM reports
//! `info tclversion` == 9.0 / `info patchlevel` == 9.0.4, so tclsh9.0 is the
//! primary oracle; where the two C versions agree the comment says "both".
//!
//! Former divergences from C Tcl on valid input (`*_bug` tests) now assert the
//! correct tclsh behaviour and pass, guarding the fix against regression.
//! Features the VM genuinely stubs (accepted no-op / "unknown subcommand"
//! where tclsh does real work) are marked `// UNIMPLEMENTED:` and asserted
//! against the VM's actual (documented) behaviour, not tclsh's.

// Harness — the `tcl-compiler` compile service plus a captured-output VM.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileService, Vm};

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// tclsh9.0.4's `info` ensemble enumeration (`info {}` → `unknown or ambiguous
/// subcommand "": must be …`).
const INFO_MUST: &str = "must be args, body, class, cmdcount, cmdtype, commands, complete, \
                         constant, consts, coroutine, default, errorstack, exists, frame, \
                         functions, globals, hostname, level, library, loaded, locals, \
                         nameofexecutable, object, patchlevel, procs, script, \
                         sharedlibextension, tclversion, or vars";

/// tclsh9.0.4's `file` ensemble enumeration.
const FILE_MUST: &str = "must be atime, attributes, channels, copy, delete, dirname, \
                         executable, exists, extension, home, isdirectory, isfile, join, \
                         link, lstat, mkdir, mtime, nativename, normalize, owned, pathtype, \
                         readable, readlink, rename, rootname, separator, size, split, stat, \
                         system, tail, tempdir, tempfile, tildeexpand, type, volumes, or \
                         writable";

/// [`run`] under an explicit release pin — one resolved profile drives both
/// the compile service and the VM's runtime version, as `tclvm --tcl-version`
/// does.
fn run_for_version(src: &str, version: tcl_dialect::TclVersion) -> (bool, String, String) {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_runtime_version(version);
    vm.set_compiler(Box::new(service));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

// tcl::prefix  (cmd_prefix.rs)

/// `tcl::prefix all table string` — every entry that has `string` as a prefix,
/// in table order; empty `string` matches all; no match yields the empty list.
#[test]
fn prefix_all_lists_matches() {
    // tclsh (both): -> "apple apricot"
    assert_eq!(
        run("tcl::prefix all {apple apricot banana} ap").1,
        "apple apricot"
    );
    // tclsh (both): no match -> empty list.
    assert_eq!(run("tcl::prefix all {apple apricot} z").1, "");
    // tclsh (both): empty string matches everything.
    assert_eq!(run("tcl::prefix all {a b c} {}").1, "a b c");
}

/// `tcl::prefix all` wrong arg count.
#[test]
fn prefix_all_wrong_args() {
    // tclsh (both): wrong # args: should be "tcl::prefix all table string"
    let (ok, msg, _) = run("tcl::prefix all a");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "tcl::prefix all table string""#
    );
}

/// `tcl::prefix all` over a malformed table surfaces the list parse error.
#[test]
fn prefix_all_bad_list_errors() {
    // tclsh (both): catch {tcl::prefix all "\{" z} -> "unmatched open brace in list"
    let (ok, msg, _) = run("tcl::prefix all \"\\{\" z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `tcl::prefix longest table string` — the longest common prefix of the
/// matching entries; a single match returns that whole entry; no match -> empty;
/// empty `string` -> longest common prefix of the whole table.
#[test]
fn prefix_longest_common_prefix() {
    // tclsh (both): -> "ap"
    assert_eq!(run("tcl::prefix longest {apple apricot banana} ap").1, "ap");
    // tclsh (both): a single match returns that whole entry.
    assert_eq!(run("tcl::prefix longest {apple apricot} app").1, "apple");
    // tclsh (both): no match -> empty.
    assert_eq!(run("tcl::prefix longest {apple apricot} z").1, "");
    // tclsh (both): empty string -> "" (no common prefix between a and b).
    assert_eq!(run("tcl::prefix longest {a b} {}").1, "");
}

/// `tcl::prefix longest` wrong arg count.
#[test]
fn prefix_longest_wrong_args() {
    // tclsh (both): wrong # args: should be "tcl::prefix longest table string"
    let (ok, msg, _) = run("tcl::prefix longest a");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "tcl::prefix longest table string""#
    );
}

/// `tcl::prefix match` — a unique prefix matches; an exact table entry always
/// wins even when it is also a prefix of another entry.
#[test]
fn prefix_match_unique_and_exact_entry() {
    // tclsh (both): unique prefix -> "banana"
    assert_eq!(
        run("tcl::prefix match {apple apricot banana} b").1,
        "banana"
    );
    // tclsh (both): exact entry `apple` wins over the longer `applet`.
    assert_eq!(run("tcl::prefix match {apple applet} apple").1, "apple");
}

/// `tcl::prefix match` ambiguous / bad errors, including the Oxford-comma list
/// rendering (no comma for two entries, comma + "or" for three+).
#[test]
fn prefix_match_ambiguous_and_bad() {
    // tclsh (both): ambiguous prefix over 3 entries (Oxford comma).
    let (ok, msg, _) = run("tcl::prefix match {apple apricot banana} ap");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"ambiguous option "ap": must be apple, apricot, or banana"#
    );
    // tclsh (both): no match -> bad option, 3-entry list.
    let (ok, msg, _) = run("tcl::prefix match {apple apricot banana} z");
    assert!(!ok);
    assert_eq!(msg, r#"bad option "z": must be apple, apricot, or banana"#);
    // tclsh (both): two-entry list omits the comma.
    let (ok, msg, _) = run("tcl::prefix match {apple apricot} z");
    assert!(!ok);
    assert_eq!(msg, r#"bad option "z": must be apple or apricot"#);
    // tclsh (both): single-entry list.
    let (ok, msg, _) = run("tcl::prefix match {apple} z");
    assert!(!ok);
    assert_eq!(msg, r#"bad option "z": must be apple"#);
    // tclsh (both): empty table -> "no valid options".
    let (ok, msg, _) = run("tcl::prefix match {} z");
    assert!(!ok);
    assert_eq!(msg, r#"bad option "z": no valid options"#);
}

/// Issue #1607: `tcl::prefix` is itself a `TclMakeEnsemble` command, and
/// `tcl::prefix match`'s own options are a `Tcl_GetIndexFromObj(…, "option", 0)`
/// table (`matchOptions[]`, `tclIndexObj.c`) — both were matched exactly here.
///
/// tclsh 8.6.16 / 9.0.4:
/// ```text
/// tcl::prefix {} {a} a         -> unknown or ambiguous subcommand "":
///                                 must be all, longest, or match
/// tcl::prefix m {a} a          -> a
/// tcl::prefix match -e {a} a   -> ambiguous option "-e": must be -error,
///                                 -exact, or -message
/// tcl::prefix match -x {a} a   -> bad option "-x": must be -error, -exact,
///                                 or -message
/// tcl::prefix match -m noun {a} b -> bad noun "b": must be a
/// ```
#[test]
fn prefix_ensemble_and_match_options_resolve_like_tclsh() {
    const OPT_MUST: &str = "must be -error, -exact, or -message";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("tcl::prefix {} {a} a"),
        "unknown or ambiguous subcommand \"\": must be all, longest, or match"
    );
    assert_eq!(run("tcl::prefix m {a} a").1, "a");
    assert_eq!(run("tcl::prefix l {apple applet} ap").1, "apple");
    assert_eq!(
        msg("tcl::prefix match -e {a} a"),
        format!("ambiguous option \"-e\": {OPT_MUST}")
    );
    assert_eq!(
        msg("tcl::prefix match -x {a} a"),
        format!("bad option \"-x\": {OPT_MUST}")
    );
    // `-m` uniquely abbreviates `-message`, whose value becomes the noun.
    assert_eq!(
        msg("tcl::prefix match -m noun {a} b"),
        "bad noun \"b\": must be a"
    );
}

/// The empty word never abbreviation-matches: one entry reports `bad`, two or
/// more report `ambiguous` — C's `Tcl_GetIndexFromObjStruct` rejects the empty
/// key before the abbreviation count is consulted. (Previously the VM resolved
/// `tcl::prefix match {apple} ""` to `apple`.)
#[test]
fn prefix_match_empty_word_never_abbreviates() {
    // tclsh 8.6.16: bad option "": must be apple
    let (ok, msg, _) = run(r#"tcl::prefix match {apple} """#);
    assert!(!ok);
    assert_eq!(msg, r#"bad option "": must be apple"#);
    // tclsh 8.6.16: ambiguous option "": must be apple or banana
    let (ok, msg, _) = run(r#"tcl::prefix match {apple banana} """#);
    assert!(!ok);
    assert_eq!(msg, r#"ambiguous option "": must be apple or banana"#);
}

/// `tcl::prefix match -exact` requires a full table entry; a mere prefix fails.
#[test]
fn prefix_match_exact_option() {
    // tclsh (both): exact match succeeds.
    assert_eq!(
        run("tcl::prefix match -exact {apple apricot} apple").1,
        "apple"
    );
    // tclsh (both): a prefix fails under -exact.
    let (ok, msg, _) = run("tcl::prefix match -exact {apple apricot} app");
    assert!(!ok);
    assert_eq!(msg, r#"bad option "app": must be apple or apricot"#);
}

/// `tcl::prefix match -message word` substitutes the noun in the error message
/// ("option" -> the given word). Works combined with other options.
#[test]
fn prefix_match_message_option() {
    // tclsh (both): -message switch -> bad switch "z": ...
    let (ok, msg, _) = run("tcl::prefix match -message switch {apple apricot} z");
    assert!(!ok);
    assert_eq!(msg, r#"bad switch "z": must be apple or apricot"#);
    // tclsh (both): -message x -exact (option region is objv[2..objc-2]).
    let (ok, msg, _) = run("tcl::prefix match -message x -exact {a b} z");
    assert!(!ok);
    assert_eq!(msg, r#"bad x "z": must be a or b"#);
}

/// `tcl::prefix match -error {}` makes a non-match return the empty string
/// (rc 0) instead of raising; `-error <opts>` re-raises with the caller's
/// return options applied (still an error, same message).
#[test]
fn prefix_match_error_option() {
    // tclsh (both): -error {} on a non-match -> empty result, no error.
    let (ok, res, _) = run("tcl::prefix match -error {} {apple apricot} z");
    assert!(ok, "got: {res}");
    assert_eq!(res, "");
    // tclsh (both): -error with return-opts re-raises the bad-option message.
    let (ok, msg, _) =
        run("catch {tcl::prefix match -error {-level 0 -code error} {a b} z} m; set m\nset m");
    assert!(ok);
    assert_eq!(msg, r#"bad option "z": must be a or b"#);
    // tclsh (both): an odd-length -error list is rejected.
    let (ok, msg, _) = run("tcl::prefix match -error {-code} {a b} z");
    assert!(!ok);
    assert_eq!(msg, "error options must have an even number of elements");
    // tclsh (both): a malformed -error list surfaces the list parse error.
    let (ok, msg, _) = run("tcl::prefix match -error \"\\{\" {a b} z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `tcl::prefix match` arg-count and bad-option diagnostics.
#[test]
fn prefix_match_arg_and_option_errors() {
    // tclsh (both): too few args.
    let (ok, msg, _) = run("tcl::prefix match");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "tcl::prefix match ?options? table string""#
    );
    // tclsh (both): a `-message` with no value lands as the `table` word, so the
    // remaining count is short -> wrong # args (NOT "missing value").
    let (ok, msg, _) = run("tcl::prefix match -message");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "tcl::prefix match ?options? table string""#
    );
    // tclsh (both): an unknown option in the option region.
    let (ok, msg, _) = run("tcl::prefix match -bogus {a b} a");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"bad option "-bogus": must be -error, -exact, or -message"#
    );
}

/// `tcl::prefix` dispatch: missing subcommand, and an unknown subcommand.
#[test]
fn prefix_dispatch_errors() {
    // tclsh (both): no subcommand.
    let (ok, msg, _) = run("tcl::prefix");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "tcl::prefix subcommand ?arg ...?""#
    );
    // tclsh (both): unknown subcommand.
    let (ok, msg, _) = run("tcl::prefix frobnicate a b");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"unknown or ambiguous subcommand "frobnicate": must be all, longest, or match"#
    );
}

// info  (cmd_info.rs)

/// `info exists` — scalars, arrays, and array elements (shared `VarStore`).
#[test]
fn info_exists_scalar_array_element() {
    // tclsh (both): set x 1 -> exists x = 1, exists y = 0.
    assert_eq!(
        run("set x 1; list [info exists x] [info exists y]").1,
        "1 0"
    );
    // tclsh (both): array, present element, absent element -> 1 1 0.
    assert_eq!(
        run("set a(k) 1; list [info exists a] [info exists a(k)] [info exists a(z)]").1,
        "1 1 0",
    );
    // tclsh (both): wrong # args.
    let (ok, msg, _) = run("info exists");
    assert!(!ok);
    assert_eq!(msg, r#"wrong # args: should be "info exists varName""#);
}

/// `info complete` — C's `Tcl_CommandComplete`; brackets inside braces are
/// literal (`{[}` is complete).
#[test]
fn info_complete_cases() {
    assert_eq!(run("info complete {set x 1}").1, "1"); // tclsh (both): 1
    assert_eq!(run("info complete {set x [}").1, "0"); // tclsh (both): 0 (open bracket)
    assert_eq!(run("info complete {{[}}").1, "1"); // tclsh (both): 1 (bracket in braces)
}

/// `info level` — depth 0 at the top, the current depth and the invoking
/// command inside a proc, and `bad level` for out-of-range / negative numbers.
#[test]
fn info_level_depth_and_errors() {
    assert_eq!(run("info level").1, "0"); // tclsh (both): top-level depth 0
    // tclsh (both): inside f -> level 1, and `info level 1` is the call argv.
    assert_eq!(
        run("proc f {} {list [info level] [info level 1]}; f").1,
        "1 f"
    );
    // tclsh (both): negative absolute level.
    let (ok, msg, _) = run("info level -5");
    assert!(!ok);
    assert_eq!(msg, r#"bad level "-5""#);
    // tclsh (both): a level deeper than the stack.
    let (ok, msg, _) = run("info level 5");
    assert!(!ok);
    assert_eq!(msg, r#"bad level "5""#);
}

/// `info commands ?pattern?` — namespace-aware, including a qualified pattern
/// that resolves a command created in a child namespace.
#[test]
fn info_commands_pattern() {
    // tclsh (both): exact builtin name.
    assert_eq!(run("info commands set").1, "set");
    // tclsh (both): a known builtin appears in the unfiltered list.
    assert_eq!(run("expr {\"set\" in [info commands]}").1, "1");
    // tclsh (both): a qualified pattern resolves the child-namespace proc.
    assert_eq!(
        run("namespace eval foo {proc bar {} {}}; info commands foo::bar").1,
        "::foo::bar"
    );
}

/// `info procs ?pattern?` — only user procs, glob-filtered.
#[test]
fn info_procs_pattern() {
    assert_eq!(run("proc foo {} {}; info procs foo").1, "foo"); // tclsh (both): foo
    // tclsh (both): glob over two procs.
    assert_eq!(
        run("proc abc {} {}; proc abd {} {}; lsort [info procs ab*]").1,
        "abc abd"
    );
    // tclsh (both): a builtin is not a proc.
    assert_eq!(run("info procs set").1, "");
}

/// `info vars` / `globals` / `locals` — the variable-listing cores. In a proc,
/// `vars`/`locals` see only the locals (not globals); `globals` filters the
/// global namespace.
#[test]
fn info_vars_globals_locals() {
    assert_eq!(run("set abc 1; info vars abc").1, "abc"); // tclsh (both): abc
    assert_eq!(run("set ::gv1 1; info globals gv1").1, "gv1"); // tclsh (both): gv1
    // tclsh (both): inside a proc, info vars lists only the local (no globals).
    assert_eq!(
        run("set ::gg 1; proc f {} {set loc 1; lsort [info vars]}; f").1,
        "loc"
    );
    // tclsh (both): info locals lists only locals.
    assert_eq!(run("proc f {} {set loc 1; info locals}; f").1, "loc");
}

/// `info args` / `body` / `default` — proc metadata, with the canonical
/// "isn't a procedure" / "doesn't have an argument" errors.
#[test]
fn info_args_body_default() {
    assert_eq!(run("proc f {a b} {}; info args f").1, "a b"); // tclsh (both): a b
    assert_eq!(run("proc f {} {set x 1}; info body f").1, "set x 1"); // tclsh (both)
    // tclsh (both): info default writes the default into the named var, returns 1.
    assert_eq!(
        run("proc f {a {b 5}} {}; list [info default f b v] $v").1,
        "1 5"
    );
    // tclsh (both): an arg with no default -> 0, var set to empty.
    assert_eq!(
        run("proc f {a} {}; list [info default f a v] <$v>").1,
        "0 <>"
    );
    // tclsh (both): info args/body of a non-proc command.
    let (ok, msg, _) = run("info args set");
    assert!(!ok);
    assert_eq!(msg, r#""set" isn't a procedure"#);
    let (ok, msg, _) = run("info body set");
    assert!(!ok);
    assert_eq!(msg, r#""set" isn't a procedure"#);
    // tclsh (both): info default on a missing proc / unknown arg.
    let (ok, msg, _) = run("info default nosuch a v");
    assert!(!ok);
    assert_eq!(msg, r#""nosuch" isn't a procedure"#);
    let (ok, msg, _) = run("proc f {a} {}; info default f z v");
    assert!(!ok);
    assert_eq!(msg, r#"procedure "f" doesn't have an argument "z""#);
}

/// `info tclversion` / `patchlevel` — the VM targets Tcl 9, so these match
/// tclsh9.0 exactly (they DIFFER from 8.6, which reports 8.6 / 8.6.14).
#[test]
fn info_version_patchlevel() {
    // tclsh9.0: 9.0  (tclsh8.6: 8.6) — VM targets 9.
    assert_eq!(run("info tclversion").1, "9.0");
    // tclsh9.0: 9.0.4  (tclsh8.6: 8.6.14) — VM targets 9.
    assert_eq!(run("info patchlevel").1, "9.0.4");
    // arg-count guards.
    let (ok, _, _) = run("info tclversion x");
    assert!(!ok);
    let (ok, _, _) = run("info patchlevel x");
    assert!(!ok);
}

/// `info functions ?pattern?` — registered math functions; `abs` is present and
/// the glob filter works.
#[test]
fn info_functions_lists_mathfuncs() {
    assert_eq!(run("expr {\"abs\" in [info functions]}").1, "1"); // tclsh (both): 1
    assert_eq!(run("info functions abs").1, "abs"); // tclsh (both): abs
    // a pattern that matches nothing -> empty.
    assert_eq!(run("info functions no_such_func_xyz").1, "");
}

/// `info script` — empty when evaluating a non-file script (matches tclsh, which
/// also reports "" when sourcing from stdin / a string).
#[test]
fn info_script_empty_for_string_eval() {
    // tclsh (both, via stdin): info script -> "" .
    assert_eq!(run("list <[info script]>").1, "<>");
}

/// `info nameofexecutable` — environment-specific; tclsh returns the interpreter
/// path. The VM has no executable, so it returns the empty string. Asserting the
/// VM's documented stub value (it does NOT error), and noting the divergence.
/// UNIMPLEMENTED / env-specific: tclsh -> "/usr/local/bin/tclsh9.0"; VM -> "".
#[test]
fn info_nameofexecutable_stub_empty() {
    let (ok, res, _) = run("info nameofexecutable");
    assert!(ok, "must not error: {res}");
    // VM stub: empty (tclsh returns a path — environment-specific, not asserted).
    assert_eq!(res, "");
}

/// `info` dispatch: missing subcommand, and prefix abbreviation of a subcommand
/// (`info comm` -> commands, `info ex` -> exists) via `Tcl_GetIndexFromObj`.
#[test]
fn info_dispatch_and_abbreviation() {
    // tclsh (both): no subcommand.
    let (ok, msg, _) = run("info");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "info subcommand ?arg ...?""#
    );
    // tclsh (both): unambiguous prefixes resolve.
    assert_eq!(run("info comm set").1, "set"); // commands
    assert_eq!(run("set x 1; info ex x").1, "1"); // exists
}

/// Subcommands the VM does not implement (no entry in its dispatch) error with
/// "unknown or ambiguous subcommand". In real tclsh these all WORK:
///   info cmdcount   -> an integer
///   info hostname   -> the host name
///   info frame      -> a frame count / dict
/// UNIMPLEMENTED in the VM (coverage limit, not a correctness bug on supported
/// input): asserting the VM's actual error so the gap is pinned and visible.
/// The list is exactly the `info` subcommands the VM's dispatch table lacks;
/// every other subcommand this suite exercises is implemented and covered
/// above.
///
/// Since #1607 the miss is composed by `tcl_cmd_core::ensemble`, so it carries
/// tclsh9.0.4's full `must be` clause — the name itself still appears there,
/// because the word *resolved* against the ensemble table and only then found
/// no implementation.
#[test]
fn info_unimplemented_subcommands_error() {
    for sub in ["cmdcount", "hostname", "frame"] {
        let (ok, msg, _) = run(&format!("info {sub}"));
        assert!(!ok, "VM unexpectedly implemented `info {sub}`: {msg}");
        assert_eq!(
            msg,
            format!("unknown or ambiguous subcommand \"{sub}\": {INFO_MUST}"),
            "`info {sub}` should report the VM's unknown-subcommand error",
        );
    }
}

// namespace  (cmd_namespace.rs)

/// `namespace eval` runs the body in the target namespace (creating it),
/// `namespace current` reports it, and a top-level `return` in the body is
/// absorbed as the result.
#[test]
fn namespace_eval_current_return() {
    assert_eq!(run("namespace current").1, "::"); // tclsh (both): global is ::
    assert_eq!(run("namespace eval foo {namespace current}").1, "::foo"); // tclsh (both)
    // tclsh (both): nested eval reports the nested namespace.
    assert_eq!(
        run("namespace eval a {namespace eval b {namespace current}}").1,
        "::a::b"
    );
    // tclsh (both): a `return` in the body is the eval's result.
    assert_eq!(run("namespace eval foo {return 42}").1, "42");
    // tclsh (both): multiple body words are joined as one script (`set z 5`).
    assert_eq!(run("namespace eval foo {set z} 5").1, "5");
}

/// `namespace eval` arg-count errors.
#[test]
fn namespace_eval_wrong_args() {
    // tclsh (both): no body.
    let (ok, msg, _) = run("namespace eval foo");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "namespace eval name arg ?arg ...?""#
    );
}

/// `namespace parent` — of the current namespace and of a named one; the global
/// namespace's parent is empty.
#[test]
fn namespace_parent() {
    // tclsh (both): inside ::a::b -> ::a .
    assert_eq!(run("namespace eval a::b {namespace parent}").1, "::a");
    // tclsh (both): of a named namespace.
    assert_eq!(
        run("namespace eval a::b {}; namespace parent ::a::b").1,
        "::a"
    );
    // tclsh (both): parent of :: is "" .
    assert_eq!(run("namespace parent ::").1, "");
}

/// `namespace children ?ns? ?pattern?` — lists child namespaces, with the
/// optional glob filter.
#[test]
fn namespace_children() {
    // tclsh (both): the created child appears in ::'s children.
    assert_eq!(
        run("namespace eval zzq {}; expr {\"::zzq\" in [namespace children ::]}").1,
        "1"
    );
    // tclsh (both): a pattern selects a single child.
    assert_eq!(
        run("namespace eval aq {}; namespace eval bq {}; lsort [namespace children :: aq]").1,
        "::aq"
    );
}

/// `namespace qualifiers` / `tail` — pure text ops over `::`-runs (a run of 3+
/// colons is one separator).
#[test]
fn namespace_qualifiers_tail() {
    assert_eq!(run("namespace qualifiers ::a::b::c").1, "::a::b"); // tclsh (both)
    assert_eq!(run("namespace tail ::a::b::c").1, "c"); // tclsh (both)
    assert_eq!(run("namespace tail foo").1, "foo"); // tclsh (both): no qualifier
    assert_eq!(run("namespace qualifiers foo").1, ""); // tclsh (both)
    assert_eq!(run("namespace tail foo:::").1, ""); // tclsh (both): 3-colon run
    assert_eq!(run("namespace qualifiers foo:::").1, "foo"); // tclsh (both)
}

/// `namespace exists` — for a present and an absent namespace.
#[test]
fn namespace_exists() {
    // tclsh (both): present -> 1, absent -> 0.
    assert_eq!(
        run("namespace eval foo {}; list [namespace exists ::foo] [namespace exists ::bar]").1,
        "1 0"
    );
}

/// `namespace delete` — destroys the namespace; an unknown namespace errors
/// (after deleting any earlier ones).
#[test]
fn namespace_delete() {
    // tclsh (both): after delete the namespace no longer exists.
    assert_eq!(
        run("namespace eval foo {}; namespace delete foo; namespace exists ::foo").1,
        "0"
    );
    // tclsh (both): deleting an unknown namespace errors.
    let (ok, msg, _) = run("namespace delete nope");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"unknown namespace "nope" in namespace delete command"#
    );
}

/// `namespace which -command name` — the resolved fully-qualified command name
/// (empty when unresolved).
#[test]
fn namespace_which_command() {
    assert_eq!(run("namespace which -command set").1, "::set"); // tclsh (both)
    assert_eq!(run("namespace which -command no_such_cmd_xyz").1, ""); // tclsh (both)
    // a bare name (no flag) still resolves the command.
    assert_eq!(run("namespace which set").1, "::set");
}

/// `namespace which -variable name` — tclsh resolves the variable's FQN, and the
/// VM now honours the `-variable` flag and resolves the variable table.
///   script `namespace eval foo {variable v 1}; namespace which -variable ::foo::v`
///   tclsh (both): `::foo::v` — matched by the VM (guards regression).
#[test]
fn namespace_which_variable_bug() {
    let (ok, res, _) = run("namespace eval foo {variable v 1}; namespace which -variable ::foo::v");
    assert!(ok, "must not error: {res}");
    // tclsh-correct; VM currently yields "" (the documented divergence).
    assert_eq!(res, "::foo::v");
}

/// `namespace origin command` — the command's fully-qualified name; an unknown
/// command errors. (The VM does not track import provenance, but for a directly
/// defined command the qualified name matches tclsh.)
#[test]
fn namespace_origin() {
    assert_eq!(run("namespace origin set").1, "::set"); // tclsh (both)
    // tclsh (both): unknown command.
    let (ok, msg, _) = run("namespace origin nope_xyz");
    assert!(!ok);
    assert_eq!(msg, r#"invalid command name "nope_xyz""#);
}

/// `namespace code script` — captures the current namespace as an `inscope`
/// callback prefix.
#[test]
fn namespace_code() {
    // tclsh (both): -> "::namespace inscope ::foo {bar baz}"
    assert_eq!(
        run("namespace eval foo {namespace code {bar baz}}").1,
        "::namespace inscope ::foo {bar baz}",
    );
}

/// `namespace inscope ns script` — runs the script in the target namespace,
/// seeing its variables; bad arg counts error.
#[test]
fn namespace_inscope() {
    // tclsh (both): the body sees foo's variable.
    assert_eq!(
        run("namespace eval foo {variable v hi}; namespace inscope foo {set v}").1,
        "hi"
    );
    // tclsh (both): missing script.
    let (ok, msg, _) = run("namespace inscope foo");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "namespace inscope namespace arg ?arg ...?""#
    );
}

/// A namespace holding a `shape` proc that reports its argument *count* and
/// each argument's exact text — the probe every `namespace inscope` tail test
/// below calls, so a wrongly-split argument shows up as a different count.
const SHAPE_NS: &str = "namespace eval foo \
     {proc shape {args} {return [llength $args]:[join $args ,]}}; ";

/// Issue #1056 — the differential pair that separates `namespace inscope` from
/// the rest of the `Tcl_ConcatObj` eval family. `inscope` appends its trailing
/// words as **list elements** (`NamespaceInscopeCmd` builds a list object and
/// concatenates its string rep), so `{a b}` reaches `puts` as one argument;
/// `namespace eval` space-joins, so the same words become two and `puts`
/// reports a bad channel. The VM used to space-join in both.
#[test]
fn namespace_inscope_appends_list_args_where_eval_concatenates() {
    // tclsh (both): prints "a b" — a single argument.
    let (ok, res, out) = run("namespace inscope :: {puts} {a b}");
    assert!(ok, "must not error: {res}");
    assert_eq!(out, "a b\n");
    // tclsh (both): `namespace eval` splits the same words -> bad channel "a".
    let (ok, msg, _) = run("namespace eval :: {puts} {a b}");
    assert!(!ok);
    assert_eq!(msg, r#"can not find channel named "a""#);
}

/// Trailing words survive as one argument each, whatever whitespace or list
/// punctuation they hold — the list quoting (`Tcl_ScanElement` /
/// `Tcl_ConvertElement`) round-trips them.
#[test]
fn namespace_inscope_tail_args_are_list_elements() {
    // tclsh (both): -> "1:x y"
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape {{x y}}")).1,
        "1:x y"
    );
    // tclsh (both): two whitespace-bearing words stay two arguments -> "2:x y,p q"
    assert_eq!(
        run(&format!(
            "{SHAPE_NS}namespace inscope foo shape {{x y}} {{p q}}"
        ))
        .1,
        "2:x y,p q"
    );
    // tclsh (both): a leading/trailing-space word keeps its spaces -> "1:  sp  "
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape \"  sp  \"")).1,
        "1:  sp  "
    );
    // tclsh (both): the script may itself be several words -> "2:one,two three"
    assert_eq!(
        run(&format!(
            "{SHAPE_NS}namespace inscope foo {{shape one}} {{two three}}"
        ))
        .1,
        "2:one,two three"
    );
}

/// The list quoting must be canonical, so a word that is not brace-safe still
/// round-trips: an unbalanced brace and a lone backslash take the backslash
/// form, an empty word becomes `{}`, and `$`/`[`/`;` are braced so no
/// substitution or command termination happens inside the built script.
#[test]
fn namespace_inscope_tail_args_special_characters() {
    // tclsh (both): empty word -> "{}" -> one empty argument.
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape {{}}")).1,
        "1:"
    );
    // tclsh (both): unbalanced open brace -> `a\{b`.
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape a\\{{b")).1,
        "1:a{b"
    );
    // tclsh (both): a lone backslash -> `\\`.
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape \\\\")).1,
        "1:\\"
    );
    // tclsh (both): an unbalanced double quote.
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape \"a\\\"b\"")).1,
        "1:a\"b"
    );
    // tclsh (both): `$`/`[`/`;` are braced, so nothing substitutes and the
    // command does not terminate early.
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape {{$nope}}")).1,
        "1:$nope"
    );
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape {{[nope]}}")).1,
        "1:[nope]"
    );
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo shape {{a;b}}")).1,
        "1:a;b"
    );
    // tclsh (both): a mixed tail -> 4 arguments, the empty one preserved.
    assert_eq!(
        run(&format!(
            "{SHAPE_NS}namespace inscope foo shape {{a b}} {{}} c\\{{d \\\\"
        ))
        .1,
        "4:a b,,c{d,\\"
    );
}

/// Zero trailing words: C takes the `objc == 3` arm and evaluates the script
/// verbatim — no list is appended and no trailing space is added, so a list
/// *inside* the script stays one argument and a plain script is untouched.
#[test]
fn namespace_inscope_zero_tail_args_leaves_script_verbatim() {
    // tclsh (both): -> "1:x y"
    assert_eq!(
        run(&format!(
            "{SHAPE_NS}namespace inscope foo {{shape {{x y}}}}"
        ))
        .1,
        "1:x y"
    );
    // tclsh (both): the body still sees foo's variable -> "hi"
    assert_eq!(
        run("namespace eval foo {variable v hi}; namespace inscope foo {set v}").1,
        "hi"
    );
}

/// `Tcl_ConcatObj` trims each part and drops one that is empty after trimming:
/// a padded script loses its padding, and an all-whitespace script contributes
/// no leading separator at all (the tail alone becomes the script).
#[test]
fn namespace_inscope_script_is_concat_trimmed() {
    // tclsh (both): -> "1:tail"
    assert_eq!(
        run(&format!(
            "{SHAPE_NS}namespace inscope foo {{  shape  }} tail"
        ))
        .1,
        "1:tail"
    );
    // tclsh (both): whitespace-only script -> the tail is the whole command.
    assert_eq!(
        run(&format!("{SHAPE_NS}namespace inscope foo {{   }} shape hi")).1,
        "1:hi"
    );
}

/// Namespace resolution is unchanged by the list-args fix: the target name is
/// still resolved relative to the current namespace, and the body still sees
/// the target's variables.
#[test]
fn namespace_inscope_relative_resolution_unchanged() {
    // tclsh (both): `bar` resolves against `foo` -> "1:x y"
    assert_eq!(
        run("namespace eval foo {namespace eval bar \
             {proc shape {args} {return [llength $args]:[join $args ,]}}}; \
             namespace eval foo {namespace inscope bar shape {x y}}")
        .1,
        "1:x y"
    );
    // tclsh (both): the invoked proc sees its own namespace variable -> "hi/1/x y"
    assert_eq!(
        run("namespace eval foo {variable v hi; \
             proc shape {args} {variable v; return $v/[llength $args]/[lindex $args 0]}}; \
             namespace inscope foo shape {x y}")
        .1,
        "hi/1/x y"
    );
}

/// The exact tail shapes the differential fuzzer's `namespace inscope`
/// production emits (`rust/tcl-fuzz/src/generator.rs`, `inscope_arg`), pinned
/// here so the VM side of the campaign's regression seed is asserted without
/// having to run a campaign. Each expectation is the reference `tclsh` output
/// for the identical generated line.
#[test]
fn namespace_inscope_fuzz_generator_tail_shapes() {
    let ns = "namespace eval n1 {proc _shape {args} \
              {return [llength $args]:[join $args ,]}}; ";
    // tclsh (both): -> "0:" (zero-word tail, script evaluated verbatim).
    assert_eq!(run(&format!("{ns}namespace inscope n1 {{_shape}}")).1, "0:");
    // tclsh (both): -> "3:[nosub],a b c,x y"
    assert_eq!(
        run(&format!(
            "{ns}namespace inscope n1 {{_shape}} {{[nosub]}} {{a b c}} {{x y}}"
        ))
        .1,
        "3:[nosub],a b c,x y"
    );
    // tclsh (both): -> "3:  padded  ,a;b,[nosub]"
    assert_eq!(
        run(&format!(
            "{ns}namespace inscope n1 {{_shape}} {{  padded  }} {{a;b}} {{[nosub]}}"
        ))
        .1,
        "3:  padded  ,a;b,[nosub]"
    );
    // tclsh (both): -> "3:a\"b,a b c,plain"
    assert_eq!(
        run(&format!(
            "{ns}namespace inscope n1 {{_shape}} {{a\"b}} {{a b c}} plain"
        ))
        .1,
        "3:a\"b,a b c,plain"
    );
    // tclsh (both): -> "3:,$nosub,[nosub]" (leading empty element preserved).
    assert_eq!(
        run(&format!(
            "{ns}namespace inscope n1 {{_shape}} {{}} {{$nosub}} {{[nosub]}}"
        ))
        .1,
        "3:,$nosub,[nosub]"
    );
    // tclsh (both): -> "3:plain,  padded  ,[nosub]"
    assert_eq!(
        run(&format!(
            "{ns}namespace inscope n1 {{_shape}} plain {{  padded  }} {{[nosub]}}"
        ))
        .1,
        "3:plain,  padded  ,[nosub]"
    );
}

/// A `namespace code` capture invoked with an extra argument is the real-world
/// path through the list-args rule (Tk-style callbacks): the appended word
/// arrives at the captured command as one argument.
#[test]
fn namespace_code_capture_invoked_with_extra_arg() {
    // tclsh (both): -> "cb:1:x y"
    assert_eq!(
        run(
            "proc cb {args} {return cb:[llength $args]:[join $args ,]}; \
             set cap [namespace code cb]; eval $cap [list {x y}]"
        )
        .1,
        "cb:1:x y"
    );
    assert_eq!(
        run(
            "proc cb {args} {return cb:[llength $args]:[join $args ,]}; \
             ::namespace inscope :: cb {x y}"
        )
        .1,
        "cb:1:x y"
    );
}

/// `namespace export` + `namespace import` — an exported proc becomes callable
/// unqualified after import.
#[test]
fn namespace_export_import() {
    // tclsh (both): after import, `bar` resolves to foo::bar -> "B".
    assert_eq!(
        run("namespace eval foo {namespace export bar; proc bar {} {return B}}; namespace import foo::bar; bar").1,
        "B",
    );
}

/// `namespace path ?list?` — get (empty by default) / set the resolution path;
/// once set, an unqualified command resolves through it.
#[test]
fn namespace_path() {
    // tclsh (both): default path is empty.
    assert_eq!(run("namespace eval foo {namespace path}").1, "");
    // tclsh (both): a set path resolves the math operator unqualified.
    assert_eq!(
        run("namespace eval foo {namespace path ::tcl::mathop; + 2 3}").1,
        "5",
    );
    // read it back as ::-qualified.
    assert_eq!(
        run("namespace eval foo {namespace path ::tcl::mathop; namespace path}").1,
        "::tcl::mathop",
    );
}

/// `namespace` dispatch: the registry supplies the complete, release-filtered
/// subcommand list used by C Tcl's error.
#[test]
fn namespace_dispatch_unknown_subcommand() {
    // tclsh (both): no subcommand.
    let (ok, msg, _) = run("namespace");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "namespace subcommand ?arg ...?""#
    );
    // tclsh9.0/8.6: all 19 subcommands, in this order.
    let (ok, msg, _) = run("namespace frob");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"frob\": must be \
         children, code, current, delete, ensemble, eval, exists, export, forget, import, \
         inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which",
    );
}

/// `namespace forget` removes a previously imported command, so a bare call to
/// it then errors, matching tclsh.
///   tclsh (both): after forget, `catch {bar}` -> 1 (command removed)
///   VM:           after forget, `catch {bar}` -> 1 (command removed)
#[test]
fn namespace_forget_removes_imported_command() {
    let (ok, res, _) = run(concat!(
        "namespace eval foo {namespace export bar; proc bar {} {return B}}; ",
        "namespace import foo::bar; namespace forget foo::bar; catch {bar}",
    ));
    assert!(ok, "must not error: {res}");
    // `namespace forget` removes the imported `bar`, so the bare call errors
    // (catch → 1), matching tclsh.
    assert_eq!(res, "1");
}

/// `namespace ensemble create` builds an ensemble command and returns its
/// fully-qualified name (`::` at the global scope), and the command dispatches
/// subcommands to the namespace's exported procs.
///   tclsh (both): namespace ensemble create -> "::"
#[test]
fn namespace_ensemble_create_dispatches() {
    let (ok, res, _) = run("namespace ensemble create");
    assert!(ok, "must not error: {res}");
    assert_eq!(res, "::");
    // A named ensemble dispatches `cmd sub` to the exported `ns::sub`.
    let (ok, res, _) = run(concat!(
        "namespace eval ns {namespace export greet; proc greet {} {return hi}; ",
        "namespace ensemble create}; ns greet",
    ));
    assert!(ok, "must not error: {res}");
    assert_eq!(res, "hi");
}

/// `namespace upvar` links the namespace cell rather than copying its value.
///   tclsh9.0/8.6: `set ::g 99; namespace upvar :: g lg; set lg 100` -> `100:100`.
#[test]
fn namespace_upvar_links_namespace_cell() {
    let (ok, result, _) = run("set ::g 99; namespace upvar :: g lg; set lg 100; list $lg $::g");
    assert!(ok, "namespace upvar must succeed: {result}");
    assert_eq!(result, "100 100");
}

// info — additional subcommands (Tcl 9.0 surface; the VM targets 9.0)

/// `info constant name` / `info consts ?pattern?` — Tcl 9.0 `const`
/// introspection (8.6 has neither). The VM matches tclsh9.0.
#[test]
fn info_constant_and_consts() {
    // tclsh9.0: const c 5 -> info constant c = 1, info constant x = 0.
    assert_eq!(
        run("const c 5; list [info constant c] [info constant x]").1,
        "1 0"
    );
    // tclsh9.0: info consts c* -> the constant name.
    assert_eq!(run("const c 5; info consts c*").1, "c");
    // wrong # args on `info constant`.
    let (ok, msg, _) = run("info constant");
    assert!(!ok);
    assert_eq!(msg, r#"wrong # args: should be "info constant varname""#);
}

/// `info cmdtype commandName` — Tcl 9.0 (8.6 lacks it). `proc` for a user proc,
/// `native` for a builtin, and "unknown command" for a missing name. The VM
/// matches tclsh9.0.
#[test]
fn info_cmdtype() {
    assert_eq!(run("proc f {} {}; info cmdtype f").1, "proc"); // tclsh9.0: proc
    assert_eq!(run("info cmdtype set").1, "native"); // tclsh9.0: native
    // tclsh9.0: a missing command.
    let (ok, msg, _) = run("info cmdtype nosuchcmd");
    assert!(!ok);
    assert_eq!(msg, r#"unknown command "nosuchcmd""#);
}

/// `info loaded ?interp?` — no binary extensions are loaded, so the result is
/// empty; a named (non-existent) interp errors.
#[test]
fn info_loaded() {
    assert_eq!(run("list <[info loaded]>").1, "<>"); // tclsh (both): empty
    // tclsh (both): an unknown interpreter.
    let (ok, msg, _) = run("info loaded badinterp");
    assert!(!ok);
    assert_eq!(msg, r#"could not find interpreter "badinterp""#);
}

/// `info library` — the script-library directory. tclsh always has one (a
/// non-empty path); the VM has no library seeded in this harness, so it raises
/// the standard "no library has been specified for Tcl" error. Asserting the
/// VM's actual behaviour (environment-specific value, hence the error path).
/// UNIMPLEMENTED here only because no `tcl_library` global is seeded in-test;
/// tclsh would return e.g. "/usr/local/lib/tcl9.0".
#[test]
fn info_library_unseeded_errors_in_vm() {
    let (ok, msg, _) = run("info library");
    assert!(!ok);
    assert_eq!(msg, "no library has been specified for Tcl");
}

// tcl::prefix / namespace — remaining option-region & set-path branches

/// `tcl::prefix match` reports a missing value for `-message`/`-error` when the
/// option lands inside the option region with no following word (here the value
/// slot is consumed by the trailing `table`/`string`).
#[test]
fn prefix_match_missing_option_value_in_region() {
    // `-exact -message {a b} z`: opts = `-exact -message`, so `-message` has no
    // value within the region.
    // tclsh (both): missing value for -message
    let (ok, msg, _) = run("tcl::prefix match -exact -message {a b} z");
    assert!(!ok);
    assert_eq!(msg, "missing value for -message");
    // Same for -error.
    // tclsh (both): missing value for -error
    let (ok, msg, _) = run("tcl::prefix match -exact -error {a b} z");
    assert!(!ok);
    assert_eq!(msg, "missing value for -error");
}

/// `tcl::prefix longest` over a malformed table surfaces the list parse error
/// (the `entries` failure path).
#[test]
fn prefix_longest_bad_list_errors() {
    // tclsh (both): unmatched open brace in list
    let (ok, msg, _) = run("tcl::prefix longest \"\\{\" z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `namespace import -force` overrides an existing command with the import.
#[test]
fn namespace_import_force() {
    // tclsh (both): -force lets foo::bar replace the local `bar` -> "B".
    assert_eq!(
        run(concat!(
            "namespace eval foo {namespace export bar; proc bar {} {return B}}; ",
            "proc bar {} {return X}; namespace import -force foo::bar; bar",
        ))
        .1,
        "B",
    );
}

/// `namespace path` set-path errors: a malformed list, and the wrong arg count.
/// (The VM's wrong-args message uses `?nsList?`; tclsh9.0 uses `?pathList?` —
/// a wording divergence, so this asserts only the malformed-list path against
/// tclsh and pins the VM's arg-count message separately.)
#[test]
fn namespace_path_set_errors() {
    // tclsh (both): a malformed path list surfaces the parse error.
    let (ok, msg, _) = run("namespace eval foo {namespace path \"\\{\"}");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
    // VM arg-count message (note: tclsh9.0 says `?pathList?`, VM says `?nsList?`).
    let (ok, msg, _) = run("namespace path a b");
    assert!(!ok);
    assert_eq!(msg, r#"wrong # args: should be "namespace path ?nsList?""#);
}

// Wrong-arg-count arms & error-propagation paths (close the last gaps)

/// The `info` subcommands' wrong-#-args usage messages (the error arms).
#[test]
fn info_subcommand_wrong_args() {
    for (script, want) in [
        (
            "info complete a b",
            r#"wrong # args: should be "info complete command""#,
        ),
        (
            "info level 1 2",
            r#"wrong # args: should be "info level ?number?""#,
        ),
        (
            "info body",
            r#"wrong # args: should be "info body procname""#,
        ),
        (
            "info args",
            r#"wrong # args: should be "info args procname""#,
        ),
        (
            "info default f a",
            r#"wrong # args: should be "info default procname arg varname""#,
        ),
        (
            "info functions a b",
            r#"wrong # args: should be "info functions ?pattern?""#,
        ),
        // tclsh9.0 wording (8.6 says just `?interp?`); the VM targets 9.0.
        (
            "info loaded a b c",
            r#"wrong # args: should be "info loaded ?interp? ?prefix?""#,
        ),
        (
            "info cmdtype",
            r#"wrong # args: should be "info cmdtype commandName""#,
        ),
    ] {
        let (ok, msg, _) = run(script);
        assert!(!ok, "`{script}` should error");
        assert_eq!(msg, want, "for `{script}`"); // tclsh9.0
    }
}

/// `info default` writes its result into the named variable; when that variable
/// can't be written (it is an existing array), the array-specific error
/// surfaces (the var-write error path, not the default lookup).
#[test]
fn info_default_var_write_error() {
    // tclsh (both): can't set "arr": variable is array
    let (ok, msg, _) = run("proc f {a {b 5}} {}; set arr(x) 1; info default f b arr");
    assert!(!ok);
    assert_eq!(msg, r#"can't set "arr": variable is array"#);
}

/// `namespace parent` / `children` of a non-existent namespace error.
#[test]
fn namespace_parent_children_missing() {
    // tclsh (both): namespace "::nosuch_ns" not found
    let (ok, msg, _) = run("namespace parent ::nosuch_ns");
    assert!(!ok);
    assert_eq!(msg, r#"namespace "::nosuch_ns" not found"#);
    let (ok, msg, _) = run("namespace children ::nosuch_ns");
    assert!(!ok);
    assert_eq!(msg, r#"namespace "::nosuch_ns" not found"#);
}

/// `namespace eval ::` runs in the global namespace (the `canon_ns` `::` fast
/// path); a hard parse error inside a `namespace eval` body is deferred to a
/// catchable runtime error (the `eval_in_ns` Err arm).
#[test]
fn namespace_eval_global_and_body_parse_error() {
    // tclsh (both): the body sets a global -> 7.
    assert_eq!(run("namespace eval :: {set ::zq 7}; set ::zq").1, "7");
    // tclsh (both): a parse error in the body is catchable with C's message.
    let (ok, msg, _) = run("namespace eval foo {set \"a\"x}");
    assert!(!ok);
    assert_eq!(msg, "extra characters after close-quote");
}

/// `namespace inscope` / `eval` bare (missing the script/body args). The VM's
/// usage wording diverges slightly from tclsh9.0 — it reads `namespace`/`name`
/// plus `?arg ...?` where tclsh reads `name` plus `?arg...?`. Asserting the
/// VM's actual message and recording the divergence (error text only), e.g.
/// `namespace inscope foo`:
///   tclsh9.0: `wrong # args: should be "namespace inscope name arg ?arg...?"`
///   VM:       `wrong # args: should be "namespace inscope namespace arg ?arg ...?"`
#[test]
fn namespace_inscope_eval_bare_wrong_args() {
    let (ok, msg, _) = run("namespace inscope foo");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "namespace inscope namespace arg ?arg ...?""#
    );
    let (ok, msg, _) = run("namespace eval");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "namespace eval name arg ?arg ...?""#
    );
}

/// `tcl::prefix match` with a single positional (one short of `table string`),
/// and over a malformed table value (the `entries` error path in match).
#[test]
fn prefix_match_one_arg_and_bad_table() {
    // tclsh (both): one positional is still too few.
    let (ok, msg, _) = run("tcl::prefix match x");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "tcl::prefix match ?options? table string""#
    );
    // tclsh (both): a malformed table surfaces the list parse error.
    let (ok, msg, _) = run("tcl::prefix match \"\\{\" z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `namespace code` at the global scope renders the namespace as `::` (the
/// `display_ns` empty-canonical branch).
#[test]
fn namespace_code_at_global_scope() {
    // tclsh (both): -> "::namespace inscope :: {x y}"
    assert_eq!(
        run("namespace code {x y}").1,
        "::namespace inscope :: {x y}"
    );
}

/// Bare `namespace inscope` (zero args) hits the `split_first` empty arm. (Same
/// VM-vs-tclsh wording divergence as the other inscope/eval usages — `namespace`
/// + `?arg ...?` rather than tclsh9.0's `name` + `?arg...?`.)
#[test]
fn namespace_inscope_zero_args() {
    let (ok, msg, _) = run("namespace inscope");
    assert!(!ok);
    assert_eq!(
        msg,
        r#"wrong # args: should be "namespace inscope namespace arg ?arg ...?""#
    );
}

/// Issue #1607: `info` and `file` are `TclMakeEnsemble` commands, so both the
/// prefix scan and the miss message belong to `tcl_cmd_core::ensemble`. The VM
/// used to emit the sentence without its `must be` clause.
///
/// tclsh 9.0.4:
///   info {}   -> unknown or ambiguous subcommand "": must be args, body,
///                class, cmdcount, cmdtype, commands, complete, constant,
///                consts, coroutine, default, errorstack, exists, frame,
///                functions, globals, hostname, level, library, loaded,
///                locals, nameofexecutable, object, patchlevel, procs, script,
///                sharedlibextension, tclversion, or vars
///   info e /  -> unknown or ambiguous subcommand "e": must be <same>
///   info ex x -> 0        (a unique prefix still resolves)
///   file {}   -> unknown or ambiguous subcommand "": must be atime,
///                attributes, channels, copy, delete, dirname, executable,
///                exists, extension, home, isdirectory, isfile, join, link,
///                lstat, mkdir, mtime, nativename, normalize, owned, pathtype,
///                readable, readlink, rename, rootname, separator, size,
///                split, stat, system, tail, tempdir, tempfile, tildeexpand,
///                type, volumes, or writable
///   file e /  -> unknown or ambiguous subcommand "e": must be <same>
///   file ex / -> unknown or ambiguous subcommand "ex": must be <same>
///                (exists/executable/extension all start with `ex`)
#[test]
fn info_and_file_ensemble_misses_carry_the_full_option_list() {
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("info \"\""),
        format!("unknown or ambiguous subcommand \"\": {INFO_MUST}")
    );
    assert_eq!(
        msg("info e x"),
        format!("unknown or ambiguous subcommand \"e\": {INFO_MUST}")
    );
    assert_eq!(run("set x 1; info ex x").1, "1");
    assert_eq!(
        msg("file \"\""),
        format!("unknown or ambiguous subcommand \"\": {FILE_MUST}")
    );
    assert_eq!(
        msg("file e /"),
        format!("unknown or ambiguous subcommand \"e\": {FILE_MUST}")
    );
    assert_eq!(
        msg("file ex /"),
        format!("unknown or ambiguous subcommand \"ex\": {FILE_MUST}")
    );
}

/// Issue #1607 follow-up: `file`'s table is the *selected release's* surface,
/// not a pinned Tcl 9 list. `home`, `tempdir` and `tildeexpand` arrive in 9.0,
/// and their presence changes the verdict for words that have nothing to do
/// with them — `file te` is a unique prefix of `tempfile` under 8.6 and
/// ambiguous under 9.0. The names and their gates come from the registry
/// through the one environment ingress seam, as the WASM runtime's `file`
/// already did.
///
/// tclsh8.6.16:
///   catch {file te x} m -> 0, a channel   (tempfile)
///   catch {file h} m    -> 1 unknown or ambiguous subcommand "h": must be
///                          … tail, tempfile, type, volumes, or writable
/// tclsh9.0.4:
///   catch {file te x} m -> 1 unknown or ambiguous subcommand "te": must be
///                          … tempdir, tempfile, tildeexpand, …
///   catch {file h} m    -> 0 /root        (home)
///
/// UNIMPLEMENTED: the VM implements neither `tempfile` nor `home`, so a word
/// that resolves to one still lands on the unknown-subcommand arm — under the
/// *canonical* name, which is exactly what shows the resolution happened, and
/// with the release's own enumeration.
#[test]
fn file_subcommand_table_follows_the_emulated_release() {
    const FILE_MUST_86: &str = "must be atime, attributes, channels, copy, delete, dirname, \
                                executable, exists, extension, isdirectory, isfile, join, \
                                link, lstat, mkdir, mtime, nativename, normalize, owned, \
                                pathtype, readable, readlink, rename, rootname, separator, \
                                size, split, stat, system, tail, tempfile, type, volumes, \
                                or writable";

    // 8.6: `te` is unique (no `tempdir`), so it resolves to `tempfile`; `h`
    // matches nothing, and the enumeration has no 9.0 names in it.
    let (ok, msg, _) = run_for_version("file te x", tcl_dialect::TclVersion::V8_6);
    assert!(!ok);
    assert_eq!(
        msg,
        format!("unknown or ambiguous subcommand \"tempfile\": {FILE_MUST_86}")
    );
    let (ok, msg, _) = run_for_version("file h", tcl_dialect::TclVersion::V8_6);
    assert!(!ok);
    assert_eq!(
        msg,
        format!("unknown or ambiguous subcommand \"h\": {FILE_MUST_86}")
    );

    // 9.0: `tempdir` makes `te` ambiguous, and `h` now resolves to `home`.
    let (ok, msg, _) = run_for_version("file te x", tcl_dialect::TclVersion::V9_0);
    assert!(!ok);
    assert_eq!(
        msg,
        format!("unknown or ambiguous subcommand \"te\": {FILE_MUST}")
    );
    let (ok, msg, _) = run_for_version("file h", tcl_dialect::TclVersion::V9_0);
    assert!(!ok);
    assert_eq!(
        msg,
        format!("unknown or ambiguous subcommand \"home\": {FILE_MUST}")
    );

    // A prefix that is unique in both releases still resolves in both.
    assert_eq!(
        run_for_version("file ext /a.b", tcl_dialect::TclVersion::V8_6).1,
        ".b"
    );
    assert_eq!(
        run_for_version("file ext /a.b", tcl_dialect::TclVersion::V9_0).1,
        ".b"
    );
}

/// Issue #1607 follow-up, the rest of the class: every `TclMakeEnsemble`
/// table this engine resolves against is a *release* fact, and the VM is
/// release-selectable. A 9-only name must not dispatch under an earlier pin,
/// and — the half that hides — must not change the prefix verdict for a word
/// that has nothing to do with it.
///
/// tclsh8.6.16 / tclsh9.0.4:
///   dict g {a 1} a     -> 1                    / ambiguous (getdef, getwithdefault)
///   string in abc 1    -> b                    / ambiguous (index, insert)
///   info cm            -> the cmdcount         / ambiguous (cmdcount, cmdtype)
///   info cmdtype set   -> unknown "cmdtype"    / native
///   array f x          -> unknown "f"          / `array for`'s own arity error
#[test]
fn ensemble_tables_follow_the_emulated_release() {
    let v86 = tcl_dialect::TclVersion::V8_6;
    let v90 = tcl_dialect::TclVersion::V9_0;

    // dict: `getdef`/`getwithdefault` are TIP 342 (Tcl 9).
    assert_eq!(run_for_version("dict g {a 1} a", v86).1, "1");
    let (ok, msg, _) = run_for_version("dict g {a 1} a", v90);
    assert!(!ok);
    assert!(
        msg.contains("get, getdef, getwithdefault,"),
        "9.0 must advertise both spellings: {msg}"
    );

    // string: `insert` is Tcl 9.
    assert_eq!(run_for_version("string in abc 1", v86).1, "b");
    let (ok, msg, _) = run_for_version("string in abc 1", v90);
    assert!(!ok);
    assert!(msg.contains("index, insert, is,"), "{msg}");

    // info: `cmdtype` is Tcl 9. The exact-name row shows that a resolution
    // miss must report rather than fall through — the dispatch arms match on
    // the canonical name, so `info cmdtype` would otherwise still run.
    // UNIMPLEMENTED: the VM has no `info cmdcount`, so the *resolved* word is
    // what the miss quotes — which is exactly the evidence that `cm` resolved
    // uniquely under 8.6 and did not under 9.0.
    let (ok, msg, _) = run_for_version("info cm", v86);
    assert!(!ok);
    assert!(
        msg.starts_with("unknown or ambiguous subcommand \"cmdcount\""),
        "8.6 must resolve `cm` to cmdcount: {msg}"
    );
    let (ok, msg, _) = run_for_version("info cmdtype set", v86);
    assert!(!ok);
    assert!(
        msg.starts_with("unknown or ambiguous subcommand \"cmdtype\": must be args, body, class,"),
        "{msg}"
    );
    let (ok, msg, _) = run_for_version("info cm", v90);
    assert!(!ok);
    assert!(
        msg.starts_with("unknown or ambiguous subcommand \"cm\""),
        "9.0 must leave `cm` ambiguous: {msg}"
    );
    assert!(msg.contains("cmdcount, cmdtype,"), "{msg}");

    // array: `for` is Tcl 9 (so is `default`, which this engine does not
    // dispatch). UNIMPLEMENTED: the enumeration is this engine's own
    // dispatched set, shorter than tclsh's, on both releases.
    let (ok, msg, _) = run_for_version("array f x", v86);
    assert!(!ok);
    assert!(
        msg.starts_with("unknown or ambiguous subcommand \"f\""),
        "{msg}"
    );
    assert!(!msg.contains("for"), "8.6 must not advertise for: {msg}");
    let (ok, msg, _) = run_for_version("array f x", v90);
    assert!(!ok);
    assert!(msg.contains("wrong # args"), "9.0 resolves `for`: {msg}");
}

/// The same rule below 8.6: `binary encode`/`decode` (TIP 317) arrive in 8.6
/// and `encoding dirs` in 8.5.
///
/// tclsh8.5.19: binary d base64 QQ== -> bad option "d": must be format or scan
/// tclsh8.6.16: binary d base64 QQ== -> A
/// tclsh8.4.20: encoding d           -> bad option "d": must be convertfrom,
///                                      convertto, names, or system
/// tclsh8.5.19: encoding d           -> the encoding directory list
///
/// The *sentence* below 8.6 is C's pre-ensemble `bad option` wording, which
/// this engine does not model — it speaks the ensemble sentence at every
/// release. The rows pin the resolution and the enumeration, not that wording.
#[test]
fn binary_and_encoding_tables_follow_the_emulated_release() {
    let v84 = tcl_dialect::TclVersion::V8_4;
    let v85 = tcl_dialect::TclVersion::V8_5;
    let v86 = tcl_dialect::TclVersion::V8_6;

    let (ok, msg, _) = run_for_version("binary d base64 QQ==", v85);
    assert!(!ok);
    // Two entries still take the ensemble owner's comma before `or`.
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"d\": must be format, or scan"
    );
    assert_eq!(run_for_version("binary d base64 QQ==", v86).1, "A");

    let (ok, msg, _) = run_for_version("encoding d", v84);
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"d\": must be convertfrom, convertto, names, or system"
    );
    assert!(
        run_for_version("encoding d", v85).0,
        "`dirs` exists from 8.5"
    );
}
