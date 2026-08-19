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

//! Dialect threading through lowering and the compilation unit (issue #1048).
//!
//! The document's dialect has to reach three layers before a dialect-only
//! operator behaves like an operator: the lexer config (word tokenisation),
//! the *expression* grammar the lowering parses conditions with, and the fold
//! policy the lattice pipeline runs under. The lowering layer used to parse
//! every `if` / `while` / `for` / `expr` condition with a hardcoded `None`
//! dialect, so an iRules word operator reached the IR as an opaque
//! `ExprNode::Raw` that no fold could evaluate — the constant-condition
//! diagnostic (I230) could not fire even with an explicit `--dialect`.
//!
//! The cases below pin the four corners for I230 on a word-operator condition:
//! it fires under `f5-irules` when the subject is constant (TP), stays silent
//! in plain Tcl where `contains` is not an operator at all (TN), stays silent
//! under `f5-irules` when the subject is *not* constant (FP guard), and the
//! pre-existing plain-Tcl constant condition still fires (FN guard).

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::expr_ast::{BinOp, ExprNode};
use tcl_compiler::ir::Statement;
use tcl_compiler::lowering::{lower_to_ir_with_config, lower_to_ir_with_dialect};
use tcl_registry::dialects::DialectSet;
use tcl_registry::registry_for_dialect;

const IRULES: &str = "f5-irules";
const TCL: &str = "tcl8.6";

/// A constant subject tested with an iRules word operator.
const CONSTANT_WORD_OP: &str = "when HTTP_REQUEST {\n    set x \"abcdef\"\n    if {$x contains \"cd\"} { HTTP::respond 200 }\n}\n";

/// The same shape with a non-constant right-hand side — nothing to fold.
const DYNAMIC_WORD_OP: &str = "when HTTP_REQUEST {\n    set y [HTTP::uri]\n    set x [HTTP::host]\n    if {$x contains $y} { HTTP::respond 200 }\n}\n";

/// Every analyser diagnostic code for `src` under `dialect`.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// The condition of the first `if` clause in the first `when` handler.
fn first_if_condition(module: &tcl_compiler::ir::Module) -> ExprNode {
    for proc in module.procedures.values() {
        for stmt in &proc.body.statements {
            if let Statement::If { clauses, .. } = stmt
                && let Some(first) = clauses.first()
            {
                return first.condition.clone();
            }
        }
    }
    panic!("no `if` statement lowered from the source");
}

// Lowering

/// Lowering under `f5-irules` parses the word operator as the operator it is;
/// the dialect-blind lowering degrades it to `ExprNode::Raw`, which is exactly
/// what stopped the fold.
#[test]
fn lowering_parses_a_word_operator_condition_under_the_irules_dialect() {
    let registry = registry_for_dialect(IRULES);
    let config = tcl_lexer::LexerConfig::for_dialect(IRULES);

    let with_dialect = lower_to_ir_with_dialect(
        CONSTANT_WORD_OP,
        registry,
        config,
        Some(tcl_dialect::DialectProfile::by_name(IRULES)),
    );
    assert!(
        matches!(
            first_if_condition(&with_dialect),
            ExprNode::Binary {
                op: BinOp::Contains,
                ..
            }
        ),
        "an iRules `contains` condition must lower to a Contains comparison"
    );

    let dialect_blind = lower_to_ir_with_config(CONSTANT_WORD_OP, registry, config);
    assert!(
        matches!(first_if_condition(&dialect_blind), ExprNode::Raw { .. }),
        "without the dialect the same condition is only an opaque Raw node"
    );
}

/// Plain Tcl must keep treating `contains` as a function-call word, not an
/// operator — the condition stays `Raw` there, which is what leaves W003 to
/// report the unavailable operator.
#[test]
fn lowering_leaves_a_word_operator_raw_in_plain_tcl() {
    let registry = registry_for_dialect(TCL);
    let module = lower_to_ir_with_dialect(
        "set x \"abcdef\"\nif {$x contains \"cd\"} { puts hit }\n",
        registry,
        tcl_lexer::LexerConfig::for_dialect(TCL),
        Some(tcl_dialect::DialectProfile::by_name(TCL)),
    );
    let condition = module
        .top_level
        .statements
        .iter()
        .find_map(|stmt| match stmt {
            Statement::If { clauses, .. } => clauses.first().map(|c| c.condition.clone()),
            _ => None,
        })
        .expect("an `if` statement");
    assert!(
        matches!(condition, ExprNode::Raw { .. }),
        "plain Tcl has no `contains` operator, so the condition must stay Raw"
    );
}

// The compilation unit

/// `build_for_dialect` folds the constant word-operator branch; the
/// dialect-blind `build_for` on the same source does not.
#[test]
fn build_for_dialect_folds_a_word_operator_branch() {
    let registry = registry_for_dialect(IRULES);
    let with_dialect =
        CompilationUnit::build_for_dialect(CONSTANT_WORD_OP, registry, false, IRULES);
    let folded: usize = with_dialect
        .functions()
        .map(|fu| fu.sccp.constant_branches.len())
        .sum();
    assert!(
        folded > 0,
        "the constant `contains` branch must fold under an iRules build"
    );

    let blind = CompilationUnit::build_for(CONSTANT_WORD_OP, registry, false);
    let blind_folded: usize = blind
        .functions()
        .map(|fu| fu.sccp.constant_branches.len())
        .sum();
    assert_eq!(
        blind_folded, 0,
        "a dialect-blind build has no word operator to fold"
    );
}

/// The resolved-profile entry point must carry the same iRules fold policy as
/// the string compatibility entry point.  This is the path used by CLI/LSP
/// callers after dialect detection and guards against a profile being reduced
/// to the plain fallback between registry construction and SCCP.
#[test]
fn build_for_profile_folds_a_word_operator_branch() {
    let registry = registry_for_dialect(IRULES);
    let profile = tcl_dialect::DialectProfile::by_name(IRULES);
    let unit = CompilationUnit::build_for_profile(CONSTANT_WORD_OP, registry, false, profile);
    assert!(
        unit.functions()
            .any(|function| !function.sccp.constant_branches.is_empty()),
        "a resolved f5-irules profile must retain the word-operator fold policy"
    );
}

/// `tk` is an additive command-surface bit rather than a catalogue profile.
/// The compatibility entry point must retain it in the unit instead of
/// canonicalising the input to the plain fallback profile's `tcl` name.
#[test]
fn build_for_dialect_retains_the_tk_set_only_bit() {
    let registry = registry_for_dialect("tk");
    let unit = CompilationUnit::build_for_dialect("button .b\n", registry, false, "tk");

    assert_eq!(unit.ir_module.dialect.as_deref(), Some("tk"));
    assert_eq!(unit.top_level.semantic_facts.dialect(), DialectSet::TK);
}

// I230 — TP / TN / FP / FN

/// TP: a constant iRules word-operator condition draws I230 under `f5-irules`.
#[test]
fn i230_fires_on_a_constant_word_operator_condition_under_irules() {
    assert!(
        fires(CONSTANT_WORD_OP, IRULES, "I230"),
        "codes: {:?}",
        codes(CONSTANT_WORD_OP, IRULES)
    );
}

/// TN: the same shape in plain Tcl draws no I230 — `contains` is not an
/// operator there, so there is no constant condition. W003 reports the
/// unavailable operator instead.
#[test]
fn i230_stays_silent_on_a_word_operator_condition_in_plain_tcl() {
    let src = "set x \"abcdef\"\nif {$x contains \"cd\"} { puts hit }\n";
    let found = codes(src, TCL);
    assert!(
        !found.iter().any(|c| c == "I230"),
        "plain Tcl must not fold a word operator: {found:?}"
    );
    assert!(
        found.iter().any(|c| c == "W003"),
        "plain Tcl reports the unavailable operator instead: {found:?}"
    );
}

/// FP guard: a word-operator condition on values the lattice cannot resolve
/// must not draw I230 under `f5-irules` either — the fold has to be proven,
/// not assumed from the operator.
#[test]
fn i230_stays_silent_on_a_non_constant_word_operator_condition() {
    let found = codes(DYNAMIC_WORD_OP, IRULES);
    assert!(
        !found.iter().any(|c| c == "I230"),
        "neither operand is constant, so nothing folds: {found:?}"
    );
}

/// FN guard: the pre-existing plain-Tcl constant condition still fires, so the
/// dialect threading did not narrow I230's ordinary reach.
#[test]
fn i230_still_fires_on_a_plain_tcl_constant_condition() {
    let src = "set x 1\nif {$x == 1} { puts hit }\n";
    assert!(fires(src, TCL, "I230"), "codes: {:?}", codes(src, TCL));
}
