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
    let registry = tcl_registry::registry_for_profile(profile);
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error_with_config(src, config) {
        return Err(CompileError(msg));
    }
    let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
        src,
        registry,
        config,
        profile.name,
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
    let profile = DialectProfile::by_name(version.dialect_name());
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

#[test]
fn braced_var_close_rule_matches_real_tclsh() {
    let mut ran = 0;
    for script in [BRACED_VAR_SUBST_SCRIPT, BRACED_VAR_ESCAPE_SCRIPT] {
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
