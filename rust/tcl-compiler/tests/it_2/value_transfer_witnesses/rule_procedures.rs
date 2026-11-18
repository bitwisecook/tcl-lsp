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

//! Rule procedures under `f5-irules`: a `proc` invoked from a `when` body
//! with `call`, the way a rule invokes one.
//!
//! The Tcl-core witnesses run a top-level call, which is no iRule (a rule's
//! top level admits only `when`, `proc`, `priority` and `timing`; IRULE5006,
//! IRULE5007, BIG-IP check item 18), so they leave f5-irules. These take
//! their place, one per fact kind. TMM runs a rule procedure with ordinary
//! `proc` semantics on its Tcl 8.4.6 base (`docs/design/f5/
//! bigip-irule-parser-measurements.md`, BIG-IP 21.1.0.1). A rule procedure
//! has no proved execution target at load, so each fact is read as hosted
//! advice (`semantic_values`, existence, summaries and diagnostics), never as
//! a rewrite.

use super::*;

const IRULES: &str = "f5-irules";

/// `procedure` invoked once from an `HTTP_REQUEST` body as `call p args`.
fn rule(procedure: &str, args: &str) -> String {
    format!("{procedure}\nwhen HTTP_REQUEST {{ call p {args} }}\n")
}

/// The advisory contents of `var`'s last version in `::p`.
fn last_advice(unit: &CompilationUnit, var: &str) -> Option<LatticeValue> {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("the rule procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    let facts = function.semantic_values()?;
    function
        .sccp
        .values
        .keys()
        .filter(|(sym, _)| *sym == symbol)
        .map(|(_, version)| *version)
        .max()
        .and_then(|version| facts.contents((symbol, version)).cloned())
}

#[test]
fn a_rule_procedure_folds_its_cell_updates_as_advice() {
    let unit = unit_of(
        &rule("proc p {} {set n 1; incr n; incr n 2; log local0. $n}", ""),
        IRULES,
    );
    assert_eq!(
        last_advice(&unit, "n"),
        Some(LatticeValue::Const(ConstValue::Int(4)))
    );
    assert_eq!(
        rewrites_of(
            &rule("proc p {} {set n 1; incr n; incr n 2; log local0. $n}", ""),
            IRULES
        ),
        []
    );
}

#[test]
fn a_rule_procedure_formal_is_bound_and_an_unassigned_local_is_not() {
    let unit = unit_of(
        &rule(
            "proc p {a} {\n    if {[info exists a]} {log local0. yes}\n    \
             if {[info exists b]} {log local0. wrong}\n}",
            "1",
        ),
        IRULES,
    );
    assert_eq!(last_existence(&unit, "::p", "a"), SCALAR);
    assert_eq!(last_existence(&unit, "::p", "b"), UNBOUND);
}

#[test]
fn a_rule_procedure_summary_returns_its_constant() {
    let source =
        "proc p {a} {return 1.0}\nwhen HTTP_REQUEST { set y [call p 1]; log local0. $y }\n";
    assert_eq!(
        summary_of(source, IRULES, "::p").constant_return,
        Some(ConstantReturn::Float(1.0))
    );
}

#[test]
fn a_rule_procedure_decides_a_condition_every_member_agrees_on() {
    let source = rule(
        "proc p {} {foreach a {1 2} {if {$a > 0} {log local0. pos} else {log local0. neg}}}",
        "",
    );
    assert_eq!(condition_claims(&source, IRULES), [true]);
}

#[test]
fn a_rule_procedure_loop_test_is_the_branch_that_leaves_it() {
    let source = rule(
        "proc p {} {\n    for {set i 0} {$i < 3} {incr i} {}\n    \
         if {$i == 3} {log local0. yes} else {log local0. no}\n}",
        "",
    );
    assert_eq!(condition_claims(&source, IRULES), [true]);
}

#[test]
fn a_rule_procedure_keeps_a_value_a_failed_match_preserves() {
    let unit = unit_of(
        &rule(
            "proc p {} {\n    set a before\n    regexp {(x)(y)} zz a b\n    log local0. $a\n}",
            "",
        ),
        IRULES,
    );
    assert_eq!(last_advice(&unit, "a"), Some(text("before")));
}

/// Under the 8.4 base a leading `#` element is not quoted: `list # a` is
/// `# a` (tclsh 8.4.20), where 8.5 and later give `{#} a`.
#[test]
fn a_rule_procedure_quotes_a_leading_hash_as_its_84_base_does() {
    let unit = unit_of(
        &rule("proc p {} {set r [list # a]; log local0. $r}", ""),
        IRULES,
    );
    assert_eq!(last_advice(&unit, "r"), Some(text("# a")));
}

#[test]
fn a_rule_procedure_catch_that_may_set_a_variable_is_reported() {
    let source = rule(
        "proc p {c} {catch {if {$c} {set x 1}}; log local0. $x}",
        "0",
    );
    assert!(reports(&source, IRULES, DiagCode::W210));
}

#[test]
fn a_rule_procedure_string_is_test_types_its_arm() {
    let source = rule(
        "proc p {s} {\n    set i [string trim $s]\n    \
         if {[string is integer -strict $i]} {\n        log local0. [lindex {a b c} $i]\n    }\n    \
         log local0. [lindex {a b c} $i]\n}",
        "1",
    );
    assert_eq!(super::refinements::shimmer_lines(&source, IRULES), vec![6]);
}
