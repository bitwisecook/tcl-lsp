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

//! M11 — cross-version variable semantics, the only resolution-semantics
//! change across 8.4→9.1: Tcl 8.x resolves an unqualified variable at
//! **namespace scope** to the global variable when the namespace has none but
//! the global namespace does (reads *and* writes); Tcl 9.0 removed the
//! fallback (TIP 278 — 8.6 `tclVar.c:757` keeps it, 9.0 forces
//! `TCL_NAMESPACE_ONLY`, `tclVar.c:935`).
//!
//! Each vector runs through the bytecode VM **twice** — once at
//! `TclVersion::V8_6` and once at `V9_0` — and, when the matching real
//! tclsh is installed (`tclsh8.6` / `tclsh9.0` on PATH, or
//! `TCL_LSP_TCLSH86` / `TCL_LSP_TCLSH90`), the same script is executed under
//! it and must agree — so the table can never drift from C Tcl.
//!
//! The namespace fallback and non-local loop completions here are runtime
//! execution behaviour. They do not produce a distinct LSP request or VS Code
//! UI state, so `lsp_e2e` and extension tests would only duplicate this VM
//! oracle. Static expression availability remains covered by the
//! registry-backed `expr_surface_e2e` suite.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        compile_exact_profile(src, profile)
    }
}

fn compile_exact_profile(
    src: &str,
    profile: &'static DialectProfile,
) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error_with_config(src, config) {
        return Err(CompileError(msg));
    }
    let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
        src,
        registry,
        config,
        Some(profile),
    );
    let cfg = build_cfg_codegen(&ir, false);
    Ok(codegen_module(&cfg, &ir, registry))
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run `src` in the VM at `version`; the script's `puts` output is returned
/// (the vectors communicate through stdout so the tclsh leg is directly
/// comparable).
fn vm_output(src: &str, version: TclVersion) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = CompilerSvc {
        registry: CommandRegistry::build_default(),
    };
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&cap.0.borrow()).trim().to_string()
}

/// Run `src` under a real tclsh, or `None` when that binary isn't available.
fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    use std::io::Write as _;
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(bin_env) {
        candidates.push(explicit);
    }
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(&name)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(src.as_bytes())
            .expect("write");
        let out = child.wait_with_output().expect("run");
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
        }
    }
    None
}

/// One behaviour vector: the script prints its observations; `want_8x` and
/// `want_90` are the full expected stdout under each semantic.
struct Vector {
    name: &'static str,
    script: &'static str,
    want_8x: &'static str,
    want_90: &'static str,
}

const VECTORS: &[Vector] = &[
    Vector {
        name: "read falls back to global only in 8.x",
        script: "set g GLOBAL\nnamespace eval foo { puts [catch {set g} m]:$m }\n",
        want_8x: "0:GLOBAL",
        want_90: "1:can't read \"g\": no such variable",
    },
    Vector {
        name: "write reaches the global in 8.x, creates in the namespace in 9.0",
        script: "set g OLD\nnamespace eval foo { set g NEW }\nputs [info exists ::foo::g]:[set ::g]\n",
        want_8x: "0:NEW",
        want_90: "1:OLD",
    },
    Vector {
        name: "a declared-but-unset `variable` blocks the fallback",
        script: "set v GLOBALV\nnamespace eval bar { variable v; puts [catch {set v}] }\n",
        want_8x: "1",
        want_90: "1",
    },
    Vector {
        name: "with neither cell, a write creates in the namespace",
        script: "namespace eval foo { set fresh NS }\nputs [info exists ::foo::fresh]:[info exists ::fresh]\n",
        want_8x: "1:0",
        want_90: "1:0",
    },
    Vector {
        name: "info exists agrees with the read rule",
        script: "set g G\nnamespace eval foo { puts [info exists g] }\n",
        want_8x: "1",
        want_90: "0",
    },
    Vector {
        name: "unset reaches the global in 8.x",
        script: "set g G\nputs [catch {namespace eval foo { unset g }}]:[info exists ::g]\n",
        want_8x: "0:0",
        want_90: "1:1",
    },
    Vector {
        name: "incr through the fallback mutates the global in 8.x",
        script: "set ctr 5\nnamespace eval foo { incr ctr }\nputs [set ::ctr]:[info exists ::foo::ctr]\n",
        want_8x: "6:0",
        want_90: "5:1",
    },
    Vector {
        name: "array element reads fall back in 8.x",
        script: "set arr(k) AV\nnamespace eval foo { puts [catch {set arr(k)} m]:$m }\n",
        want_8x: "0:AV",
        want_90: "1:can't read \"arr(k)\": no such variable",
    },
    Vector {
        name: "procs never use the namespace-scope fallback",
        script: "set g G\nnamespace eval foo { proc p {} { catch {set g} m; return $m } }\nputs [foo::p]\n",
        want_8x: "can't read \"g\": no such variable",
        want_90: "can't read \"g\": no such variable",
    },
    // --- issue #1328: the rule governs *relative variable resolution*, so it
    // reaches every command that resolves a relative name, not just `append`.
    Vector {
        name: "append reaches the global in 8.x (the shape issue #1328 was filed on)",
        script: "set g foo\nnamespace eval n { append g baz }\nputs [set ::g]:[info exists ::n::g]\n",
        want_8x: "foobaz:0",
        want_90: "foo:1",
    },
    Vector {
        name: "lappend reaches the global in 8.x",
        script: "set g a\nnamespace eval n { lappend g b }\nputs [set ::g]:[info exists ::n::g]\n",
        want_8x: "a b:0",
        want_90: "a:1",
    },
    Vector {
        name: "$-substitution falls back in 8.x",
        script: "set g VAL\nnamespace eval n { puts [catch {set x \"<$g>\"} m]:$m }\n",
        want_8x: "0:<VAL>",
        want_90: "1:can't read \"g\": no such variable",
    },
    Vector {
        name: "an array-element write reaches the global array in 8.x",
        script: "array set A {k v}\nnamespace eval n { set A(k) NEW }\nputs [array get ::A]:[info exists ::n::A]\n",
        want_8x: "k NEW:0",
        want_90: "k v:1",
    },
    // The fallback is "current, then global" — never the intermediate parents.
    // Both halves matter: a nested namespace still reaches the *global* (so the
    // depth of nesting is irrelevant), and a parent's variable is never found.
    Vector {
        name: "a nested namespace still falls back to the global in 8.x",
        script: "set g G\nnamespace eval outer { namespace eval inner { puts [catch {set g} m]:$m } }\n",
        want_8x: "0:G",
        want_90: "1:can't read \"g\": no such variable",
    },
    Vector {
        name: "an intermediate parent's variable is never found, in either release",
        script: "namespace eval P { variable pv PARENT\n  namespace eval C { puts [catch {set pv} m]:$m } }\n",
        want_8x: "1:can't read \"pv\": no such variable",
        want_90: "1:can't read \"pv\": no such variable",
    },
    Vector {
        name: "an existing namespace variable shadows the global in both releases",
        script: "set g GLOBAL\nnamespace eval n { variable g NS }\nnamespace eval n { puts [set g]:[set ::g] }\n",
        want_8x: "NS:GLOBAL",
        want_90: "NS:GLOBAL",
    },
    Vector {
        name: "namespace upvar aliases an existing namespace cell",
        script: "namespace eval cfg { set value before }\nproc mutate {} { namespace upvar ::cfg value local\n set local after\n puts $local:$::cfg::value }\nmutate\n",
        want_8x: "after:after",
        want_90: "after:after",
    },
    Vector {
        name: "namespace upvar leaves a missing target unset until written",
        script: "namespace eval cfg {}\nproc probe {} { namespace upvar ::cfg absent local\n puts [info exists local]\n set local made }\nprobe\nputs $::cfg::absent\n",
        want_8x: "0\nmade",
        want_90: "0\nmade",
    },
    Vector {
        name: "namespace upvar resolves a relative namespace from the current namespace",
        script: "namespace eval outer { namespace eval child { set value K }\n namespace upvar child value alias\n puts $alias }\n",
        want_8x: "K",
        want_90: "K",
    },
    Vector {
        name: "namespace upvar keeps byte array identity across append shimmering",
        script: "set raw [binary format H* 80ff]\nnamespace eval bytes { set value $::raw }\nproc shimmer {} { namespace upvar ::bytes value local\n puts [binary encode hex $local]\n append local A\n puts [binary encode hex $::bytes::value] }\nshimmer\n",
        want_8x: "80ff\n80ff41",
        want_90: "80ff\n80ff41",
    },
    Vector {
        name: "an absolute namespace-upvar source ignores the namespace argument",
        script: "namespace eval A { set value A }\nnamespace eval B { set value B }\nproc probe {} { namespace upvar ::A ::B::value local\n puts $local }\nprobe\n",
        want_8x: "B",
        want_90: "B",
    },
    Vector {
        name: "namespace upvar reports missing namespaces and incomplete pairs",
        script: "puts [catch {namespace upvar ::missing value local} msg]:$msg\nputs [catch {namespace upvar :: value} msg]:$msg\n",
        want_8x: "1:namespace \"::missing\" not found\n1:wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"",
        want_90: "1:namespace \"::missing\" not found\n1:wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"",
    },
    //
    // The user-visible consequence of getting this wrong: under 8.x the write
    // reaches the global, so the *global's* write trace must fire.
    Vector {
        name: "a write through the 8.x fallback fires the global's write trace",
        script: "proc log {n1 n2 op} { puts \"trace:$n1:$op\" }\nset g foo\ntrace add variable g write log\nnamespace eval n { set g X }\nputs [set ::g]\n",
        want_8x: "trace:g:write\nX",
        want_90: "foo",
    },
    // A store returns the variable read back *after* its write traces
    // (`TclPtrSetVarIdx`, issue #1633 row 1) — and C reads back the very `Var`
    // it wrote, not the name a second time. Only 8.x can tell the two apart,
    // because only 8.x lets a callback change which cell a bare name reaches:
    // here the write lands on `::x` through the fallback, and the callback
    // creates `::n::x` beside it. Re-resolving would answer `NSVAL`.
    Vector {
        name: "a write callback creating the namespace cell cannot move the read-back",
        script: "proc mk {n1 n2 op} { set ::n::x NSVAL }\n\
                 set ::x GLOBAL\n\
                 namespace eval n {}\n\
                 trace add variable ::x write mk\n\
                 namespace eval n { puts [set x V] }\n\
                 puts [set ::x]:[info exists ::n::x]\n",
        want_8x: "V\nV:1",
        want_90: "V\nGLOBAL:1",
    },
    // The mirror image: the name reaches `::m::y` and the callback unsets it,
    // so the read-back finds the cell gone and the store is empty at both
    // releases. Re-resolving would fall back to `::y` at 8.x and answer
    // `GLOBAL2`.
    Vector {
        name: "a write callback unsetting the cell leaves the store empty, with no fallback",
        script: "proc rm {n1 n2 op} { unset ::m::y }\n\
                 set ::y GLOBAL2\n\
                 namespace eval m {}\n\
                 set ::m::y NSVAL2\n\
                 trace add variable ::m::y write rm\n\
                 namespace eval m { puts <[set y V]> }\n\
                 puts [info exists ::m::y]:[set ::y]\n",
        want_8x: "<>\n0:GLOBAL2",
        want_90: "<>\n0:GLOBAL2",
    },
];

#[test]
fn vm_matches_the_pinned_cross_version_vectors() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want_8x,
            "[8.6] {}",
            v.name,
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want_90,
            "[9.0] {}",
            v.name,
        );
    }
}

#[test]
fn namespace_upvar_release_surface_and_zero_pair_arity_are_registry_driven() {
    let unavailable = vm_output(
        "puts [catch {namespace upvar :: value local} msg]:$msg\n",
        TclVersion::V8_4,
    );
    assert!(
        unavailable.starts_with("1:unknown or ambiguous subcommand \"upvar\":"),
        "Tcl 8.4 must not expose the 8.5 subcommand: {unavailable}"
    );

    assert_eq!(
        vm_output(
            "set ::value ok\nnamespace upvar :: value local\nputs $local\n",
            TclVersion::V8_5,
        ),
        "ok"
    );
    assert_eq!(
        vm_output(
            "puts [catch {namespace upvar ::} msg]:$msg\n",
            TclVersion::V8_5,
        ),
        "1:wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\""
    );

    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output("puts [catch {namespace upvar ::} msg]:$msg\n", version),
            "0:",
            "the zero-pair form is a no-op from Tcl 8.6: {version:?}"
        );
    }
}

/// The table itself is pinned to C Tcl: every vector's `want` must match what
/// the matching real tclsh prints.  Skips silently per-binary when not
/// installed (CI / dev machines with `make ensure-test-deps` have both).
#[test]
fn vectors_match_real_tclsh() {
    let mut ran = 0;
    for v in VECTORS {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], v.script) {
            assert_eq!(got, v.want_8x, "[tclsh8.6] {}", v.name);
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], v.script) {
            assert_eq!(got, v.want_90, "[tclsh9.0] {}", v.name);
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}

/// M10.1 — the command-resolution `namespace path` tier is a Tcl 8.5 feature
/// (TIP 181): under an 8.4 runtime a bare call resolves current-namespace →
/// global only, never through the path. Real tclsh8.4 rejects the `namespace
/// path` subcommand outright, so the script catches that release-gated error
/// before probing resolution. The analyser applies the same gate at its
/// recording site (`bare_call_honours_namespace_path_only_from_8_5`).
#[test]
fn namespace_path_resolution_tier_is_8_5_plus_m10() {
    let script = "proc ::helper {} { return GLOBAL }\n\
         namespace eval ::mymod { proc helper {} { return MYMOD } }\n\
         namespace eval ::app { catch {namespace path ::mymod} }\n\
         namespace eval ::app { puts [helper] }\n";
    assert_eq!(
        vm_output(script, TclVersion::V8_4),
        "GLOBAL",
        "8.4 has no path tier"
    );
    for v in [TclVersion::V8_5, TclVersion::V8_6, TclVersion::V9_0] {
        assert_eq!(
            vm_output(script, v),
            "MYMOD",
            "8.5+ resolves through the namespace path"
        );
    }
}

/// Completion vectors exercise the shared compiler/VM hand-off instead of a
/// command implementation.  C Tcl 8.6 and 9.0 agree on every vector, so an
/// unhandled completion produced by a proc or `try` phase must be caught by the
/// enclosing foreach's opcode-layout targets.
struct CompletionVector {
    /// TP/TN/FP category, retained in failures so a changed completion path is
    /// immediately distinguishable from a changed loop result.
    category: &'static str,
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const COMPLETION_VECTORS: &[CompletionVector] = &[
    CompletionVector {
        category: "TP",
        name: "a proc-returned continue advances the existing foreach iterator",
        script: "proc skip {} { return -code continue }\nforeach i {1 2 3} { if {$i == 2} { skip }; puts $i }\n",
        want: "1\n3",
    },
    CompletionVector {
        category: "TP",
        name: "an unhandled try continue advances the existing foreach iterator",
        script: "foreach i {1 2 3} { try { if {$i == 2} { continue }; puts $i } }\n",
        want: "1\n3",
    },
    CompletionVector {
        category: "TP",
        name: "an unhandled try break clears the iterator and exits the foreach",
        script: "foreach i {1 2 3} { try { if {$i == 2} { break }; puts $i } }\n",
        want: "1",
    },
    CompletionVector {
        category: "TP",
        name: "a directly nested foreach keeps the outer iterator state",
        script: "foreach outer {A B} { foreach inner {1 2} { puts \"$outer$inner\" } }\n",
        want: "A1\nA2\nB1\nB2",
    },
    CompletionVector {
        category: "TP",
        name: "lmap continue skips collection and advances its iterator",
        script: "puts [lmap i {1 2 3} { if {$i == 2} { continue }; set i }]\n",
        want: "1 3",
    },
    CompletionVector {
        category: "TP",
        name: "lmap break retains only completed iterations",
        script: "puts [lmap i {1 2 3} { if {$i == 2} { break }; set i }]\n",
        want: "1",
    },
    CompletionVector {
        category: "TN",
        name: "a normal foreach fall-through still visits every element",
        script: "foreach i {1 2 3} { puts $i }\n",
        want: "1\n2\n3",
    },
    CompletionVector {
        category: "FP",
        name: "a try on-continue handler consumes its own completion",
        script: "foreach i {1 2 3} {\n  try { if {$i == 2} { continue }; puts $i } on continue {} { puts handled }\n}\n",
        want: "1\nhandled\n3",
    },
];

#[test]
fn foreach_completion_handoff_matches_c_tcl_8_and_9() {
    for vector in COMPLETION_VECTORS {
        for version in [TclVersion::V8_6, TclVersion::V9_0] {
            assert_eq!(
                vm_output(vector.script, version),
                vector.want,
                "[{}] [{version:?}] {}",
                vector.category,
                vector.name,
            );
        }
    }
}

#[test]
fn foreach_completion_vectors_match_real_tclsh() {
    let mut ran = 0;
    for vector in COMPLETION_VECTORS {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], vector.script) {
            assert_eq!(
                got, vector.want,
                "[tclsh8.6] [{}] {}",
                vector.category, vector.name
            );
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], vector.script) {
            assert_eq!(
                got, vector.want,
                "[tclsh9.0] [{}] {}",
                vector.category, vector.name
            );
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}

/// The `${…}` close rule is release-specific, and `subst` must follow it.
///
/// `Tcl_ParseVarName` delimits the brace form differently across the supported
/// range: 8.4–8.6 (`tclParse.c(8.6.16):1398`) end the name at the **first**
/// literal `}`, while 9.0+ (`tclParse.c(9.0.4):1315`) count nested `{…}` and
/// consume `\X` as an inert pair. So `subst {${a{b}c}}` reads variable `a{b`
/// under 8.x — an error, since the variable is named `a{b}c` — and reads
/// `a{b}c` under 9.x.
///
/// The VM's `subst` engine scanned to the first `}` unconditionally, so it gave
/// the 8.x answer at *every* release (issue #1457). Both engines now resolve
/// the form through the one owner, `tcl_lexer::braced_var_name_end`.
const BRACED_VAR_SUBST_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {subst {${a{b}c}}} m]} { puts \"error:$m\" } else { puts \"ok:$m\" }\n",
);

/// The other half of the same rule: an escaped `}` is inert under the 9.x rule
/// only, so `${a\}b}` names `a\}b` there and `a\` under 8.x.
const BRACED_VAR_ESCAPE_SCRIPT: &str = concat!(
    "set {a\\}b} ESC\n",
    "if {[catch {subst {${a\\}b}}} m]} { puts \"error:$m\" } else { puts \"ok:$m\" }\n",
);

#[test]
fn subst_braced_var_close_rule_follows_the_emulated_release() {
    // 8.x family: the name is `a{b`, which no variable is called.
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        assert_eq!(
            vm_output(BRACED_VAR_SUBST_SCRIPT, version),
            "error:can't read \"a{b\": no such variable",
            "{version:?} must use the 8.x first-close rule"
        );
    }
    // 9.x: nested braces balance, so the whole `a{b}c` is the name.
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(BRACED_VAR_SUBST_SCRIPT, version),
            "ok:WORLD",
            "{version:?} must use the 9.x nesting rule"
        );
    }
    assert_eq!(
        vm_output(BRACED_VAR_ESCAPE_SCRIPT, TclVersion::V9_0),
        "ok:ESC",
        "9.x consumes an escaped close-brace as an inert pair"
    );
    assert!(
        vm_output(BRACED_VAR_ESCAPE_SCRIPT, TclVersion::V8_6).starts_with("error:"),
        "8.x closes the name at the first literal close-brace"
    );
}

/// An unterminated `${…}` — the outcome the close rule's `Option` had no
/// contract for, and where the two engines diverged from C and from each other
/// (issue #1457).
///
/// C raises `missing close-brace for variable name` on **both** releases, but
/// *which* templates count as unterminated is release-specific: the 9.x nesting
/// rule widens it, so `${a{b}` and `${a\}` close under 8.x (naming `a{b` and
/// `a\`) yet run off the end under 9.x.
///
/// Every template is built with `binary format H*` so the script parser's own
/// brace matching cannot reshape the bytes before `subst` sees them.
const UNTERMINATED_SCRIPT: &str = concat!(
    "foreach {label hex} {\n",
    "  plain_unterm    247B616263\n",   // `${abc`
    "  open_brace      247B617B62\n",   // `${a{b`
    "  esc_close       247B615C7D\n",   // `${a\}`
    "  trailing_bslash 247B615C\n",     // `${a\`
    "  closes_in_8x    247B617B627D\n", // `${a{b}`
    "} {\n",
    "  set t [binary format H* $hex]\n",
    "  if {[catch {subst $t} m]} { puts \"$label:$m\" } else { puts \"$label:ok:$m\" }\n",
    "}\n",
);

/// The `[...]` before a bad `${` has already run when the error is raised —
/// C evaluates the template left to right and keeps the side effect.
const UNTERMINATED_ORDER_SCRIPT: &str = concat!(
    "set c 0\n",
    "catch {subst [binary format H* 5B696E637220635D247B617B62]} m\n",
    "puts \"$m c=$c\"\n",
);

#[test]
fn unterminated_braced_var_raises_on_both_releases() {
    const MSG: &str = "missing close-brace for variable name";
    // 8.x: the first literal `}` closes, so only the templates with no `}` at
    // all are unterminated.
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        assert_eq!(
            vm_output(UNTERMINATED_SCRIPT, version),
            format!(
                "plain_unterm:{MSG}\n\
                 open_brace:{MSG}\n\
                 esc_close:can't read \"a\\\": no such variable\n\
                 trailing_bslash:{MSG}\n\
                 closes_in_8x:can't read \"a{{b\": no such variable"
            ),
            "{version:?} must use the 8.x first-close rule"
        );
    }
    // 9.x: nesting and inert `\X` pairs make three more of them unterminated.
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(UNTERMINATED_SCRIPT, version),
            format!(
                "plain_unterm:{MSG}\n\
                 open_brace:{MSG}\n\
                 esc_close:{MSG}\n\
                 trailing_bslash:{MSG}\n\
                 closes_in_8x:{MSG}"
            ),
            "{version:?} must use the 9.x nesting rule"
        );
    }
    // Raised in evaluation order, so the earlier `[incr c]` kept its effect.
    for version in [TclVersion::V8_6, TclVersion::V9_0] {
        assert_eq!(
            vm_output(UNTERMINATED_ORDER_SCRIPT, version),
            format!("{MSG} c=1"),
            "{version:?} must raise only once the walk reaches the bad `${{`"
        );
    }
}

#[test]
fn braced_var_close_rule_matches_real_tclsh() {
    let mut ran = 0;
    for script in [
        BRACED_VAR_SUBST_SCRIPT,
        BRACED_VAR_ESCAPE_SCRIPT,
        UNTERMINATED_SCRIPT,
        UNTERMINATED_ORDER_SCRIPT,
    ] {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}

// The COMPILED-WORD `${…}` path (issue #1568)
//
// The scripts above drive `subst`, an *interpreted* engine. These drive the
// compiler's normalised-word round-trip instead: the segmenter re-spells a
// `Var` token as source-like text and codegen decodes that spelling back. The
// two paths were fixed separately because they failed differently — #1457's
// engines hard-coded the 8.x rule at every release, while here the *encoder*
// discarded the braced form and the *two decoders* applied opposite rules, so
// the compiled path was wrong in both directions at once: 8.x produced the 9.x
// answer and 9.x substituted nothing at all.
//
// Keeping both sets in one file makes the two paths' agreement visible.

/// Assignment position — `set r ${a{b}c}`, the form #1568 was filed with.
const COMPILED_BRACED_VAR_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {set r ${a{b}c}} m]} { puts \"error:$m\" } else { puts \"ok:$r\" }\n",
);

/// Argument position — `puts ${a{b}c}` lowers differently from an assignment.
const COMPILED_BRACED_VAR_ARG_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {puts ${a{b}c}} m]} { puts \"error:$m\" }\n",
);

/// Inside a proc body — the local-variable-table path, a third lowering.
const COMPILED_BRACED_VAR_PROC_SCRIPT: &str = concat!(
    "proc f {} {\n",
    "    set {a{b}c} WORLD\n",
    "    if {[catch {set r ${a{b}c}} m]} { puts \"error:$m\" } else { puts \"ok:$r\" }\n",
    "}\n",
    "f\n",
);

/// The escape half: `\}` is an inert pair under 9.x only.
const COMPILED_BRACED_VAR_ESCAPE_SCRIPT: &str = concat!(
    "set {a\\}b} ESC\n",
    "if {[catch {set r ${a\\}b}} m]} { puts \"error:$m\" } else { puts \"ok:$r\" }\n",
);

/// Ordinary braced references that every release spells identically — the
/// regression guard. A fix that made the release rule reachable must not have
/// disturbed the overwhelming majority of `${…}` words, including the array
/// element form, whose parentheses are name characters in the brace form.
const COMPILED_PLAIN_BRACED_SCRIPT: &str =
    concat!("set a X\n", "set arr(k) v\n", "puts ${a}-${arr(k)}\n",);

#[test]
fn compiled_braced_var_close_rule_follows_the_emulated_release() {
    // The argument-position script prints the value straight from `puts`, so
    // it has no `ok:` prefix to echo; the other two report through a result
    // variable. Carrying the expected success text per script keeps each one
    // asserting its own real output rather than a shared guess.
    for (label, script, ok) in [
        ("assignment", COMPILED_BRACED_VAR_SCRIPT, "ok:WORLD"),
        ("argument", COMPILED_BRACED_VAR_ARG_SCRIPT, "WORLD"),
        ("proc body", COMPILED_BRACED_VAR_PROC_SCRIPT, "ok:WORLD"),
    ] {
        // 8.x family: the name ends at the first literal `}`, so it is `a{b` —
        // which no variable is called — and `c}` is ordinary word text.
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            assert_eq!(
                vm_output(script, version),
                "error:can't read \"a{b\": no such variable",
                "{label} at {version:?} must use the 8.x first-close rule"
            );
        }
        // 9.x: nested braces balance, so the whole `a{b}c` is the name. Before
        // the fix this emitted the literal text `$a{b}c` — no substitution at
        // all — because the segmenter had discarded the braced spelling.
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                vm_output(script, version),
                ok,
                "{label} at {version:?} must use the 9.x nesting rule"
            );
        }
    }
}

#[test]
fn compiled_braced_var_escape_follows_the_emulated_release() {
    assert_eq!(
        vm_output(COMPILED_BRACED_VAR_ESCAPE_SCRIPT, TclVersion::V9_0),
        "ok:ESC",
        "9.x consumes an escaped close-brace as an inert pair"
    );
    assert!(
        vm_output(COMPILED_BRACED_VAR_ESCAPE_SCRIPT, TclVersion::V8_6).starts_with("error:"),
        "8.x closes the name at the first literal close-brace"
    );
}

#[test]
fn ordinary_braced_var_references_are_unchanged_at_every_release() {
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            vm_output(COMPILED_PLAIN_BRACED_SCRIPT, version),
            "X-v",
            "{version:?} must still resolve a plain ${{name}} and ${{arr(k)}}"
        );
    }
}

/// The compiled path pinned against real tclsh, the sibling of
/// [`braced_var_close_rule_matches_real_tclsh`] for the `subst` scripts.
///
/// Skips loudly (and says which binary was missing) rather than passing
/// silently when no oracle is installed.
#[test]
fn compiled_braced_var_close_rule_matches_real_tclsh() {
    let mut ran = 0;
    for script in [
        COMPILED_BRACED_VAR_SCRIPT,
        COMPILED_BRACED_VAR_ARG_SCRIPT,
        COMPILED_BRACED_VAR_PROC_SCRIPT,
        COMPILED_BRACED_VAR_ESCAPE_SCRIPT,
        COMPILED_PLAIN_BRACED_SCRIPT,
    ] {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}

/// An **interpolated** word — `"pre${a{b}c}post"` — reaches neither of the
/// compiler's two decoders: the codegen declines to decompose it, so the whole
/// word is interned as one literal and the VM's `subst::subst_word` performs
/// the substitution at run time. That runtime decoder is a *third* pair of
/// `${…}` scans, and it hard-coded the 8.x first-close rule at every release
/// just as the compile-time ones did (issue #1568).
///
/// This vector exists because a mutation pass proved the compile-time fix
/// alone was not observable here: reverting `parse_subst_template` left every
/// test green. The gap was real — the interpolated path was still wrong at
/// 9.x — and this is the vector that fails when it is.
const COMPILED_INTERPOLATED_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {puts \"pre${a{b}c}post\"} m]} { puts \"error:$m\" }\n",
);

/// A `switch` **subject** takes its own lowering (`switch_subject_operand`
/// picks a `Raw` operand over a substituted `String`), so it is a distinct
/// route to the same decoders.
const COMPILED_SWITCH_SUBJECT_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {switch -- ${a{b}c} { WORLD { puts hit } default { puts miss } }} m]} ",
    "{ puts \"error:$m\" }\n",
);

#[test]
fn compiled_interpolated_and_switch_paths_follow_the_emulated_release() {
    for (label, script, ok) in [
        (
            "interpolated word",
            COMPILED_INTERPOLATED_SCRIPT,
            "preWORLDpost",
        ),
        ("switch subject", COMPILED_SWITCH_SUBJECT_SCRIPT, "hit"),
    ] {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            assert_eq!(
                vm_output(script, version),
                "error:can't read \"a{b\": no such variable",
                "{label} at {version:?} must use the 8.x first-close rule"
            );
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                vm_output(script, version),
                ok,
                "{label} at {version:?} must use the 9.x nesting rule"
            );
        }
    }
}

/// Both new routes pinned against real tclsh.
#[test]
fn compiled_interpolated_and_switch_paths_match_real_tclsh() {
    let mut ran = 0;
    for script in [COMPILED_INTERPOLATED_SCRIPT, COMPILED_SWITCH_SUBJECT_SCRIPT] {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}

/// An **array index** carrying a `${…}` substitution is the route that reaches
/// `codegen::helpers::parse_subst_template` — the compiler's *other* `${…}`
/// decoder, the one that hard-coded the 8.x first-close rule while
/// `parse_simple_var_ref` hard-coded the 9.x nesting rule (issue #1568).
///
/// These vectors exist because a mutation pass proved the rest of the suite
/// could not see that decoder at all: reverting it to `find('}')` left every
/// other test green. The array-index path is where it is observable, and each
/// of the three shapes below fails when it is reverted.
const COMPILED_ARRAY_INDEX_READ_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "set arr(WORLD) V\n",
    "if {[catch {puts $arr(${a{b}c})} m]} { puts \"error:$m\" }\n",
);

/// The store direction: the index is decoded when writing, too.
const COMPILED_ARRAY_INDEX_WRITE_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {set arr(${a{b}c}) V} m]} { puts \"error:$m\" } ",
    "else { puts [array names arr] }\n",
);

/// The same index nested inside an interpolated word, so both decoders are on
/// the path at once.
const COMPILED_ARRAY_INDEX_INTERP_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "set arr(WORLD) V\n",
    "if {[catch {puts \"v=$arr(${a{b}c})\"} m]} { puts \"error:$m\" }\n",
);

#[test]
fn compiled_array_index_braced_var_follows_the_emulated_release() {
    for (label, script, ok) in [
        ("array read", COMPILED_ARRAY_INDEX_READ_SCRIPT, "V"),
        ("array write", COMPILED_ARRAY_INDEX_WRITE_SCRIPT, "WORLD"),
        (
            "array read in a word",
            COMPILED_ARRAY_INDEX_INTERP_SCRIPT,
            "v=V",
        ),
    ] {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            assert_eq!(
                vm_output(script, version),
                "error:can't read \"a{b\": no such variable",
                "{label} at {version:?} must use the 8.x first-close rule"
            );
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                vm_output(script, version),
                ok,
                "{label} at {version:?} must use the 9.x nesting rule"
            );
        }
    }
}

#[test]
fn compiled_array_index_braced_var_matches_real_tclsh() {
    let mut ran = 0;
    for script in [
        COMPILED_ARRAY_INDEX_READ_SCRIPT,
        COMPILED_ARRAY_INDEX_WRITE_SCRIPT,
        COMPILED_ARRAY_INDEX_INTERP_SCRIPT,
    ] {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}

/// Issue #1732 — the Tcl 9 parser rejects raw brace bytes in an *array read*
/// before evaluation, while Tcl 8 keeps them as key text. The assignment is
/// deliberately outside the caught script: store-side `set a({key}) V` is an
/// ordinary word and remains valid at every release.
#[test]
fn compiled_array_index_source_mask_follows_the_emulated_release() {
    let script = concat!(
        "set a({key}) V\n",
        "puts [catch {set ignored $a({key})} message]:$message\n",
    );
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        assert_eq!(vm_output(script, version), "0:V", "{version:?}");
    }
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(script, version),
            "1:invalid character in array index",
            "{version:?}"
        );
    }
}

// Backslash-carrying `${…}` names (adversarial review of the #1568 fix)
//
// Every vector above uses a name whose only awkward character is a brace. A
// name carrying a **backslash** is a distinct shape, and it caught three
// separate mistakes the brace vectors could not see:
//
//   * `push_array_key` hand-rolled a fourth copy of the close rule, so a whole
//     `${…}` key containing a backslash fell through to the literal arm and had
//     its escapes decoded — wrong at 9.x for `${a\}b}` and at *every* release
//     for `${a\{b}`;
//   * the CFG switch-subject gate is wrong under `FirstClose` and wrong when
//     it requires both rules to agree — at both releases;
//   * declining a reference in `parse_simple_var_ref` is not free, because the
//     runtime fallback does not round-trip such a name.

/// A whole `${…}` **array key** whose name carries an escaped close-brace.
const COMPILED_KEY_ESCAPED_CLOSE_SCRIPT: &str = concat!(
    "set {a\\}b} K\n",
    "set arr(K) V\n",
    "if {[catch {puts $arr(${a\\}b})} m]} { puts \"error:$m\" }\n",
);

/// The same key in the **store** direction.
const COMPILED_KEY_ESCAPED_WRITE_SCRIPT: &str = concat!(
    "set {a\\}b} K\n",
    "if {[catch {set arr(${a\\}b}) V} m]} { puts \"error:$m\" } ",
    "else { puts [array names arr] }\n",
);

/// An escaped **open**-brace name. Both releases agree here — the `\{` is
/// inert under 9.x nesting and an ordinary name character under the 8.x
/// first-close rule — so a single wrong answer is wrong everywhere. This is
/// the vector that was broken at all five releases.
const COMPILED_KEY_ESCAPED_OPEN_SCRIPT: &str = concat!(
    "set {a\\{b} K\n",
    "set arr(K) V\n",
    "if {[catch {puts $arr(${a\\{b})} m]} { puts \"error:$m\" }\n",
);

/// A `switch` subject whose name carries an escaped close-brace — the program
/// that disproved "the close rule is immaterial in the CFG gate".
const COMPILED_SWITCH_ESCAPED_SCRIPT: &str = concat!(
    "set {a\\}b} K\n",
    "if {[catch {switch -- ${a\\}b} { K {puts hit} default {puts miss} }} m]} ",
    "{ puts \"error:$m\" }\n",
);

/// A **leading-brace** name: the counterexample to "declining a reference is
/// always safe", since the runtime fallback cannot round-trip this name.
const COMPILED_LEADING_BRACE_SCRIPT: &str = concat!(
    "set {{}} V\n",
    "if {[catch {puts ${{}}} m]} { puts \"error:$m\" }\n",
);

#[test]
fn compiled_backslash_braced_var_names_follow_the_emulated_release() {
    for (label, script, err8, ok9) in [
        (
            "array key, escaped close",
            COMPILED_KEY_ESCAPED_CLOSE_SCRIPT,
            "error:can't read \"a\\\": no such variable",
            "V",
        ),
        (
            "array key store, escaped close",
            COMPILED_KEY_ESCAPED_WRITE_SCRIPT,
            "error:can't read \"a\\\": no such variable",
            "K",
        ),
        (
            "switch subject, escaped close",
            COMPILED_SWITCH_ESCAPED_SCRIPT,
            "error:can't read \"a\\\": no such variable",
            "hit",
        ),
        (
            "leading brace name",
            COMPILED_LEADING_BRACE_SCRIPT,
            "error:can't read \"{\": no such variable",
            "V",
        ),
    ] {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            assert_eq!(
                vm_output(script, version),
                err8,
                "{label} at {version:?} must use the 8.x first-close rule"
            );
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                vm_output(script, version),
                ok9,
                "{label} at {version:?} must use the 9.x nesting rule"
            );
        }
    }
}

/// The escaped-**open**-brace key resolves identically at every release, so it
/// gets its own assertion rather than an 8-vs-9 split.
#[test]
fn compiled_escaped_open_brace_key_resolves_at_every_release() {
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            vm_output(COMPILED_KEY_ESCAPED_OPEN_SCRIPT, version),
            "V",
            "{version:?}: `\\{{` is a name character under both close rules"
        );
    }
}

/// The complement of the `${a\}b}` switch vector: a subject that is one whole
/// reference under the **8.x** rule only.
///
/// `${a\}` closes at the first `}` under `FirstClose`, naming the variable
/// `a\`. Under `Tcl9Nesting` the `\}` is an inert pair, so the name never
/// closes and the reference is unterminated.
///
/// Real tclsh: `hit` at 8.6, and at 9.0 a *parse* error (`missing close-brace
/// for variable name`) that rejects the whole script. Both halves matter — the
/// switch-subject gate has to keep the 8.x answer without disturbing the 9.x
/// rejection.
/// The name is built with `format` rather than written as `set "a\\" K`,
/// which would be the obvious spelling. Compiled `set` currently stores a
/// quoted name containing a backslash under its *unsubstituted source
/// spelling* (`a\\`, 3 bytes) instead of the substituted name (`a\`, 2 bytes),
/// so the obvious spelling would fail this test for a reason that has nothing
/// to do with the switch gate. That is a separate defect, filed apart from
/// #1568; `format` sidesteps it and leaves this vector testing only the gate.
const SWITCH_SUBJECT_EIGHT_ONLY_SCRIPT: &str = concat!(
    "set n [format a%c 92]\n",
    "set $n K\n",
    "switch -- ${a\\} { K {puts hit} default {puts miss} }\n",
);

#[test]
fn switch_subject_whole_under_the_8x_rule_only_follows_the_emulated_release() {
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        assert_eq!(
            vm_output(SWITCH_SUBJECT_EIGHT_ONLY_SCRIPT, version),
            "hit",
            "{version:?}: `${{a\\}}` is one whole reference under the 8.x rule, \
             so the subject must load the variable `a\\` rather than being \
             re-substituted as text (which would process the backslash)"
        );
    }

    // At 9.x the same spelling never closes, so the script is rejected outright
    // and the gate must not have made it compilable.
    let service = CompilerSvc {
        registry: CommandRegistry::build_default(),
    };
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
            .analyser_profile();
        let err = service
            .compile_for_profile(SWITCH_SUBJECT_EIGHT_ONLY_SCRIPT, profile)
            .expect_err("an unterminated ${ must not compile at 9.x");
        let text = format!("{err:?}");
        assert!(
            text.contains("close-brace"),
            "{version:?}: must be rejected for the close-brace, got {text}"
        );
    }
}

#[test]
fn switch_subject_whole_under_the_8x_rule_only_matches_real_tclsh() {
    let mut ran = 0;
    if let Some(got) = tclsh_output(
        "TCL_LSP_TCLSH86",
        &["tclsh8.6"],
        SWITCH_SUBJECT_EIGHT_ONLY_SCRIPT,
    ) {
        assert_eq!(
            got,
            vm_output(SWITCH_SUBJECT_EIGHT_ONLY_SCRIPT, TclVersion::V8_6)
        );
        ran += 1;
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: tclsh8.6 (or \
             $TCL_LSP_TCLSH86) was not found"
        );
    }
}

/// An unterminated `${` in a compiled word is a **parse** error: C parses every
/// word of a command before evaluating any, so an earlier `[…]` in the same
/// word never runs. Verified against 8.6.16 and 9.0.4, neither of which prints
/// `SIDE`.
const UNTERMINATED_IN_COMPILED_WORD_SCRIPT: &str = concat!(
    "proc side {} { puts SIDE; return x }\n",
    "if {[catch {puts \"[side]pre${abc\"} m]} { puts \"error:$m\" }\n",
);

#[test]
fn unterminated_braced_var_in_a_compiled_word_is_a_parse_error() {
    // The whole script is REJECTED, at every release: an unterminated `${` is
    // a parse error, and C parses every word of a command before evaluating
    // any of them. Nothing runs — real tclsh 8.6.16 and 9.0.4 both report
    // `missing close-brace for variable name` and neither prints `SIDE`.
    //
    // This is why `subst_word`'s `Unterminated` arm cannot be reached from
    // compiled source: the compiler refuses the source first. The arm is kept
    // defensive and consistent with C, not because a vector reaches it.
    let service = CompilerSvc {
        registry: CommandRegistry::build_default(),
    };
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
            .analyser_profile();
        let err = service
            .compile_for_profile(UNTERMINATED_IN_COMPILED_WORD_SCRIPT, profile)
            .expect_err("an unterminated ${ must not compile");
        let text = format!("{err:?}");
        assert!(
            text.contains("close-brace"),
            "{version:?}: must be rejected for the close-brace, got {text}"
        );
    }
}

#[test]
fn compiled_backslash_braced_var_names_match_real_tclsh() {
    let mut ran = 0;
    for script in [
        COMPILED_KEY_ESCAPED_CLOSE_SCRIPT,
        COMPILED_KEY_ESCAPED_WRITE_SCRIPT,
        COMPILED_KEY_ESCAPED_OPEN_SCRIPT,
        COMPILED_SWITCH_ESCAPED_SCRIPT,
        COMPILED_LEADING_BRACE_SCRIPT,
    ] {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}

/// A **composite** array key — one carrying literal text alongside the `${…}` —
/// is what reaches `codegen::helpers::parse_subst_template`'s `${…}` arm.
///
/// This distinction is easy to lose. Before `push_array_key` was routed through
/// the shared decoder, a *whole* `${…}` key reached `parse_subst_template` too,
/// and the whole-key vectors were what pinned that decoder. Fixing the key arm
/// re-routed them to `parse_simple_var_ref` and silently un-pinned it — a
/// mutation that reverts `parse_subst_template` to `find('}')` went from
/// failing two tests to failing none. These vectors restore the coverage on the
/// path that still uses it.
const COMPILED_COMPOSITE_KEY_PREFIX_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "set arr(xWORLD) V\n",
    "if {[catch {puts $arr(x${a{b}c})} m]} { puts \"error:$m\" }\n",
);

const COMPILED_COMPOSITE_KEY_SUFFIX_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "set arr(WORLDy) V\n",
    "if {[catch {puts $arr(${a{b}c}y)} m]} { puts \"error:$m\" }\n",
);

/// Two references in one key, so the decoder must find both closers.
const COMPILED_COMPOSITE_KEY_TWICE_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "set arr(WORLDWORLD) V\n",
    "if {[catch {puts $arr(${a{b}c}${a{b}c})} m]} { puts \"error:$m\" }\n",
);

/// The same composite shape with a backslash in the name.
const COMPILED_COMPOSITE_KEY_ESCAPE_SCRIPT: &str = concat!(
    "set {a\\}b} K\n",
    "set arr(xK) V\n",
    "if {[catch {puts $arr(x${a\\}b})} m]} { puts \"error:$m\" }\n",
);

/// The store direction of a composite key.
const COMPILED_COMPOSITE_KEY_WRITE_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {set arr(x${a{b}c}) V} m]} { puts \"error:$m\" } ",
    "else { puts [array names arr] }\n",
);

// The decoder is not reached only through array keys. These three are the other
// live routes a compiled `${…}` word takes, and each one has to agree with the
// release rule the same way the key does.
/// Through a nested command substitution.
const COMPILED_COMPOSITE_KEY_NESTED_CMD_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {puts [expr [string length ${a{b}c}]]} m]} { puts \"error:$m\" }\n",
);

/// Through a list-building argument.
const COMPILED_COMPOSITE_KEY_LIST_ARG_SCRIPT: &str = concat!(
    "set {a{b}c} WORLD\n",
    "if {[catch {puts [string map [list ${a{b}c} Z] WORLD]} m]} { puts \"error:$m\" }\n",
);

/// Through an unbraced expr word, where expr re-parses the substituted text.
const COMPILED_COMPOSITE_KEY_EXPR_WORD_SCRIPT: &str = concat!(
    "set {a{b}c} 41\n",
    "if {[catch {puts [expr ${a{b}c}+1]} m]} { puts \"error:$m\" }\n",
);

#[test]
fn compiled_composite_array_key_follows_the_emulated_release() {
    for (label, script, err8, ok9) in [
        (
            "prefix",
            COMPILED_COMPOSITE_KEY_PREFIX_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "V",
        ),
        (
            "suffix",
            COMPILED_COMPOSITE_KEY_SUFFIX_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "V",
        ),
        (
            "two references",
            COMPILED_COMPOSITE_KEY_TWICE_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "V",
        ),
        (
            "escaped close",
            COMPILED_COMPOSITE_KEY_ESCAPE_SCRIPT,
            "error:can't read \"a\\\": no such variable",
            "V",
        ),
        (
            "store",
            COMPILED_COMPOSITE_KEY_WRITE_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "xWORLD",
        ),
        (
            "nested command",
            COMPILED_COMPOSITE_KEY_NESTED_CMD_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "5",
        ),
        (
            "list argument",
            COMPILED_COMPOSITE_KEY_LIST_ARG_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "Z",
        ),
        (
            "expr word",
            COMPILED_COMPOSITE_KEY_EXPR_WORD_SCRIPT,
            "error:can't read \"a{b\": no such variable",
            "42",
        ),
    ] {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            assert_eq!(
                vm_output(script, version),
                err8,
                "composite key ({label}) at {version:?} must use the 8.x rule"
            );
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                vm_output(script, version),
                ok9,
                "composite key ({label}) at {version:?} must use the 9.x rule"
            );
        }
    }
}

// Issue #1617 — `$={name}` is the user's literal text, not a compiler marker.
//
// The codegen used to decode a whole word spelt `$={name}` as an internal
// "braced scalar" marker and compile it to `push "name"; loadStk`. Nothing in
// this workspace has ever *produced* that spelling — the segmenter re-spells a
// braced variable word verbatim from source — so the only words that ever
// reached the decoder were the user's own, where `$=` is not a substitution
// trigger in any release (`=` is not a name character; `Tcl_ParseVarName`,
// tmp/tcl9.0.4/generic/tclParse.c). Every reach was wrong code, and because
// producer and consumer were both release-blind in compatible ways no program
// could tell the two halves apart — which is why mutation M3b (flip the marker
// arm's close scan) survived the whole corpus.
//
// These vectors pin the literal reading on each route the marker decoder sat
// on: a command word, a `set` value, the `switch` subject (`cfg_lower`), a
// list argument, an array key, and a proc body (the LVT path).

/// A bare command word.
const DOLLAR_EQ_WORD_SCRIPT: &str = concat!("set y hi\n", "puts $={y}\n");

/// The value half of a `set` — `emit_value` / `emit_value_interpolated`.
const DOLLAR_EQ_SET_VALUE_SCRIPT: &str = concat!("set y hi\n", "set z $={y}\n", "puts $z\n");

/// The `switch` subject, which `cfg_builder::cfg_lower` used to promote to a
/// `Raw` (variable-reference) operand when it parsed as the marker.
const DOLLAR_EQ_SWITCH_SUBJECT_SCRIPT: &str = concat!(
    "set y hi\n",
    "switch -- $={y} {\n",
    "  hi { puts matched-var }\n",
    "  default { puts literal }\n",
    "}\n",
);

/// A list argument, so the literal is observable through list quoting.
const DOLLAR_EQ_LIST_ARG_SCRIPT: &str = concat!("set y hi\n", "puts [list $={y}]\n");

/// An array key — the `push_array_key` route.
const DOLLAR_EQ_ARRAY_KEY_SCRIPT: &str = concat!(
    "set y hi\n",
    "set arr($={y}) V\n",
    "puts [array names arr]\n",
);

/// Inside a proc, where a decoded name would have become an LVT slot load.
const DOLLAR_EQ_IN_PROC_SCRIPT: &str = concat!("proc p {} { set y inner; puts $={y} }\n", "p\n");

// The **mixed** shape: a literal `$` and a real substitution in the *same*
// word. `Tcl_ParseVarName` reads a `$` that starts no reference as the text
// `$` and keeps parsing (form 3, `justADollarSign`,
// tmp/tcl9.0.4/generic/tclParse.c:1454 and :1502) — so the substitution after
// it still happens.
//
// The lone-`$=` vectors above cannot see this: a top-level word reaches codegen
// through the segmenter, which re-spells a bare `$x` as `${x}`, and the runtime
// `subst_word` fallback resolves that even from a word pushed raw. Inside an
// inline command substitution the argument text is the **raw source**, bare
// `$x` and all, so a decoder that gives up on the whole word loses the
// substitution outright: `[list $={y}$x]` yielded `{$={y}$x}` where both
// oracles give `{$={y}X}` (#1668 review).

/// The reported repro — a literal `$=` marker-shaped run then a real `$x`,
/// inside an inline `[list …]`.
const DOLLAR_EQ_MIXED_LIST_ARG_SCRIPT: &str =
    concat!("set x X\n", "set y Y\n", "puts [list $={y}$x]\n");

/// The same word measured, so the *string* is pinned and not just its list
/// rendering.
const DOLLAR_EQ_MIXED_LENGTH_SCRIPT: &str =
    concat!("set x X\n", "set y Y\n", "puts [string length $={y}$x]\n");

/// The bare command-word route.
const DOLLAR_EQ_MIXED_WORD_SCRIPT: &str = concat!("set x X\n", "set y Y\n", "puts $={y}$x\n");

/// The `set` value route.
const DOLLAR_EQ_MIXED_SET_VALUE_SCRIPT: &str =
    concat!("set x X\n", "set y Y\n", "set z $={y}$x\n", "puts $z\n",);

/// The `switch` subject route: the subject is the *substituted* word, so it
/// matches neither arm and falls to `default`.
const DOLLAR_EQ_MIXED_SWITCH_SCRIPT: &str = concat!(
    "set x X\n",
    "set y Y\n",
    "switch -- $={y}$x {\n",
    "  X { puts matched-var }\n",
    "  default { puts literal }\n",
    "}\n",
);

/// The array-key route.
const DOLLAR_EQ_MIXED_ARRAY_KEY_SCRIPT: &str = concat!(
    "set x X\n",
    "set y Y\n",
    "set arr($={y}$x) V\n",
    "puts [array names arr]\n",
);

/// A proc body, where the substituted half is an LVT slot.
const DOLLAR_EQ_MIXED_IN_PROC_SCRIPT: &str =
    concat!("proc p {} { set x X; puts [list $=$x] }\n", "p\n");

/// `$(idx)` is the **empty-named** array, not a bare `$` — C admits it
/// explicitly ("Support for empty array names here", `tclParse.c:1449-1453`),
/// so the literal-`$` rule must not swallow it. This composite was un-decodable
/// before the same fix.
const DOLLAR_EMPTY_ARRAY_MIXED_SCRIPT: &str =
    concat!("set (k) EMPTY\n", "set x X\n", "puts [list a$(k)b$x]\n",);

/// A `$` that is a whole word of its own, beside a real reference.
const DOLLAR_ALONE_BESIDE_VAR_SCRIPT: &str = concat!("set x X\n", "puts [list $ $x]\n");

const DOLLAR_EQ_VECTORS: &[(&str, &str, &str)] = &[
    ("command word", DOLLAR_EQ_WORD_SCRIPT, "$={y}"),
    ("set value", DOLLAR_EQ_SET_VALUE_SCRIPT, "$={y}"),
    ("switch subject", DOLLAR_EQ_SWITCH_SUBJECT_SCRIPT, "literal"),
    ("list argument", DOLLAR_EQ_LIST_ARG_SCRIPT, "{$={y}}"),
    ("array key", DOLLAR_EQ_ARRAY_KEY_SCRIPT, "{$={y}}"),
    ("proc body", DOLLAR_EQ_IN_PROC_SCRIPT, "$={y}"),
    // Mixed: literal `$` run, then a real substitution in the same word.
    (
        "mixed list argument",
        DOLLAR_EQ_MIXED_LIST_ARG_SCRIPT,
        "{$={y}X}",
    ),
    ("mixed string length", DOLLAR_EQ_MIXED_LENGTH_SCRIPT, "6"),
    ("mixed command word", DOLLAR_EQ_MIXED_WORD_SCRIPT, "$={y}X"),
    (
        "mixed set value",
        DOLLAR_EQ_MIXED_SET_VALUE_SCRIPT,
        "$={y}X",
    ),
    (
        "mixed switch subject",
        DOLLAR_EQ_MIXED_SWITCH_SCRIPT,
        "literal",
    ),
    (
        "mixed array key",
        DOLLAR_EQ_MIXED_ARRAY_KEY_SCRIPT,
        "{$={y}X}",
    ),
    ("mixed proc body", DOLLAR_EQ_MIXED_IN_PROC_SCRIPT, "{$=X}"),
    (
        "empty-name array beside a var",
        DOLLAR_EMPTY_ARRAY_MIXED_SCRIPT,
        "aEMPTYbX",
    ),
    (
        "lone dollar beside a var",
        DOLLAR_ALONE_BESIDE_VAR_SCRIPT,
        "{$} X",
    ),
];

/// The reading is the same at every release: `$=` never starts a substitution,
/// so there is no release axis here to get wrong.
#[test]
fn compiled_dollar_equals_word_is_literal_at_every_release() {
    for (label, script, want) in DOLLAR_EQ_VECTORS {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                vm_output(script, version),
                *want,
                "`$={{y}}` ({label}) at {version:?} must stay literal, not load a variable"
            );
        }
    }
}

#[test]
fn compiled_dollar_equals_word_matches_real_tclsh() {
    let mut ran = 0;
    for (_, script, _) in DOLLAR_EQ_VECTORS {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}

#[test]
fn compiled_composite_array_key_matches_real_tclsh() {
    let mut ran = 0;
    for script in [
        COMPILED_COMPOSITE_KEY_PREFIX_SCRIPT,
        COMPILED_COMPOSITE_KEY_SUFFIX_SCRIPT,
        COMPILED_COMPOSITE_KEY_TWICE_SCRIPT,
        COMPILED_COMPOSITE_KEY_ESCAPE_SCRIPT,
        COMPILED_COMPOSITE_KEY_WRITE_SCRIPT,
        COMPILED_COMPOSITE_KEY_NESTED_CMD_SCRIPT,
        COMPILED_COMPOSITE_KEY_LIST_ARG_SCRIPT,
        COMPILED_COMPOSITE_KEY_EXPR_WORD_SCRIPT,
    ] {
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH86", &["tclsh8.6"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V8_6));
            ran += 1;
        }
        if let Some(got) = tclsh_output("TCL_LSP_TCLSH90", &["tclsh9.0"], script) {
            assert_eq!(got, vm_output(script, TclVersion::V9_0));
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}
