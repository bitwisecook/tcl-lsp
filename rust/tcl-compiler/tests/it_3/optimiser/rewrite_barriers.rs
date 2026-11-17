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

//! What blocks a rewrite: dynamic names, traces, barriers, shared variables, the method guard and closed native consumers.

use super::*;

// The dynamic-name value-motion barrier and the whole-module variable-trace
// fact. Every source below would miscompile (or be mis-reported) without
// the shared `FunctionUnit::dynamic_barrier_blocks_value_motion` gate and
// the `Module::traced_variables` widening; each pin asserts the optimiser
// abstains. tclsh oracles cited per test.

// tclsh 8.6/9.0: `f acc` returns `zzz b` — folding the chain to
// `set acc {a b}` (O130) walks straight past the dynamic write.
#[test]
fn dynamic_name_write_blocks_o130_chain_fold_issue_1374() {
    let src =
        "proc f {name} { set acc {}; lappend acc a; set $name zzz; lappend acc b; return $acc }";
    assert!(!opt_fires(src, TCL, "O130"));
    assert!(!opt_fires(src, TCL, "O104"));
    assert_eq!(optimised(src, TCL), src);
}

// tclsh 8.6/9.0: `f x` prints `2` — forwarding `puts $x` to `puts 1`
// (O102, or O100 off the SCCP lattice) ignores that `set $name 2` can
// rewrite `x`.
#[test]
fn dynamic_name_write_blocks_o102_forwarding_issue_1374() {
    let src = "proc f {name} { set x 1; set $name 2; puts $x }";
    assert!(!opt_fires(src, TCL, "O102"));
    assert!(!opt_fires(src, TCL, "O100"));
    assert!(!opt_fires(src, TCL, "O109"));
    assert_eq!(optimised(src, TCL), src);
}

// tclsh 8.6/9.0: `f flag 1` prints `zzz` — sinking `set flag hello` past
// the dynamic write into the branch (O125) makes it print `hello`.
#[test]
fn dynamic_name_write_blocks_o125_sinking_issue_1374() {
    let src = "proc f {name c} { set flag hello; if {$c} { set $name zzz; puts $flag } }";
    assert!(!opt_fires(src, TCL, "O125"));
    assert_eq!(optimised(src, TCL), src);
}

// O119 moves and deletes `set` statements, so the same barrier applies.
#[test]
fn dynamic_name_write_blocks_o119_packing_issue_1374() {
    let src = "set $name q\nset a 1\nset b 2\nset c 3\n";
    assert!(!opt_fires(src, TCL, "O119"));
    // Control: the clean top-level sibling still packs under tcl8.6 (top
    // level, because inside a proc the unused-variable O126 deletions win
    // overlap selection over the pack either way).
    let clean = "set a 1\nset b 2\nset c 3\n";
    assert!(opt_fires(clean, TCL, "O119"));
}

// tclsh 8.6/9.0: each `set` fires the `::onw` write trace (prints `trace`
// twice) — deleting `set g 1` as a dead store (O109) drops a callback.
// The trace spells `::g`, the stores are unqualified; both name the same
// top-level global.
#[test]
fn traced_global_store_is_not_a_dead_store_issue_1377() {
    let src = "proc onw {a b c} { puts trace }\ntrace add variable ::g write ::onw\nset g 1\nset g 2\nputs $g\n";
    assert!(!opt_fires(src, TCL, "O109"));
    assert_eq!(optimised(src, TCL), src);
}

// tclsh 8.6/9.0: the `lappend`s fire the `::acc` write trace — folding to
// `set acc {a b}` (O130) collapses three observable writes into one.
#[test]
fn traced_list_chain_is_not_folded_issue_1377() {
    let src = "proc onw {a b c} { puts trace }\ntrace add variable ::acc write ::onw\nset acc {}\nlappend acc a\nlappend acc b\n";
    assert!(!opt_fires(src, TCL, "O130"));
    assert_eq!(optimised(src, TCL), src);
}

// A computed command head (`$cmd $x`) lowers to a `Statement::Barrier`
// whose retained words reference `$x` and raises no dynamic-name flag;
// treating a barrier as referencing no variable would let O125 sink
// `set x 1` into the branch, leaving `f 0 puts` to read an unset `x` (tclsh:
// `can't read "x"`). A dynamic `eval` spelling of the trailing reference
// must abstain too (that one via the opaque-script barrier).
#[test]
fn barrier_reference_blocks_o125_issue_1402() {
    let src = "proc f {flag cmd} {\nset x 1\nif {$flag} { puts $x }\n$cmd $x\n}";
    assert!(!opt_fires(src, TCL, "O125"));
    let eval_src = "proc f {flag} {\nset x 1\nif {$flag} { puts $x }\neval [list puts $x]\n}";
    assert!(!opt_fires(eval_src, TCL, "O125"));
}

// O107 inside a command-resolution-guarded `TclOO` method body.
//
// A method runs in the receiver's namespace, which is chosen at run time and
// can shadow any relative command name, so one unqualified head anywhere in
// the body excludes the method from deep analysis
// (`ir_helpers::requires_runtime_command_namespace`). Such a unit carries an
// empty SCCP executable-block set — no facts, rather than the fact that every
// block is dead — and O107 is the report that would otherwise read that
// absence as proof and delete the whole body.
//
// Structural rather than Tcl-observable: the assertion is that the optimiser
// leaves the source alone, which is trivially semantics-preserving. Deleting
// the body is what is not — `[Greeter new] whoami` answers `::Greeter`
// intact and the empty string once emptied.

/// A one-method class whose method body is `body`.
fn method_body(body: &str) -> String {
    format!("oo::class create Greeter {{\n    method whoami {{}} {{ {body} }}\n}}\n")
}

#[test]
fn tcloo_method_body_survives_the_command_resolution_guard() {
    // Every `self` spelling that carries a value: the defining class, a bare
    // `[self]` (equivalent to `self object`), and the instance namespace.
    for body in [
        "return [self class]",
        "return [self]",
        "return [self namespace]",
    ] {
        let src = method_body(body);
        assert!(
            !opt_fires(&src, TCL, "O107"),
            "{body}: a live method body must not be reported unreachable"
        );
        assert_eq!(
            optimised(&src, TCL),
            src,
            "{body}: the method body must survive the optimiser intact"
        );
    }
}

#[test]
fn a_guarded_method_body_keeps_every_relative_head() {
    // The guard is about command resolution, not `TclOO` introspection: an
    // ordinary builtin, a `my` dispatch, and a `set` whose value is a nested
    // call each name a command relatively, so each method is guarded.
    for body in ["puts hello", "my helper", "set x [string length abc]"] {
        let src = method_body(body);
        assert!(
            !opt_fires(&src, TCL, "O107"),
            "{body}: a guarded body must not be reported unreachable"
        );
        assert_eq!(optimised(&src, TCL), src, "{body}: body must survive");
    }
}

#[test]
fn o107_still_fires_on_genuinely_unreachable_method_code() {
    // Control: O107 is suppressed only where the lattice is absent. Every head
    // here is `::`-qualified, so the method gets deep analysis and the `if {0}`
    // body is provably dead. tclsh: `[Greeter new] whoami` prints `live` with
    // or without the rewrite.
    let src = "oo::class create Greeter {\n    method whoami {} {\n        ::if {0} { ::puts never }\n        ::puts live\n    }\n}\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts never"),
        "dead branch must still be eliminated: {out}"
    );
    assert!(out.contains("::puts live"), "live code must survive: {out}");
}

/// The ordinary optimiser folds a source literal into an expression operand
/// under its own licence, not the closed native consumer's: the numeric
/// reading of a source literal follows its string on every release
/// (`expr_operand_literal_reading` in `optimiser_coverage`). tclsh 8.4.20,
/// 8.6.18, 9.0.4 and 9.1.0 print `2` and `1` for the source and for
/// `set x 1; puts 2; puts 1`.
#[test]
fn the_ordinary_optimiser_folds_a_source_literal_operand() {
    let src = "set x 1\nputs [expr {$x + 1}]\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(out.contains("puts 2") && !out.contains("[expr"), "{out}");
}

#[test]
fn closed_native_consumer_refuses_callbacks_rebinding_and_value_escape() {
    for source in [
        "set x 1\nputs [expr {$x + [incr x]}]\n",
        "set x 1\nunknown_call\nputs $x\n",
        "set x 1\nrename puts saved_puts\nputs $x\n",
        "set x 1\ntrace add variable x read observe\nputs $x\n",
        "set x 1\nchan push stdout observe\nputs $x\n",
        "set x 1\nreturn $x\n",
        "set tcl_version 1\nputs $tcl_version\n",
    ] {
        assert!(
            tcl_engine_tclvm::execute_optimised_closed_source(
                source.to_owned(),
                tcl_dialect::TclVersion::V8_6,
            )
            .is_err(),
            "{source}"
        );
    }
}

/// A `sharedvar` names the connection's shared cell, which another event
/// reads: its writes are never unused or dead stores (#2413).
#[test]
fn sharedvar_writes_are_kept() {
    const IR: &str = "f5-irules";
    for src in [
        "when CLIENT_ACCEPTED {\n    sharedvar mode\n    set mode 1\n}\n\
         when SERVER_CONNECTED {\n    sharedvar mode\n    if {$mode == 1} { log local0. \"one\" }\n    set mode 2\n}\n",
        "when SERVER_CONNECTED {\n    sharedvar mode\n    if {$mode == 1} { log local0. \"one\" }\n    set mode 2\n}\n\
         when CLIENT_ACCEPTED {\n    sharedvar mode\n    set mode 1\n}\n",
    ] {
        for code in ["O126", "O109"] {
            assert!(!opt_fires(src, IR, code), "{code} must not fire: {src}");
        }
        assert!(optimised(src, IR).contains("set mode 1"), "{src}");
        assert!(optimised(src, IR).contains("set mode 2"), "{src}");
        let codes = analyser_codes(src, IR);
        for code in ["W211", "W220"] {
            assert!(!codes.iter().any(|c| c == code), "{code}: {codes:?}");
        }
    }
}
