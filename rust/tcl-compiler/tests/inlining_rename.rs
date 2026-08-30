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

//! Integration coverage for the inlining α-renamer
//! (`src/inlining/rename.rs`), the lowest-covered inlining file.
//!
//! `inlining.rs` (do not touch) covers the inline *decision* and the
//! call-site *transform* shapes. This suite drives the **renamer**: every
//! statement kind it must descend into and every variable-reference form it
//! must rewrite when a callee's body is spliced into a caller. The renamer
//! is internal (`pub(super) fn rewrite_script`), so it is exercised through
//! the public `inline_module`: a v3 inline α-renames every callee parameter
//! and local to a fresh `__inline_<cid>__<name>` slot, and these tests
//! inspect the renamed result.
//!
//! ## What "collision avoidance / freshening" means here
//!
//! The renamer's anti-capture guarantee is structural: *every* callee local
//! is rewritten to a `__inline_<cid>__` slot whose `<cid>` is unique per call
//! site. So a callee local that shares a name with a caller local (e.g. both
//! named `t`) can never capture it — the callee's `t` becomes
//! `__inline_1__t` while the caller's `t` stays `t`. The tests assert this
//! invariant directly: after inlining, no `__inline_*` slot equals a caller
//! local name, and two call sites get distinct `<cid>` prefixes.
//!
//! ## Structural vs. semantic proof
//!
//! Which fresh suffix a name receives is compiler-internal, so it is asserted
//! structurally (consistency: every reference to one renamed local uses the
//! same new name; no callee local equals a caller local). The load-bearing
//! proof is that α-renaming is **semantics-preserving**: the inlined+renamed
//! program computes the same value as the un-inlined call. Each snippet with
//! an observable value cites the result confirmed against real `tclsh8.6` and
//! `tclsh9.0` via `scripts/dev/tclsh_check.sh` (8.4 / 8.5 are not installed
//! here) in a `// tclsh:` comment, and the test asserts the inlined IR still
//! routes every variable reference so that value is preserved.
//!
//! ## Out of scope (renamer arms unreachable via `inline_module`)
//!
//! Some forms the renamer *handles* are never reached by `inline_module`
//! because an earlier eligibility gate declines the proc before the renamer
//! runs (declining is always semantics-safe — the runtime call stays):
//!   * `upvar` / `global` / `variable` make the proc non-pure-leaf
//!     (`var_escape`), so it is never inlinable. Their renamer arms (the
//!     `Call.defs`/`Call.reads` rewrite for `upvar` targets) are covered by
//!     the renamer's own in-module unit tests, not here.
//!   * `append` / `lappend` lower to a `Call` with a non-empty `defs`, which
//!     `stmt_is_splice_eligible` rejects (see `append_lappend_block_inline`).
//!
//! These exclusions are pinned by tests so the boundary is explicit.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::inlining::inline_module;
use tcl_compiler::ir::{Module, Statement};
use tcl_registry::CommandRegistry;

// Harness (mirrors `inlining.rs`)

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// `source → Module`.
fn module_for(source: &str) -> Module {
    CompilationUnit::build_for(source, &reg(), false).ir_module
}

/// Run the whole-module inline transform over a fresh module.
fn inlined(source: &str) -> Module {
    inline_module(module_for(source), &reg())
}

/// The body statements of one proc in `module`.
fn proc_body<'a>(module: &'a Module, qname: &str) -> &'a [Statement] {
    &module
        .procedures
        .get(qname)
        .unwrap_or_else(|| panic!("proc {qname} missing"))
        .body
        .statements
}

/// Count statement-position calls to `command` in a flat statement list.
fn calls_to(stmts: &[Statement], command: &str) -> usize {
    stmts
        .iter()
        .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
        .count()
}

/// Recursively collect every assignment-target name (`AssignConst`,
/// `AssignValue`, `AssignExpr`, `Incr`) reachable from a statement list,
/// descending into all nested bodies. Used to find the renamed slots.
fn collect_assign_names(stmts: &[Statement], out: &mut Vec<String>) {
    for s in stmts {
        match s {
            Statement::AssignConst { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::Incr { name, .. } => out.push(name.clone()),
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                collect_assign_names(&body.statements, out);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    collect_assign_names(&c.body.statements, out);
                }
                if let Some(b) = else_body {
                    collect_assign_names(&b.statements, out);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_assign_names(&init.statements, out);
                collect_assign_names(&next.statements, out);
                collect_assign_names(&body.statements, out);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. } => collect_assign_names(&body.statements, out),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_assign_names(&body.statements, out);
                for h in handlers {
                    collect_assign_names(&h.body.statements, out);
                }
                if let Some(b) = finally_body {
                    collect_assign_names(&b.statements, out);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        collect_assign_names(&b.statements, out);
                    }
                }
                if let Some(b) = default_body {
                    collect_assign_names(&b.statements, out);
                }
            }
            _ => {}
        }
    }
}

/// Every assignment-target name reachable from a statement list.
fn all_assign_names(stmts: &[Statement]) -> Vec<String> {
    let mut v = Vec::new();
    collect_assign_names(stmts, &mut v);
    v
}

/// Recursively collect every `${...}` / `$name` token's variable text that
/// appears in a `puts`/`Call` argument across a statement list. Returns the
/// raw arg strings so a test can check the embedded substitution name.
fn collect_call_args(stmts: &[Statement], command: &str, out: &mut Vec<String>) {
    for s in stmts {
        match s {
            Statement::Call {
                command: c, args, ..
            } if c == command => out.extend(args.iter().cloned()),
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                collect_call_args(&body.statements, command, out);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for cl in clauses {
                    collect_call_args(&cl.body.statements, command, out);
                }
                if let Some(b) = else_body {
                    collect_call_args(&b.statements, command, out);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_call_args(&init.statements, command, out);
                collect_call_args(&next.statements, command, out);
                collect_call_args(&body.statements, command, out);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. } => collect_call_args(&body.statements, command, out),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_call_args(&body.statements, command, out);
                for h in handlers {
                    collect_call_args(&h.body.statements, command, out);
                }
                if let Some(b) = finally_body {
                    collect_call_args(&b.statements, command, out);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        collect_call_args(&b.statements, command, out);
                    }
                }
                if let Some(b) = default_body {
                    collect_call_args(&b.statements, command, out);
                }
            }
            _ => {}
        }
    }
}

/// Every argument string passed to `command` across a statement list.
fn all_call_args(stmts: &[Statement], command: &str) -> Vec<String> {
    let mut v = Vec::new();
    collect_call_args(stmts, command, &mut v);
    v
}

/// Pull the single `__inline_<cid>__<suffix>` slot ending in `__<suffix>`
/// out of a name list, asserting exactly one matches.
fn mangled_ending(names: &[String], suffix: &str) -> String {
    let hits: Vec<&String> = names
        .iter()
        .filter(|n| n.starts_with("__inline_") && n.ends_with(suffix))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one mangled slot ending {suffix:?}, got {names:?}"
    );
    hits[0].clone()
}

/// True iff any name in the list is a mangled `__inline_*` slot.
fn has_mangled(names: &[String]) -> bool {
    names.iter().any(|n| n.starts_with("__inline_"))
}

// 1. Assignment-target rename arms — set / incr (AssignConst/Value/Expr/Incr)

#[test]
fn set_const_target_is_renamed() {
    // `set marker 1` → `set __inline_N__marker 1`. The renamer rewrites the
    // `AssignConst.name` field via `rename_var_name`.
    let out = inlined("proc f {} { set marker 1 }\nf\n");
    let names = all_assign_names(&out.top_level.statements);
    let slot = mangled_ending(&names, "__marker");
    assert!(slot.starts_with("__inline_"));
    assert_eq!(calls_to(&out.top_level.statements, "f"), 0, "call inlined");
}

#[test]
fn incr_target_and_read_renamed_consistently() {
    // tclsh (8.6, 9.0): `proc f {} { set c 0; incr c 2; return $c }; f` → 2.
    // The `Incr.name` and the later `$c` read must use the SAME mangled slot,
    // or the read would resolve to a different (uninitialised) variable.
    let out = inlined("proc f {} { set c 0\nincr c 2\nputs $c }\nf\n");
    let stmts = &out.top_level.statements;
    // Both the `set c` (AssignConst) and `incr c` (Incr) target the local
    // `c`, so the renamer must give BOTH the *same* mangled slot — that
    // consistency is the property under test. Collect every `__c` slot and
    // assert they are all one identical name.
    let c_slots: Vec<String> = all_assign_names(stmts)
        .into_iter()
        .filter(|n| n.starts_with("__inline_") && n.ends_with("__c"))
        .collect();
    assert_eq!(
        c_slots.len(),
        2,
        "both the set and the incr write the renamed c slot, got {c_slots:?}"
    );
    assert_eq!(
        c_slots[0], c_slots[1],
        "set and incr rename `c` to the SAME slot (consistency)"
    );
    let slot = c_slots[0].clone();
    // Confirm specifically the Incr target carries that slot.
    let incr_renamed = stmts
        .iter()
        .any(|s| matches!(s, Statement::Incr { name, .. } if *name == slot));
    assert!(incr_renamed, "incr target renamed to {slot}");
    // The `puts $c` read references the same slot (as `${slot}`).
    let puts_args = all_call_args(stmts, "puts");
    assert!(
        puts_args.iter().any(|a| a.contains(&slot)),
        "read of c resolves to the same mangled slot {slot}; puts args {puts_args:?}"
    );
}

#[test]
fn incr_amount_substitution_is_renamed() {
    // tclsh (8.6, 9.0): `proc f {n} { set acc 0; incr acc $n; return $acc };
    // f 5` → 5. The `incr acc $n` amount field carries `$n`, which is a
    // parameter — it must be renamed in `Incr.amount`.
    let out = inlined("proc f {n} { set acc 0\nincr acc $n\nputs $acc }\nf 5\n");
    let stmts = &out.top_level.statements;
    let amount = stmts.iter().find_map(|s| match s {
        Statement::Incr { amount, .. } => amount.clone(),
        _ => None,
    });
    let amount = amount.expect("an incr with an amount is spliced");
    assert!(
        amount.contains("__inline_") && amount.contains("__n"),
        "incr amount renames the $n param reference, got {amount:?}"
    );
}

#[test]
fn assign_expr_operands_are_renamed() {
    // tclsh (8.6, 9.0): `proc add {a b} { set t [expr {$a + $b}]; return $t };
    // add 3 4` → 7. The `set t [expr ...]` lowers to an AssignExpr whose
    // expr-AST `$a` / `$b` operands must each be rewritten by `rewrite_expr`.
    let out = inlined("proc add {a b} { set t [expr {$a + $b}]\nreturn $t }\nadd 3 4\n");
    let stmts = &out.top_level.statements;
    let expr = stmts.iter().find_map(|s| match s {
        Statement::AssignExpr { expr, .. } => Some(expr.clone()),
        _ => None,
    });
    let ExprNode::Binary { left, right, .. } = expr.expect("AssignExpr spliced") else {
        panic!("expected a Binary expr operand tree");
    };
    let var_name = |n: &ExprNode| match n {
        ExprNode::Var { name, .. } => name.clone(),
        other => panic!("expected a Var operand, got {other:?}"),
    };
    let lname = var_name(&left);
    let rname = var_name(&right);
    assert!(
        lname.starts_with("__inline_") && lname.ends_with("__a"),
        "left operand $a renamed, got {lname}"
    );
    assert!(
        rname.starts_with("__inline_") && rname.ends_with("__b"),
        "right operand $b renamed, got {rname}"
    );
    // The two operands share the SAME cid prefix (same call site).
    let cid = |n: &str| {
        n.trim_start_matches("__inline_")
            .split("__")
            .next()
            .unwrap()
            .to_owned()
    };
    assert_eq!(
        cid(&lname),
        cid(&rname),
        "both operands share one call-site cid"
    );
}

#[test]
fn assign_expr_braced_operand_is_renamed() {
    // tclsh (8.6, 9.0): `proc f {a} { set t [expr {${a} + 1}]; return $t };
    // f 5` → 6. The `${a}` brace-form var node must rewrite both its `name`
    // and its `text` (the renamer updates `text` so the brace form stays
    // coherent: `${a}` → `${__inline_N__a}`).
    let out = inlined("proc f {a} { set t [expr {${a} + 1}]\nputs $t }\nf 5\n");
    let expr = out.top_level.statements.iter().find_map(|s| match s {
        Statement::AssignExpr { expr, .. } => Some(expr.clone()),
        _ => None,
    });
    let ExprNode::Binary { left, .. } = expr.expect("AssignExpr spliced") else {
        panic!("expected Binary");
    };
    let ExprNode::Var { name, text, .. } = *left else {
        panic!("expected a Var left operand");
    };
    assert!(
        name.starts_with("__inline_") && name.ends_with("__a"),
        "brace-form operand name renamed, got {name}"
    );
    assert!(
        text.contains('{') && text.contains(&name),
        "brace-form `text` rewritten to carry the new name, got {text:?}"
    );
}

// 2. value-string substitution arms — $name / ${name} / array / command-sub

#[test]
fn value_string_dollar_read_is_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { set y 5; return $y }; f` → 5. The body's
    // `$y` read (a value-string substitution) must be renamed to the same
    // slot the `set y` write produced.
    let out = inlined("proc f {} { set y 5\nputs $y }\nf\n");
    let stmts = &out.top_level.statements;
    let slot = mangled_ending(&all_assign_names(stmts), "__y");
    let puts_args = all_call_args(stmts, "puts");
    assert!(
        puts_args.iter().any(|a| a.contains(&slot)),
        "value-string read of $y renamed to {slot}, puts args {puts_args:?}"
    );
}

#[test]
fn command_subst_read_inside_value_is_renamed() {
    // tclsh (8.6, 9.0): `proc f {x} { set y [string length $x]; return $y };
    // f abc` → 3. The `$x` lives inside a `[string length $x]` command
    // substitution embedded in the value string — the renamer rewrites the
    // `$x` token while preserving the `[...]` wrapper verbatim.
    let out = inlined("proc f {x} { set y [string length $x]\nputs $y }\nf abc\n");
    let value = out.top_level.statements.iter().find_map(|s| match s {
        Statement::AssignValue { name, value, .. } if name.ends_with("__y") => Some(value.clone()),
        _ => None,
    });
    let value = value.expect("the command-sub assignment is spliced");
    assert!(
        value.starts_with("[string length ") && value.ends_with(']'),
        "command-sub wrapper preserved, got {value:?}"
    );
    assert!(
        value.contains("__inline_") && value.contains("__x"),
        "the $x read inside the [...] is renamed, got {value:?}"
    );
}

#[test]
fn array_element_target_renames_base_keeps_index() {
    // tclsh (8.6, 9.0): `proc f {} { set a(1) 5; return $a(1) }; f` → 5. An
    // array element `a(1)`: the renamer rewrites the array *base* (`a`) and
    // preserves the `(1)` index verbatim, on both the write and the read.
    let out = inlined("proc f {} { set a(1) 5\nputs $a(1) }\nf\n");
    let stmts = &out.top_level.statements;
    let write = stmts.iter().find_map(|s| match s {
        Statement::AssignConst { name, .. } if name.contains('(') => Some(name.clone()),
        _ => None,
    });
    let write = write.expect("an array-element write is spliced");
    assert!(
        write.starts_with("__inline_") && write.ends_with("__a(1)"),
        "array base renamed, index preserved, got {write:?}"
    );
    let base = write.split('(').next().unwrap().to_owned();
    let puts_args = all_call_args(stmts, "puts");
    assert!(
        puts_args
            .iter()
            .any(|a| a.contains(&base) && a.contains("(1)")),
        "the $a(1) read targets the same renamed base with the index kept, got {puts_args:?}"
    );
}

#[test]
fn braced_name_target_is_renamed_and_flag_preserved() {
    // `set {x} 1` — the name word was a braced literal, so `name_braced` is
    // true. The renamer must rewrite the name and the read `${x}` while
    // leaving the braced flag intact.
    // tclsh (8.6, 9.0): `proc f {} { set {x} 1; return ${x} }; f` → 1.
    let out = inlined("proc f {} { set {x} 1\nputs ${x} }\nf\n");
    let stmts = &out.top_level.statements;
    let (slot, braced) = stmts
        .iter()
        .find_map(|s| match s {
            Statement::AssignConst {
                name, name_braced, ..
            } if name.ends_with("__x") => Some((name.clone(), *name_braced)),
            _ => None,
        })
        .expect("braced-name set spliced");
    assert!(slot.starts_with("__inline_"), "braced name renamed");
    assert!(braced, "name_braced flag preserved through the rename");
    let puts_args = all_call_args(stmts, "puts");
    assert!(
        puts_args.iter().any(|a| a.contains(&slot)),
        "the ${{x}} read renamed to the same slot {slot}"
    );
}

#[test]
fn qualified_name_passes_through_unrenamed() {
    // The rename map only carries the callee's locals. A `$::ns::x` reference
    // is a global / namespace var — it must pass through verbatim even when a
    // *local* `x` in the same body IS renamed. This proves the renamer keys on
    // exact names, not on substrings.
    // tclsh (8.6, 9.0): `namespace eval ::ns { variable x 42 }
    //   proc f {} { set x 1; return $::ns::x }; f` → 42 (the qualified read is
    // the namespace var, untouched by the local rename).
    let out = inlined("proc f {} { set x 1\nputs $::ns::x\nputs $x }\nf\n");
    let stmts = &out.top_level.statements;
    // The local `x` write is renamed.
    let slot = mangled_ending(&all_assign_names(stmts), "__x");
    let puts_args = all_call_args(stmts, "puts");
    // Exactly one puts reads the renamed local slot; another reads the
    // untouched qualified `::ns::x`.
    assert!(
        puts_args
            .iter()
            .any(|a| a.contains("::ns::x") && !a.contains("__inline_")),
        "qualified $::ns::x passes through unrenamed, got {puts_args:?}"
    );
    assert!(
        puts_args.iter().any(|a| a.contains(&slot)),
        "the bare local $x is renamed to {slot}, got {puts_args:?}"
    );
}

// 3. nested control-flow body descent — if / while / for / foreach / switch

#[test]
fn if_clause_condition_and_body_renamed() {
    // tclsh (8.6, 9.0): `proc f {x} { set r 0; if {$x > 0} { set r 1 };
    // return $r }; f 5` → 1. The renamer must rewrite the `If` clause's
    // condition expr (`$x`) AND the clause body's `set r` target.
    let out = inlined("proc f {x} { set r 0\nif {$x > 0} { set r 1 }\nputs $r }\nf 5\n");
    let if_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::If { .. }))
        .expect("the nested if survives inlining");
    let Statement::If { clauses, .. } = if_node else {
        unreachable!()
    };
    // Condition operand `$x` renamed.
    let ExprNode::Binary { left, .. } = &clauses[0].condition else {
        panic!("expected a Binary condition");
    };
    let ExprNode::Var { name, .. } = left.as_ref() else {
        panic!("expected $x Var in condition");
    };
    assert!(
        name.starts_with("__inline_") && name.ends_with("__x"),
        "if-condition $x renamed, got {name}"
    );
    // Clause body's `set r 1` renamed.
    let body_names = all_assign_names(&clauses[0].body.statements);
    assert!(
        body_names
            .iter()
            .any(|n| n.starts_with("__inline_") && n.ends_with("__r")),
        "if-body set target renamed, got {body_names:?}"
    );
}

#[test]
fn while_condition_and_body_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { set i 0; while {$i < 3} { incr i };
    // return $i }; f` → 3. The `While.condition` ($i) and body (`incr i`)
    // both reference the same callee local and must rename to one slot.
    let out = inlined("proc f {} { set i 0\nwhile {$i < 3} { incr i } }\nf\n");
    let while_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::While { .. }))
        .expect("the while survives inlining");
    let Statement::While {
        condition, body, ..
    } = while_node
    else {
        unreachable!()
    };
    let ExprNode::Binary { left, .. } = condition else {
        panic!("expected Binary while-condition");
    };
    let ExprNode::Var {
        name: cond_name, ..
    } = left.as_ref()
    else {
        panic!("expected $i Var");
    };
    assert!(
        cond_name.starts_with("__inline_") && cond_name.ends_with("__i"),
        "while-condition $i renamed, got {cond_name}"
    );
    let body_incr = body.statements.iter().find_map(|s| match s {
        Statement::Incr { name, .. } => Some(name.clone()),
        _ => None,
    });
    assert_eq!(
        body_incr.as_deref(),
        Some(cond_name.as_str()),
        "while-body incr uses the SAME renamed slot as the condition"
    );
}

#[test]
fn for_init_cond_next_body_all_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { set acc 0; for {set i 0} {$i < 3}
    // {incr i} { incr acc $i }; return $acc }; f` → 3 — but that body has 6
    // statements (> threshold), so use the smaller whole-body `for` that is
    // ALWAYS-inlinable. The loop var `i` is a callee local referenced in all
    // four `for` sub-scripts; every one must rename to the same slot.
    // tclsh (8.6, 9.0): `proc f {} { for {set i 0} {$i < 3} {incr i}
    // { puts $i } }; f` prints 0\n1\n2.
    let out = inlined("proc f {} { for {set i 0} {$i < 3} {incr i} { puts $i } }\nf\n");
    let for_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::For { .. }))
        .expect("the for survives inlining");
    let Statement::For {
        init,
        condition,
        next,
        body,
        ..
    } = for_node
    else {
        unreachable!()
    };
    // init: `set i 0`
    let init_name = mangled_ending(&all_assign_names(&init.statements), "__i");
    // condition: `$i < 3`
    let ExprNode::Binary { left, .. } = condition else {
        panic!("Binary condition");
    };
    let ExprNode::Var {
        name: cond_name, ..
    } = left.as_ref()
    else {
        panic!("$i Var");
    };
    // next: `incr i`
    let next_name = next
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::Incr { name, .. } => Some(name.clone()),
            _ => None,
        })
        .expect("next has the incr");
    // body: `puts $i`
    let body_args = all_call_args(&body.statements, "puts");
    assert_eq!(&init_name, cond_name, "init and condition share the slot");
    assert_eq!(init_name, next_name, "init and next share the slot");
    assert!(
        body_args.iter().any(|a| a.contains(&init_name)),
        "for-body read uses the same renamed slot {init_name}, got {body_args:?}"
    );
}

#[test]
fn foreach_iterator_vars_and_body_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { set acc 0; foreach {a b} {1 2 3 4}
    // { incr acc $a; incr acc $b }; return $acc }; f` → 10. Both iterator
    // variables of a multi-var `foreach` are callee locals — each must be
    // renamed, and the body's reads must match.
    let out = inlined(
        "proc f {} { set acc 0\nforeach {a b} {1 2 3 4} { incr acc $a\nincr acc $b }\nputs $acc }\nf\n",
    );
    let fe = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Foreach { .. }))
        .expect("the foreach survives inlining");
    let Statement::Foreach {
        iterators, body, ..
    } = fe
    else {
        unreachable!()
    };
    let vars = &iterators[0].vars;
    assert_eq!(vars.len(), 2, "two iterator vars");
    assert!(
        vars[0].starts_with("__inline_") && vars[0].ends_with("__a"),
        "first iterator var renamed, got {:?}",
        vars[0]
    );
    assert!(
        vars[1].starts_with("__inline_") && vars[1].ends_with("__b"),
        "second iterator var renamed, got {:?}",
        vars[1]
    );
    // The body's `incr acc $a` / `incr acc $b` amounts reference both vars.
    let amounts: Vec<String> = body
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Incr { amount, .. } => amount.clone(),
            _ => None,
        })
        .collect();
    assert!(
        amounts.iter().any(|a| a.contains(&vars[0])),
        "body reads renamed iterator var a, got {amounts:?}"
    );
    assert!(
        amounts.iter().any(|a| a.contains(&vars[1])),
        "body reads renamed iterator var b, got {amounts:?}"
    );
    // The list_arg `{1 2 3 4}` is a literal — no rename, preserved verbatim.
    assert_eq!(
        iterators[0].list_arg, "1 2 3 4",
        "literal list_arg untouched"
    );
}

#[test]
fn switch_subject_and_arm_body_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { set s a; switch $s { a { return "got-$s" }
    // default { return no } } }; f` → "got-a". The `Switch.subject` (`$s`)
    // and the matched arm body's `$s` read must both rename to one slot.
    let out = inlined("proc f {} { set s a\nswitch $s { a { puts $s } } }\nf\n");
    let sw = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Switch { .. }))
        .expect("the switch survives inlining");
    let Statement::Switch { subject, arms, .. } = sw else {
        unreachable!()
    };
    let slot = mangled_ending(&all_assign_names(&out.top_level.statements), "__s");
    assert!(
        subject.contains(&slot),
        "switch subject $s renamed to {slot}, got {subject:?}"
    );
    let arm_args = all_call_args(
        arms[0]
            .body
            .as_ref()
            .expect("arm body")
            .statements
            .as_slice(),
        "puts",
    );
    assert!(
        arm_args.iter().any(|a| a.contains(&slot)),
        "arm body $s read renamed to the same slot {slot}, got {arm_args:?}"
    );
    // The pattern `a` is a literal — never renamed.
    assert_eq!(
        arms[0].pattern, "a",
        "switch pattern is not a variable, untouched"
    );
}

// 4. exception-binding rename arms — catch / try (rename.rs ~L270)

#[test]
fn catch_result_and_options_vars_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { catch { set e 1 } msg opts; return $msg };
    // f` → 1. The `Catch.result_var` (`msg`), `Catch.options_var` (`opts`),
    // AND the body's `set e` local must all be renamed; the later `$msg` read
    // must match the result_var slot.
    let out = inlined("proc f {} { catch { set e 1 } msg opts\nputs $msg }\nf\n");
    let stmts = &out.top_level.statements;
    let catch = stmts
        .iter()
        .find(|s| matches!(s, Statement::Catch { .. }))
        .expect("the catch survives inlining");
    let Statement::Catch {
        body,
        result_var,
        options_var,
        ..
    } = catch
    else {
        unreachable!()
    };
    let rv = result_var.clone().expect("result var present");
    let ov = options_var.clone().expect("options var present");
    assert!(
        rv.starts_with("__inline_") && rv.ends_with("__msg"),
        "catch result_var renamed, got {rv}"
    );
    assert!(
        ov.starts_with("__inline_") && ov.ends_with("__opts"),
        "catch options_var renamed, got {ov}"
    );
    // Body's `set e 1` renamed.
    assert!(
        has_mangled(&all_assign_names(&body.statements)),
        "catch body local set renamed"
    );
    // `puts $msg` reads the same slot as result_var.
    let puts_args = all_call_args(stmts, "puts");
    assert!(
        puts_args.iter().any(|a| a.contains(&rv)),
        "the $msg read resolves to the renamed result_var {rv}, got {puts_args:?}"
    );
}

#[test]
fn try_on_handler_var_and_body_renamed() {
    // tclsh (8.6, 9.0): `proc f {} { try { set x 1 } on ok {res opts}
    // { return "ok-$res" } }; f` → "ok-1" (try body's value is the handler
    // input). Structurally: the `on` handler's var_name (`msg`), options_var
    // (`opts`), try-body local (`x`), and handler-body `$msg` read all rename.
    let out = inlined("proc f {} { try { set x 1 } on error {msg opts} { puts $msg } }\nf\n");
    let try_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Try { .. }))
        .expect("the try survives inlining");
    let Statement::Try { body, handlers, .. } = try_node else {
        unreachable!()
    };
    // try body local `x` renamed.
    assert!(
        has_mangled(&all_assign_names(&body.statements)),
        "try-body local renamed"
    );
    let h = &handlers[0];
    let vn = h.var_name.clone().expect("handler var");
    let ov = h.options_var.clone().expect("handler options var");
    assert!(
        vn.starts_with("__inline_") && vn.ends_with("__msg"),
        "handler var_name renamed, got {vn}"
    );
    assert!(
        ov.starts_with("__inline_") && ov.ends_with("__opts"),
        "handler options_var renamed, got {ov}"
    );
    // Handler body `puts $msg` uses the same slot as the handler var.
    let hb_args = all_call_args(&h.body.statements, "puts");
    assert!(
        hb_args.iter().any(|a| a.contains(&vn)),
        "handler body $msg read renamed to the handler var slot {vn}, got {hb_args:?}"
    );
}

#[test]
fn try_trap_handler_vars_renamed() {
    // A `trap {ERRCODE} {m o}` handler — same rename arm as `on`, exercising
    // the `kind == "trap"` path. The match_arg (`ARITH`) is a literal error
    // code, never a variable; only the handler vars and body rename.
    let out = inlined("proc f {} { try { set x 1 } trap {ARITH} {m o} { puts $m } }\nf\n");
    let Statement::Try { handlers, .. } = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Try { .. }))
        .expect("try survives")
    else {
        unreachable!()
    };
    let h = &handlers[0];
    assert_eq!(h.kind, "trap", "trap handler kind preserved");
    assert_eq!(h.match_arg, "ARITH", "literal error code untouched");
    let vn = h.var_name.clone().expect("trap var");
    assert!(
        vn.starts_with("__inline_") && vn.ends_with("__m"),
        "trap handler var renamed, got {vn}"
    );
    let hb_args = all_call_args(&h.body.statements, "puts");
    assert!(
        hb_args.iter().any(|a| a.contains(&vn)),
        "trap body $m read renamed to {vn}, got {hb_args:?}"
    );
}

// 5. collision avoidance / freshening — the load-bearing capture-avoidance

#[test]
fn callee_local_colliding_with_caller_local_is_freshened() {
    // THE capture-avoidance proof. Caller proc has a local `t` = 99; the
    // callee `add` also has a local `t`. After inlining `add 3 4` at
    // statement position, the callee's `t` becomes `__inline_N__t`, leaving
    // the caller's `t` (still spelled `t`) untouched. The caller's later
    // `set out $t` reads the caller's 99 — NOT the callee's 7.
    //
    // tclsh (8.6, 9.0):
    //   proc add {a b} { set t [expr {$a + $b}]; return $t }
    //   proc caller {} { set t 99; add 3 4; set out $t; puts $out }; caller
    // → 99 (the callee's computation of 7 never clobbers the caller's t).
    let out = inlined(
        "proc add {a b} { set t [expr {$a + $b}]\nreturn $t }\n\
         proc caller {} { set t 99\nadd 3 4\nset out $t\nputs $out }\n",
    );
    let body = proc_body(&out, "::caller");
    assert_eq!(calls_to(body, "add"), 0, "the add call was inlined");

    // The caller's own `t` write stays spelled `t` (un-mangled, value 99).
    let caller_t = body.iter().any(|s| {
        matches!(
            s, Statement::AssignConst { name, value, .. } if name == "t" && value == "99"
        )
    });
    assert!(caller_t, "caller's own `t`=99 is preserved un-renamed");

    // The callee's `t` is freshened to a mangled slot somewhere in the body
    // (it lives inside the one-shot while-wrap the non-terminal return uses).
    let all_names = all_assign_names(body);
    let callee_mangled = mangled_ending(&all_names, "__t");
    assert_ne!(
        callee_mangled, "t",
        "the callee local is a distinct fresh name"
    );

    // No mangled slot is literally `t` — i.e. the freshening never produces a
    // name equal to a caller local (the structural anti-capture invariant).
    assert!(
        !all_names
            .iter()
            .any(|n| n == "t" && n.starts_with("__inline_")),
        "freshened names never equal a caller local"
    );

    // The caller's read `set out $t` references the caller's `t`, NOT the
    // mangled callee slot.
    let out_read = body.iter().find_map(|s| match s {
        Statement::AssignValue { name, value, .. } if name == "out" => Some(value.clone()),
        _ => None,
    });
    let out_read = out_read.expect("`set out $t` present");
    assert!(
        out_read.contains("${t}") || out_read == "$t",
        "caller read targets the caller's t, got {out_read:?}"
    );
    assert!(
        !out_read.contains("__inline_"),
        "caller read is NOT captured by the callee's freshened slot, got {out_read:?}"
    );
}

#[test]
fn two_call_sites_get_distinct_fresh_prefixes() {
    // tclsh (8.6, 9.0): `proc add {a b} { set t [expr {$a + $b}]; puts $t }
    //   proc caller {} { add 1 2; add 3 4 }; caller` → prints 3 then 7. Two
    // statement-position inlines of the same callee must get DIFFERENT cid
    // prefixes so their `t` slots (and param slots) never alias.
    let out = inlined(
        "proc add {a b} { set t [expr {$a + $b}]\nputs $t }\n\
         proc caller {} { add 1 2\nadd 3 4 }\n",
    );
    let body = proc_body(&out, "::caller");
    assert_eq!(calls_to(body, "add"), 0, "both calls inlined");
    let t_slots: Vec<String> = all_assign_names(body)
        .into_iter()
        .filter(|n| n.starts_with("__inline_") && n.ends_with("__t"))
        .collect();
    assert_eq!(t_slots.len(), 2, "each site emits its own `t` slot");
    assert_ne!(
        t_slots[0], t_slots[1],
        "the two call sites get distinct fresh prefixes (no aliasing)"
    );
    // Each spliced `puts` reads the slot from its own site.
    let puts_args = all_call_args(body, "puts");
    assert!(
        puts_args.contains(&format!("${{{}}}", t_slots[0]))
            && puts_args.contains(&format!("${{{}}}", t_slots[1])),
        "each spliced puts reads its own site's renamed slot, got {puts_args:?}"
    );
}

#[test]
fn callee_local_shadowing_caller_param_is_freshened() {
    // A subtler collision: the callee's local name equals the *caller's
    // parameter* name. Caller `caller` has param `sum`; callee `work` has a
    // local `sum`. The inlined `work 5` must freshen the callee's `sum` so it
    // can't clobber the caller's `sum` parameter.
    //
    // tclsh (8.6, 9.0):
    //   proc work {x} { set sum [expr {$x * 2}]; puts $sum }
    //   proc caller {sum} { work 5; puts $sum }; caller 1
    // → prints 10 (callee) then 1 (caller's param) — never 10 twice.
    let out = inlined(
        "proc work {x} { set sum [expr {$x * 2}]\nputs $sum }\n\
         proc caller {sum} { work 5\nputs $sum }\n",
    );
    let body = proc_body(&out, "::caller");
    assert_eq!(calls_to(body, "work"), 0, "the work call inlined");
    // Callee's `sum` freshened.
    let callee_sum = mangled_ending(&all_assign_names(body), "__sum");
    assert_ne!(callee_sum, "sum");
    // The caller's `puts $sum` (param read) must reference the bare `sum`,
    // never the callee's freshened slot.
    let puts_args = all_call_args(body, "puts");
    let bare_sum_read = puts_args.iter().any(|a| a == "${sum}" || a == "$sum");
    let mangled_sum_read = puts_args.iter().any(|a| a.contains(&callee_sum));
    assert!(
        bare_sum_read,
        "caller's param read stays bare `sum`, got {puts_args:?}"
    );
    assert!(
        mangled_sum_read,
        "callee's own read uses its freshened slot {callee_sum}, got {puts_args:?}"
    );
}

// 6. boundary pins — forms the renamer is never asked to rewrite

#[test]
fn append_lappend_block_inline_so_renamer_not_reached() {
    // `append` / `lappend` lower to a `Call` with a non-empty `defs`, which
    // the splice-eligibility gate rejects — so the proc is never inlined and
    // the renamer is never asked to rewrite them. Pin that boundary: the call
    // to `f` survives. (Declining is always semantics-safe.)
    let after_append = inlined("proc f {} { set s a\nappend s b }\nf\n");
    assert_eq!(
        calls_to(&after_append.top_level.statements, "f"),
        1,
        "append in body blocks inlining; renamer not invoked"
    );
    let lafter_append = inlined("proc f {} { set l x\nlappend l y }\nf\n");
    assert_eq!(
        calls_to(&lafter_append.top_level.statements, "f"),
        1,
        "lappend in body blocks inlining; renamer not invoked"
    );
}

#[test]
fn upvar_body_blocks_inline_so_renamer_not_reached() {
    // `upvar` makes the proc non-pure-leaf (`var_escape`), so it is never
    // inlinable and the renamer's `Call.reads`/`defs` rename arm for upvar
    // targets is never reached via `inline_module`. Pin it.
    let out =
        inlined("proc setter {name val} { upvar 1 $name v\nset v $val }\nset x 0\nsetter x 9\n");
    assert_eq!(
        calls_to(&out.top_level.statements, "setter"),
        1,
        "upvar body is non-pure-leaf; call kept, renamer not invoked"
    );
}

#[test]
fn empty_rename_map_is_an_identity_pass() {
    // A zero-parameter, local-free wrapper has an empty rename map, so the
    // renamer's fast path returns the script clone unchanged. The verbatim
    // splice still drops the call boundary, and the body `puts` is unaltered.
    // tclsh (8.6, 9.0): `proc w {} { puts hi }; w` prints "hi".
    let out = inlined("proc w {} { puts hi }\nw\n");
    assert_eq!(
        calls_to(&out.top_level.statements, "w"),
        0,
        "wrapper inlined"
    );
    let puts_args = all_call_args(&out.top_level.statements, "puts");
    assert!(
        puts_args.iter().any(|a| a == "hi"),
        "no-rename body spliced verbatim, got {puts_args:?}"
    );
    // No mangled slot is introduced when there is nothing to rename.
    assert!(
        !has_mangled(&all_assign_names(&out.top_level.statements)),
        "empty rename map introduces no __inline_ slot"
    );
}

#[test]
fn idempotent_reinlining_preserves_renamed_names() {
    // Re-running the inliner over an already-inlined module is a no-op: the
    // renamed slots stay byte-identical (the renamer never double-mangles).
    let once =
        inlined("proc add {a b} { set t [expr {$a + $b}]\nputs $t }\nproc caller {} { add 1 2 }\n");
    let twice = inline_module(once.clone(), &reg());
    assert_eq!(once, twice, "second pass leaves renamed names unchanged");
}

// 7. nested-script descent through a Script helper (compile-coverage)

#[test]
fn renamer_descends_through_block_in_terminal_if() {
    // A trailing return inside the taken branch of a terminal `if` is kept
    // intact (forwarded), and the renamer must have descended through the
    // `If` clause body Script to rename the returned `$x`.
    // tclsh (8.6, 9.0): `proc id {x} { return $x }
    //   proc outer {} { if {1} { id 7 } }; outer` → 7.
    let out = inlined("proc id {x} { return $x }\nproc outer {} { if {1} { id 7 } }\n");
    let body = proc_body(&out, "::outer");
    let Statement::If { clauses, .. } = &body[0] else {
        panic!("expected if at outer body head");
    };
    let clause = &clauses[0].body;
    // The param binding `__inline_N__x = 7` is inside the clause body.
    let names = all_assign_names(&clause.statements);
    let slot = mangled_ending(&names, "__x");
    // The trailing return forwards `${slot}`.
    let ret_val = clause.statements.iter().find_map(|s| match s {
        Statement::Return { value, .. } => value.clone(),
        _ => None,
    });
    assert_eq!(
        ret_val,
        Some(format!("${{{slot}}}")),
        "the kept terminal return forwards the renamed param slot"
    );
}
