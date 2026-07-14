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

//! Unit tests for the proc inliner (v0 / verbatim / v3).
//!
//! These are IR-shape tests — the execution-differential standard the
//! repo applies to the inliner is gated on the WASM codegen consumer
//! (not yet implemented), so these verify the spliced IR's structure
//! meanwhile.

use super::*;
use crate::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn module_for(source: &str) -> Module {
    CompilationUnit::build_for(source, &CommandRegistry::build_default(), false).ir_module
}

/// `inline_module` against a freshly-built default registry.
fn inline_module_default(module: Module) -> Module {
    inline_module(module, &CommandRegistry::build_default())
}

/// Count statement-position calls to `command` across the top level.
fn top_calls_to(module: &Module, command: &str) -> usize {
    module
        .top_level
        .statements
        .iter()
        .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
        .count()
}

/// All top-level statements after inlining.
fn inlined_top(source: &str) -> Vec<Statement> {
    inline_module(module_for(source), &CommandRegistry::build_default())
        .top_level
        .statements
}

/// Whether any top-level statement is an assignment to a mangled
/// `__inline_*` slot.
fn has_inline_binding(stmts: &[Statement]) -> bool {
    stmts.iter().any(|s| match s {
        Statement::AssignValue { name, .. } | Statement::AssignConst { name, .. } => {
            name.starts_with("__inline_")
        }
        _ => false,
    })
}

// v0 / verbatim tests

#[test]
fn empty_body_call_vanishes() {
    let module = module_for("proc ::noop {} {}\nnoop\nputs done");
    assert_eq!(top_calls_to(&module, "noop"), 1);
    let inlined = inline_module(module, &CommandRegistry::build_default());
    assert_eq!(
        top_calls_to(&inlined, "noop"),
        0,
        "empty-body call should vanish"
    );
    assert_eq!(top_calls_to(&inlined, "puts"), 1, "unrelated puts survives");
}

#[test]
fn verbatim_wrapper_body_is_spliced() {
    let module = module_for("proc ::greet {} { puts hello }\ngreet");
    let inlined = inline_module(module, &CommandRegistry::build_default());
    assert_eq!(top_calls_to(&inlined, "greet"), 0, "wrapper call replaced");
    assert_eq!(top_calls_to(&inlined, "puts"), 1, "wrapper body spliced");
}

#[test]
fn multi_statement_verbatim_wrapper() {
    let module = module_for("proc ::two {} { puts a\n puts b }\ntwo");
    let inlined = inline_module(module, &CommandRegistry::build_default());
    assert_eq!(top_calls_to(&inlined, "two"), 0);
    assert_eq!(top_calls_to(&inlined, "puts"), 2, "both body stmts spliced");
}

#[test]
fn redefined_proc_is_not_inlined() {
    let module = module_for("proc ::r {} { puts a }\nproc ::r {} { puts b }\nr");
    assert!(module.redefined_procedures.contains("::r"));
    let inlined = inline_module(module, &CommandRegistry::build_default());
    assert_eq!(top_calls_to(&inlined, "r"), 1, "redefined proc kept");
}

#[test]
fn arg_command_subst_blocks_inline() {
    // `puts [clock seconds]` depends on frame command resolution — neither
    // verbatim nor v3 may splice it.
    let module = module_for("proc ::w {} { puts [clock seconds] }\nw");
    let inlined = inline_module(module, &CommandRegistry::build_default());
    assert_eq!(top_calls_to(&inlined, "w"), 1);
}

#[test]
fn inline_inside_control_flow_body() {
    let module = module_for("proc ::greet {} { puts hi }\nif {1} { greet }");
    let inlined = inline_module(module, &CommandRegistry::build_default());
    let if_has_puts = inlined.top_level.statements.iter().any(|s| {
        matches!(s, Statement::If { clauses, .. }
            if clauses.iter().any(|c| c.body.statements.iter().any(|b|
                matches!(b, Statement::Call { command, .. } if command == "puts"))))
    });
    assert!(if_has_puts, "call inside if-body should be inlined");
}

#[test]
fn count_statements_walks_nested_bodies() {
    let module = module_for("proc ::f {} { puts a\n if {1} { puts b\n puts c } }");
    let proc = &module.procedures["::f"];
    assert_eq!(count_statements(&proc.body), 4);
}

// v3 parameterised

#[test]
fn v3_binds_param_and_renames_body() {
    let stmts = inlined_top("proc ::id {x} { puts $x }\nid hello");
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "id")),
        "param call replaced"
    );
    assert!(
        has_inline_binding(&stmts),
        "a mangled param binding is emitted"
    );
    // The body `puts $x` is spliced with the renamed variable.
    let puts_arg = stmts.iter().find_map(|s| match s {
        Statement::Call { command, args, .. } if command == "puts" => args.first().cloned(),
        _ => None,
    });
    let puts_arg = puts_arg.expect("spliced puts present");
    assert!(
        puts_arg == "$__inline_1__x" || puts_arg == "${__inline_1__x}",
        "body arg renamed to the mangled slot, got {puts_arg:?}"
    );
}

#[test]
fn v3_local_write_is_renamed() {
    // Zero-param wrapper with a local write — not verbatim-eligible
    // (`set` isn't a splice-safe call), handled by v3 instead.
    let stmts = inlined_top("proc ::w {} { set x 1 }\nw");
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "w")),
        "wrapper call replaced by v3"
    );
    let renamed = stmts.iter().any(|s| {
        matches!(s, Statement::AssignConst { name, .. } | Statement::AssignValue { name, .. }
            if name.starts_with("__inline_") && name.ends_with("__x"))
    });
    assert!(renamed, "local write renamed to a mangled slot");
}

#[test]
fn v3_braced_literal_arg_binds_as_const() {
    // `id {$y}` must bind the *literal* string `$y`, not re-substitute it
    // from the caller frame — so the binding is an AssignConst.
    let stmts = inlined_top("proc ::id {x} { puts $x }\nid {$y}");
    let const_binding = stmts.iter().any(|s| {
        matches!(s, Statement::AssignConst { name, value, .. }
            if name.starts_with("__inline_") && value == "$y")
    });
    assert!(const_binding, "braced-literal arg bound via AssignConst");
}

#[test]
fn v3_default_fills_missing_arg() {
    let stmts = inlined_top("proc ::f {{x 5}} { puts $x }\nf");
    let bound_default = stmts.iter().any(|s| {
        matches!(s, Statement::AssignValue { name, value, .. } | Statement::AssignConst { name, value, .. }
            if name.starts_with("__inline_") && value == "5")
    });
    assert!(bound_default, "missing arg filled from declared default");
}

#[test]
fn v3_variadic_packs_extras_into_list() {
    let stmts = inlined_top("proc ::v {args} { puts $args }\nv a b c");
    let packed = stmts.iter().any(|s| {
        matches!(s, Statement::AssignValue { name, value, .. }
            if name.starts_with("__inline_") && value == "[list a b c]")
    });
    assert!(packed, "variadic extras packed into a [list …] literal");
}

#[test]
fn v3_variadic_quoted_word_declines() {
    // `"hello world"` would re-tokenise into two list words, inflating the
    // variadic slot — the binder declines and keeps the call.
    let stmts = inlined_top("proc ::v {args} { puts $args }\nv a \"hello world\" c");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "v")),
        "quoted variadic word keeps the call"
    );
}

#[test]
fn v3_empty_variadic_extra_declines() {
    // An empty extra word (`""`) would collapse inside `[list …]` (dropping
    // the element), so the binder declines and keeps the call.
    let stmts = inlined_top("proc ::v {args} { puts $args }\nv a \"\" c");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "v")),
        "empty variadic extra keeps the call"
    );
}

#[test]
fn v3_default_path_preserves_braced_literal_arg() {
    // `f {$caller}` with a defaulted 2nd param takes the defaults path; the
    // braced first arg must still bind as a literal (AssignConst `$caller`),
    // never re-substituted in the caller frame — and the declared default
    // binds as a literal too.
    let stmts = inlined_top("proc ::f {x {y d}} { puts $x }\nf {$caller}");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { value, .. } if value == "$caller")),
        "braced call arg bound as a literal on the defaults path"
    );
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { value, .. } if value == "d")),
        "declared default bound as a literal"
    );
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::AssignValue { value, .. } if value == "$caller")),
        "braced arg must not be re-substituted via AssignValue"
    );
}

#[test]
fn v3_default_value_is_literal() {
    // Tcl proc defaults are literal: `f` with no args binds `x` to the
    // string `$caller`, not the caller's variable.
    let stmts = inlined_top("proc ::f {{x $caller}} { puts $x }\nf");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { value, .. } if value == "$caller")),
        "literal default bound without substitution"
    );
}

#[test]
fn v3_terminal_if_branch_keeps_trailing_return() {
    // The inlined `id 7` sits in the terminal branch of a terminal `if`, so
    // its trailing return forwards `outer`'s value — kept intact, not wrapped.
    let module = inline_module_default(module_for(
        "proc ::id {x} { return $x }\nproc ::outer {} { if {1} { id 7 } }",
    ));
    let outer = &module.procedures["::outer"];
    let clause_body = match &outer.body.statements[0] {
        Statement::If { clauses, .. } => &clauses[0].body,
        other => panic!("expected if, got {other:?}"),
    };
    assert!(
        clause_body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Return { .. })),
        "terminal if-branch keeps the trailing return"
    );
    assert!(
        !clause_body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::While { .. })),
        "terminal if-branch is not wrapped in a while-loop"
    );
    assert!(
        !clause_body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "id")),
        "the call was inlined"
    );
}

#[test]
fn v3_non_terminal_if_branch_still_wraps_return() {
    // Same inline, but now the `if` is followed by another statement, so the
    // branch is not terminal — the trailing return must be wrapped so it
    // doesn't escape `outer` early.
    let module = inline_module_default(module_for(
        "proc ::id {x} { return $x }\nproc ::outer {} { if {1} { id 7 }\n puts done }",
    ));
    let outer = &module.procedures["::outer"];
    let clause_body = match &outer.body.statements[0] {
        Statement::If { clauses, .. } => &clauses[0].body,
        other => panic!("expected if, got {other:?}"),
    };
    assert!(
        clause_body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::While { .. })),
        "non-terminal if-branch wraps the return"
    );
}

#[test]
fn v3_too_many_args_declines() {
    let stmts = inlined_top("proc ::id {x} { puts $x }\nid a b");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "id")),
        "arity overflow keeps the call"
    );
}

#[test]
fn v3_star_expansion_declines() {
    let stmts = inlined_top("proc ::id {x} { puts $x }\nset args {1}\nid {*}$args");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "id")),
        "star-expansion keeps the call (runtime arity)"
    );
}

#[test]
fn v3_trailing_return_at_terminal_is_kept() {
    // The call is the last top-level statement → terminal, so the callee's
    // trailing return is preserved as a caller-level return.
    let stmts = inlined_top("proc ::g {x} { return $x }\ng 7");
    assert!(
        stmts.iter().any(|s| matches!(s, Statement::Return { .. })),
        "terminal trailing return preserved"
    );
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "g")),
        "call replaced"
    );
}

#[test]
fn v3_trailing_return_non_terminal_is_wrapped() {
    // Call followed by another statement → not terminal, so the trailing
    // return is lowered into the one-shot while-wrap instead of escaping
    // the caller.
    let stmts = inlined_top("proc ::g {x} { return $x }\ng 7\nputs after");
    assert!(
        stmts.iter().any(|s| matches!(s, Statement::While { .. })),
        "non-terminal return lowered to a while-wrap"
    );
    assert_eq!(
        top_calls_to(
            &inline_module_default(module_for("proc ::g {x} { return $x }\ng 7\nputs after")),
            "puts"
        ),
        1
    );
}

#[test]
fn v3_non_trailing_return_is_wrapped() {
    let src = "proc ::h {x} { if {$x} { return 1 }\n set y 2 }\nh 0\nputs done";
    let stmts = inlined_top(src);
    assert!(
        stmts.iter().any(|s| matches!(s, Statement::While { .. })),
        "non-trailing return wrapped in a one-shot loop"
    );
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "h")),
        "call replaced"
    );
}

#[test]
fn v3_return_inside_loop_declines() {
    // A `return` inside a `while` body would be trapped by our break-based
    // wrap — v3 declines the proc.
    let stmts = inlined_top("proc ::h {x} { while {1} { return $x } }\nh 1");
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "h")),
        "return-inside-loop proc keeps the call"
    );
}

#[test]
fn idempotent_second_pass_is_noop() {
    let once = inline_module_default(module_for("proc ::id {x} { puts $x }\nid 1"));
    let twice = inline_module(once.clone(), &CommandRegistry::build_default());
    assert_eq!(once, twice, "re-running the inliner changes nothing");
}

// catalogue helpers

#[test]
fn classify_large_single_call_is_if_single_call() {
    // A pure-leaf proc larger than the threshold with exactly one static
    // caller is IF_SINGLE_CALL (size-neutral) — not inlined by the pass.
    let module =
        module_for("proc ::big {} { puts a\n puts b\n puts c\n puts d\n puts e\n puts f }\nbig");
    let summaries = crate::var_escape::analyse_var_escape(&module, true);
    let counts = count_static_calls(&module, &summaries);
    let proc = &module.procedures["::big"];
    assert_eq!(
        classify_proc(proc, summaries.get("::big"), counts["::big"]),
        InlineDecision::IfSingleCall
    );
    // The pass leaves IF_SINGLE_CALL procs alone.
    let inlined = inline_module(module, &CommandRegistry::build_default());
    assert_eq!(top_calls_to(&inlined, "big"), 1);
}
