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

//! Reads in the words of a nested substitution: evaluated roles, data roles and body words.

use super::*;

/// The classified uses of every `Call` in a one-statement proc, lowered
/// from source so the statement carries the real word snapshot.
fn classified_call_uses(body: &str) -> Vec<(String, UseClass)> {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = format!("proc f {{tainted}} {{\n {body}\n}}");
    let cu = crate::compilation_unit::CompilationUnit::build_for(&src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let mut out = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            if matches!(stmt, Statement::Call { .. }) {
                out.extend(uses_of_classified(stmt, &mut scanner, &reg));
            }
        }
    }
    out
}

/// A braced word of a *nested* command is read exactly as the same word of
/// a direct call is, when the registry says the callee evaluates it in
/// this frame. Only a pair of braces separated `[expr $tainted]` (whose
/// read the enclosing word's own scan already saw) from
/// `[expr {$tainted}]`, which would otherwise be invisible to every
/// consumer of the use map.
#[test]
fn braced_word_of_a_nested_substitution_is_read_in_an_evaluated_role() {
    let uses = classified_call_uses("puts [expr {$tainted}]");
    assert!(
        uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
        "an `Expr` role's braced word substitutes in this frame: {uses:?}"
    );
}

#[test]
fn conditional_nested_expression_reads_keep_original_tokens_without_entry() {
    let registry = default_registry();
    let source = "proc f {tainted} {puts [expr {$tainted + 1}]}";
    let unit = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
    let function = unit.function("::f").unwrap();
    let child = function
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(Statement::tokens)
        .flat_map(|tokens| {
            crate::word_subst::lifted_calls(Some(tokens), unit.ir_module.lexer_config)
        })
        .find(|call| call.command == "expr")
        .and_then(|call| call.tokens)
        .expect("original nested expression");
    let binding = child.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.runtime_reachability(),
        crate::command_binding::SourceRuntimeReachability::Conditional,
    );
    assert!(binding.proved_execution_target().is_none());
    let scanner = VarReferenceScanner::new(VarScanOptions::default());
    let mut reads = BTreeSet::new();
    scan_selected_expression_roles(
        &child,
        &scanner,
        SsaInvocationContext::standalone(&registry),
        &mut reads,
    );
    assert_eq!(reads, BTreeSet::from(["tainted".to_owned()]));

    let mut foreign = child.clone();
    foreign.argv_texts[1] = "$other + 1".to_owned();
    foreign.word_exprs[1] = WordExpr::BracedLiteral {
        text: "$other + 1".to_owned(),
        source: foreign.word_exprs[1].source().clone(),
    };
    reads.clear();
    scan_selected_expression_roles(
        &foreign,
        &scanner,
        SsaInvocationContext::standalone(&registry),
        &mut reads,
    );
    assert!(
        reads.is_empty(),
        "foreign source cannot borrow May-read topology"
    );
}

/// …and a braced word in a role the callee does *not* evaluate stays the
/// literal it is. `list` builds a value; `{$tainted}` is one of its
/// elements, not a script.
#[test]
fn braced_word_of_a_nested_substitution_stays_literal_in_a_data_role() {
    let uses = classified_call_uses("puts [list {$tainted}]");
    assert!(
        !uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
        "`list` does not evaluate its arguments in the calling frame: {uses:?}"
    );
}

/// A nested command's **body** must not be read as a frame use, even
/// though a body written on a *statement* is. The difference is the
/// lowering: a statement's body becomes its own CFG block, so `lmap`'s
/// loop variable is defined where the body's reads are seen, while a
/// nested substitution gets no block at all — so its body's reads arrive
/// with none of its own definitions and `x` looks read-before-set.
#[test]
fn body_word_of_a_nested_substitution_is_not_a_frame_read() {
    let uses = classified_call_uses("puts [join [lmap x $tainted {string toupper $x}] ,]");
    assert!(
        !uses.iter().any(|(name, _)| name == "x"),
        "`lmap` binds its own loop variable; the frame never reads it: {uses:?}"
    );
}

/// A script a substitution runs in this frame — `catch`'s — shares the
/// frame's names, so `inner` is the frame's: its write is the effect
/// call's. The walk cannot order the script's read of it after the
/// script's own write, so the effect call reads it by name beside that
/// write, and nothing the host substitutes names it.
#[test]
fn a_protected_scripts_local_is_read_by_name_beside_its_write() {
    let uses = classified_call_uses("puts [catch {set inner 1; expr {$inner + 1}}]");
    let inner: Vec<_> = uses.iter().filter(|(name, _)| name == "inner").collect();
    assert_eq!(inner, [&("inner".to_owned(), UseClass::Name)], "{uses:?}");
}

/// The classified uses of every `AssignValue` in a one-statement proc —
/// the assignment twin of [`classified_call_uses`].
fn classified_assign_uses(body: &str) -> Vec<(String, UseClass)> {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = format!("proc f {{tainted}} {{\n {body}\n}}");
    let cu = crate::compilation_unit::CompilationUnit::build_for(&src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let mut out = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            if matches!(stmt, Statement::AssignValue { .. }) {
                out.extend(uses_of_classified(stmt, &mut scanner, &reg));
            }
        }
    }
    out
}

/// An assignment's value word substitutes by exactly the rules a call's
/// word does, so the braced body of a nested `[expr …]`
/// is a read there too. Without it the expression's operands are absent
/// from `uses` and every consumer of that map — the expr shimmer detector
/// included — is blind to `set r [list [expr {$tainted + 1}]]`.
#[test]
fn braced_word_of_a_nested_substitution_is_read_in_an_assignment_value() {
    let uses = classified_assign_uses("set r [list [expr {$tainted + 1}]]");
    assert!(
        uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
        "an `Expr` role's braced word substitutes in this frame: {uses:?}"
    );
}

/// …and the body exclusion holds on the assignment path too — this is the
/// spelling the repository's own corpus actually contains.
#[test]
fn body_word_of_a_nested_substitution_in_an_assignment_is_not_a_frame_read() {
    let uses = classified_assign_uses("set r [join [lmap x $tainted {string toupper $x}] ,]");
    assert!(
        !uses.iter().any(|(name, _)| name == "x"),
        "`lmap` binds its own loop variable; the frame never reads it: {uses:?}"
    );
}
