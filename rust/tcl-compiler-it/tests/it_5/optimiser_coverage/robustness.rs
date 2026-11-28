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

//! Edge cases: malformed input, loops and namespaces, variable shapes, pass interactions and the profile directive under multipass.

use super::*;

// apply_optimisations / find_optimisations — edge cases

#[test]
fn apply_optimisations_edge_cases() {
    crate::install_surfaces();
    let registry = static_context_for(TCL).commands();
    // Empty optimisation set ⇒ source unchanged.
    assert_eq!(apply_optimisations("set x 1", &[]), "set x 1");

    // Multiple non-overlapping rewrites. tclsh: a=1, b=2 ⇒ a+b == 3.
    assert!(optimised("set a 1\nset b 2\nset c [expr {$a + $b}]", TCL).contains("set c 3"));

    // Reverse-offset application stays stable. tclsh: a=1, b=2 ⇒ `puts 1`, `puts 2`.
    let rev = optimised("set a 1\nset b 2\nputs $a\nputs $b", TCL);
    assert!(rev.contains("puts 1"));
    assert!(rev.contains("puts 2"));

    // Sorting: the optimise_with_dialect set is span-sorted (the manager's
    // determinism chokepoint). Co-located entries (e.g. O100 + O102 on the same
    // expr span) are permitted before application; `apply_optimisations` resolves
    // them, which the value assertions above already exercise.
    let src = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$a + 3}]";
    let opts = optimise_with_dialect(
        src,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
    );
    let starts: Vec<u32> = opts.iter().map(|o| o.span.start()).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(
        starts, sorted,
        "optimisations must be span-start-sorted: {starts:?}"
    );
}

// Variable-shape guardrails — array/namespaced refs not rewritten

#[test]
fn variable_shape_guardrails() {
    crate::install_surfaces();
    for src in [
        "set x ${a(1)}\nputs $x",
        "set x $a(1)\nputs $x",
        "set x $::ns::arr(k)\nputs $x",
    ] {
        assert_eq!(optimised(src, TCL), src);
        assert!(
            opt_codes(src, TCL).is_empty(),
            "no rewrite for shape: {src}"
        );
    }
}

// Edge cases and robustness — must not crash, well-formed results

#[test]
fn edge_cases_no_crash() {
    crate::install_surfaces();
    // Empty / comment / whitespace sources ⇒ no optimisations.
    assert_eq!(opt_codes("", TCL), [] as [std::string::String; 0]);
    assert_eq!(
        opt_codes("# just a comment", TCL),
        [] as [std::string::String; 0]
    );
    assert_eq!(opt_codes("   \n\n  ", TCL), [] as [std::string::String; 0]);
    assert_eq!(optimised("", TCL), "");

    // Nested proc definitions ⇒ no crash.
    let _ = opt_codes(
        "proc outer {} {\n    proc inner {} { return 1 }\n    return [inner]\n}\n",
        TCL,
    );

    // Deep expression nesting ⇒ no stack overflow.
    let mut expr = String::from("$x");
    for _ in 0..20 {
        expr = format!("({expr} + 1)");
    }
    let _ = opt_codes(&format!("set v [expr {{{expr}}}]"), TCL);

    // Many sets ⇒ no performance pathology (just completes).
    let mut lines: Vec<String> = (0..50).map(|i| format!("set v{i} {i}")).collect();
    lines.push("puts $v49".into());
    let _ = opt_codes(&lines.join("\n"), TCL);

    // unset clears the constant ⇒ no fold across it (source unchanged).
    let unset_src = "set a 1\nunset a\nset b [expr {$a + 2}]";
    assert_eq!(optimised(unset_src, TCL), unset_src);

    // catch / try / foreach bodies ⇒ no crash.
    let _ = opt_codes("catch {\n    set x [expr {1 + 2}]\n}\n", TCL);
    let _ = opt_codes(
        "try {\n    set x [expr {1 + 2}]\n} on error {e} {\n    puts $e\n}\n",
        TCL,
    );
    let _ = opt_codes("foreach item {a b c} {\n    puts $item\n}\n", TCL);
    let _ = opt_codes("switch abc {\n    abc -\n    def { set x 1 }\n}\n", TCL);
}

#[test]
fn edge_cases_loops_and_namespaces() {
    crate::install_surfaces();
    // Constants before a loop that may modify them are NOT propagated in.
    let loop_src =
        "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\n";
    assert_eq!(optimised(loop_src, TCL), loop_src);
    assert_eq!(opt_codes(loop_src, TCL), [] as [std::string::String; 0]);

    // Deeply nested namespace proc folds at the call site. tclsh: a::b::c::f == 1.
    let deep = "namespace eval a {\n    namespace eval b {\n        namespace eval c {\n            proc f {} { return 1 }\n        }\n    }\n}\nset v [a::b::c::f]\n";
    assert!(optimised(deep, TCL).contains("set v 1"));
}

#[test]
fn robustness_malformed_inputs() {
    crate::install_surfaces();
    // Malformed / unusual inputs ⇒ no panic, well-formed (possibly empty) result.
    for src in [
        "set v [expr {",
        "if {$x",
        "puts",
        "set x 1",
        "set x \"héllo\"\nputs $x",
        "set x \"hello\\nworld\"\nputs $x",
        "set x {$not_a_var}\nputs $x",
        "set a 1; set b [expr {$a + 1}]",
    ] {
        let _ = opt_codes(src, TCL); // no panic; result is a Vec
    }
    // `{$not_a_var}` is a brace literal, not a variable ref — propagation is
    // suppressed (no fresh substitution injected). Output is sound either way.
    let bracelit = optimised("set x {$not_a_var}\nputs $x", TCL);
    assert!(bracelit.contains("$not_a_var")); // tclsh: prints literal "$not_a_var"
}

// Pass interactions — composed rewrites

#[test]
fn pass_interactions() {
    crate::install_surfaces();
    // Constant fold resolves the condition, then O112/O101 eliminates the if.
    assert!(opt_any_of(
        "set x 1\nif {$x > 0} {\n    puts yes\n}\n",
        TCL,
        &["O101", "O112"]
    ));
    // Propagation enables DSE: after folding x into puts, `set x` is dead.
    let prop = "set x 42\nputs $x";
    assert!(optimised(prop, TCL).contains("puts 42"));
    assert!(opt_fires(prop, TCL, "O109"));

    // Chained propagation across multiple variables. tclsh: a=1 ⇒ d==4.
    assert!(optimised(
        "set a 1\nset b [expr {$a + 1}]\nset c [expr {$b + 1}]\nset d [expr {$c + 1}]\nputs $d\n",
        TCL
    )
    .contains("puts 4"));

    // All uses propagated then DSE. tclsh: x=5 ⇒ `puts 5` and y==6.
    let dse = "set x 5\nputs $x\nset y [expr {$x + 1}]";
    assert!(optimised(dse, TCL).contains("puts 5"));
    assert!(optimised(dse, TCL).contains("set y 6"));

    // Overlapping nested-expr rewrites resolve without corruption (no panic).
    let _ = opt_codes("set a 1\nset b [expr {$a + [expr {2 + 3}]}]", TCL);

    // InstCombine in a branch then fold. tclsh: a=5 ⇒ 5+0==5 ⇒ condition true.
    assert!(opt_any_of(
        "set a 5\nif {$a + 0 == 5} { puts yes }",
        TCL,
        &["O101", "O112"]
    ));

    // Strength reduction inside a command substitution expr.
    assert!(opt_fires(&int_xy("if {$x ** 2} { puts yes }"), TCL, "O113"));
    // O117 fires in branch conditions.
    assert!(opt_fires(
        &in_proc("if {[string length $s] == 0} { puts empty }"),
        TCL,
        "O117"
    ));
    // String-compare ⇒ eq rewrite inside [expr {...}]. Rust routes the expr-subst
    // form through O110 (instcombine) rather than O120; the rewrite is identical.
    // tclsh: $a == "hello" <=> $a eq "hello".
    let expr_eq = &in_proc("set ok [expr {$a == \"hello\"}]");
    assert!(optimised(expr_eq, TCL).contains("$a eq \"hello\""));
    assert!(opt_any_of(expr_eq, TCL, &["O120", "O110"]));
}

// Profile directive survival + multipass string-build collapse

#[test]
fn profile_directive_and_multipass() {
    crate::install_surfaces();
    // The `# profiles: HTTP2` comment survives optimisation; the `if {1}` inside
    // the event unwraps (O112) and keeps the HTTP2 call.
    let src =
        "# profiles: HTTP2\nwhen HTTP_REQUEST {\n    if {1} {\n        HTTP2::active\n    }\n}\n";
    let out = optimised(src, IR);
    assert!(out.contains("# profiles: HTTP2"));
    assert!(out.contains("HTTP2::active"));
    assert!(opt_fires(src, IR, "O112"));

    // Multipass: a write-only string build in a proc collapses to a single
    // `return {Hello World}`; the intermediate local is removed. tclsh: the proc
    // returns "Hello World" either way.
    let registry = static_context_for(TCL).commands();
    let build = "proc build_banner {} {\n    set msg {Hello}\n    append msg { }\n    append msg World\n    return $msg\n}\n";
    let (mp, _) = optimise_source_multipass(
        build,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert!(mp.contains("return {Hello World}"));
    assert!(!mp.contains("append"));
    assert!(!mp.contains("set msg"));
}
