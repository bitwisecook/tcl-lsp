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

//! `regexp` / `regsub` command-level behaviour, covering Tcl 9's
//! `regexp.test` / `regexpComp.test` cases, run end-to-end through the VM (which
//! drives the pure-Rust `tcl-regex` ARE engine). Each expected value was
//! captured from real `tclsh` 9.0.3, so this exercises the full command stack
//! — option parsing, the match/advance loop, `-all`/`-inline`/`-indices`/
//! `-start`/`-line`/`-nocase`, submatch-variable assignment, and the `regsub`
//! substitution-spec expansion — over the faithful engine.

// The case table keeps uniform `r#"…"#` delimiters so patterns with and
// without embedded quotes line up; a few don't strictly need the hashes.
#![allow(clippy::needless_raw_string_hashes)]

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileService, Vm};

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

/// Compile + run `src`; return `(ok, result)`.
fn run(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let c = vm.run_module(&asm);
    (c.code.is_ok(), c.result.to_str().to_string())
}

/// `(Tcl source, expected result)` — every expected value verified against
/// `tclsh` 9.0.3.
const CASES: &[(&str, &str)] = &[
    (r#"regexp ab*c aaabbbccc"#, "1"),
    // `-about` end to end, including the `re_info` flag list the engine
    // records as it compiles — the half that needs `AreEngine::info_names`,
    // and the half Tcl's own `regexp-20.2` cannot discriminate because its
    // pattern has no flags. tclsh8.6.18 and tclsh9.0.4 agree on all three.
    (r#"regexp -about {a(b)c}"#, "1 {}"),
    (r#"regexp -about {(?:a)}"#, "0 REG_UNONPOSIX"),
    (r#"regexp -about {}"#, "0 {REG_UUNSPEC REG_UEMPTYMATCH}"),
    (r#"regexp -inline {a(b*)c} xabbbcx"#, "abbbc bbb"),
    (r#"regexp -indices -inline {a(b*)c} xabbbcx"#, "{1 5} {2 4}"),
    (r#"regexp -all -inline {[0-9]+} "a12b345c""#, "12 345"),
    (
        r#"list [regexp {(a)(b)?(c)} ac m a b c] $m $a $b $c"#,
        "1 ac a {} c",
    ),
    (r#"regexp -nocase {abc} XABCY"#, "1"),
    (r#"regexp -- -foo -foo"#, "1"),
    (r#"set s "a\nb"; regexp -line {^b} $s"#, "1"),
    (r#"regexp {^b} "a\nb""#, "0"),
    (r#"regexp -all -inline {a.} "axaybz""#, "ax ay"),
    (r#"regexp -start 3 {a} "aaaaa""#, "1"),
    (r#"regsub {b+} aabbbcc X"#, "aaXcc"),
    (r#"regsub -all {b} aabbbcc X"#, "aaXXXcc"),
    (r#"regsub {(a)(b)} ab {\2\1}"#, "ba"),
    (r#"regsub -all {[abc]} "abcd" {[&]}"#, "[a][b][c]d"),
    (r#"regsub -nocase {ABC} xabcy Q"#, "xQy"),
    // Word edges and classes — not expressible in an approximate regex engine.
    (r#"regexp {\mfoo\M} "a foo b""#, "1"),
    (r#"regexp {\w+} "  hi_there! ""#, "1"),
    // POSIX longest-match submatch semantics (differs from Perl leftmost-first).
    (r#"regexp -inline {(a*)(a*)} aaa"#, "aaa aaa {}"),
    (r#"regexp {(a|ab)(c|bcd)(d*)} abcd"#, "1"),
    (r#"regexp -inline {(a|ab)(c|bcd)(d*)} abcd"#, "abcd ab c d"),
    (r#"regexp {[[:digit:]]+} "ab123""#, "1"),
    (r#"regsub -all {(\w)(\w)} abcd {\2\1}"#, "badc"),
    (r#"regexp -inline {(\w+)@(\w+)} foo@bar"#, "foo@bar foo bar"),
    (r#"regsub {^} abc >"#, ">abc"),
    (r#"regexp -all -inline -indices {a} aXa"#, "{0 0} {2 2}"),
    (r#"regexp (?n)^b "a\nb""#, "1"),
    (
        r#"catch {regexp (*) x} e; set e"#,
        "cannot compile regular expression pattern: invalid quantifier operand",
    ),
];

#[test]
fn regexp_command_corpus() {
    let mut failures = Vec::new();
    for (src, expected) in CASES {
        let (ok, result) = run(src);
        if !ok || &result != expected {
            failures.push(format!(
                "  src=`{src}`\n    got ok={ok} result={result:?}, expected {expected:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} regexp/regsub command cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Compile + run `src` pinned to `version` (the release the VM reports and
/// the profile the compiler uses), returning `(ok, result)` — `regsub`'s
/// `-command` option only exists from 9.0, so its refusal needs a VM that is
/// not 9.0.
fn run_for_version(src: &str, version: tcl_dialect::TclVersion) -> (bool, String) {
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
    let c = vm.run_module(&asm);
    (c.code.is_ok(), c.result.to_str().to_string())
}

/// `regsub -command` evaluates the prefix once per substitution with the whole
/// match and each submatch appended. Verified on tclsh 9.0.4 and 9.1b0:
///
/// ```text
/// % regsub -command {.x.} {abcxdef} {string length}
/// ab3ef
/// % regsub -command -all {(.)(.)} {abcdef} {list ,}
/// , ab a b, cd c d, ef e f
/// % regsub -command {(a)|(b)} ab {list <}
/// < a a {}b
/// % set n [regsub -command {.x.} abcxdef {string length} out]; list $n $out
/// 1 ab3ef
/// % regsub -command {z} abc {string toupper}
/// abc
/// ```
#[test]
fn regsub_command_evaluates_the_prefix() {
    let cases: &[(&str, &str)] = &[
        (
            r#"regsub -command {.x.} {abcxdef} {string length}"#,
            "ab3ef",
        ),
        (
            r#"regsub -command -all {(.)(.)} {abcdef} {list ,}"#,
            ", ab a b, cd c d, ef e f",
        ),
        // A submatch that did not participate is an empty word, not a missing
        // one — the prefix still sees one word per submatch.
        (r#"regsub -command {(a)|(b)} ab {list <}"#, "< a a {}b"),
        (
            r#"set n [regsub -command {.x.} abcxdef {string length} out]; list $n $out"#,
            "1 ab3ef",
        ),
        // A pattern that never matches never calls the prefix.
        (r#"regsub -command {z} abc {string toupper}"#, "abc"),
    ];
    for (src, want) in cases {
        let (ok, result) = run(src);
        assert!(ok, "script errored: {result}\n  src: {src}");
        assert_eq!(&result.as_str(), want, "for script: {src}");
    }
}

/// A script error inside the `-command` prefix propagates, and C's context
/// frame is appended to `errorInfo`. tclsh 9.0.4 / 9.1b0:
///
/// ```text
/// % proc boomp args { error boom }
/// % catch {regsub -command {.x.} abcxdef boomp} e; set ::errorInfo
/// boom
///     while executing
/// "error boom "
///     (procedure "boomp" line 1)
///     invoked from within
/// "boomp cxd"
///     (-command substitution computation script)
///     invoked from within
/// "regsub -command {.x.} abcxdef boomp"
/// ```
///
/// The VM's trace omits the `invoked from within "boomp cxd"` frame because
/// the prefix is invoked argv-wise (`Vm::dispatch`), which carries no source
/// text to quote — the same pre-existing shape `lsort -command` has here
/// (tclsh prints `invoked from within "errchk 1 2"` for `lsort -command
/// errchk {1 2}`, the VM does not). Everything this change owns — the
/// message, the `(-command substitution computation script)` frame and its
/// position before the enclosing command's frame — matches C.
#[test]
fn regsub_command_error_appends_the_c_error_info_trailer() {
    let (ok, result) = run(r#"proc boomp args { error boom }
           catch {regsub -command {.x.} abcxdef boomp} e
           list $e $::errorInfo"#);
    assert!(ok, "script errored: {result}");
    let (msg, info) = result
        .split_once(' ')
        .expect("the list is the message then the trace");
    assert_eq!(msg, "boom");
    assert_eq!(
        info,
        "{boom\n    while executing\n\"error boom\"\n    (procedure \"boomp\" line 1)\n    \
         (-command substitution computation script)\n    invoked from within\n\"regsub -command \
         {.x.} abcxdef boomp\"}"
    );
}

/// A non-error completion code from the prefix propagates untouched, with no
/// `errorInfo` trailer. tclsh 9.0.4:
///
/// ```text
/// % proc q args { return -code continue }
/// % catch {regsub -command {.x.} {abcxdef} q} r
/// 4
/// % info exists ::errorInfo
/// 0
/// ```
#[test]
fn regsub_command_non_error_code_propagates_untouched() {
    let (ok, result) = run(r#"proc q args { return -code continue }
           list [catch {regsub -command {.x.} {abcxdef} q} r] $r [info exists ::errorInfo]"#);
    assert!(ok, "script errored: {result}");
    assert_eq!(result, "4 {} 0");
}

/// `-command` is a 9.0 option: before it, `regsub` rejects it in that
/// release's own noun and enumeration. The VM passes its pinned release to
/// the core, so the refusal follows `info patchlevel`:
///
/// ```text
/// $ tclsh8.4 / tclsh8.5   (8.4.20 / 8.5.19)
/// % catch {regsub -command {a} abc {string toupper}} r; set r
/// bad switch "-command": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --
/// $ tclsh8.6              (8.6.18)
/// bad option "-command": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --
/// $ tclsh9.0 / tclsh9.1   (9.0.4 / 9.1b0)
/// Abc
/// ```
#[test]
fn regsub_command_is_a_9_0_option() {
    use tcl_dialect::TclVersion;
    const ENUM: &str =
        "must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --";
    let src = r#"regsub -command {a} abc {string toupper}"#;
    for (version, want) in [
        (
            TclVersion::V8_4,
            format!(r#"bad switch "-command": {ENUM}"#),
        ),
        (
            TclVersion::V8_5,
            format!(r#"bad switch "-command": {ENUM}"#),
        ),
        (
            TclVersion::V8_6,
            format!(r#"bad option "-command": {ENUM}"#),
        ),
    ] {
        let (ok, result) = run_for_version(src, version);
        assert!(!ok, "{version:?}: expected a refusal, got {result}");
        assert_eq!(result, want, "for {version:?}");
    }
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let (ok, result) = run_for_version(src, version);
        assert!(ok, "{version:?}: {result}");
        assert_eq!(result, "Abc", "for {version:?}");
    }
}
