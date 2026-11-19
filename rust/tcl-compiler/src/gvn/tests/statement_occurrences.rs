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

//! Purity, state writes and the occurrences one statement contributes, including embedded command substitutions.

use super::*;

// Statement-level helpers.

fn call_stmt(cmd: &str, args: &[&str]) -> Statement {
    Statement::Call {
        span: Span::new(0, 0),
        command: cmd.into(),
        canonical_command: None,
        args: args.iter().map(|s| (*s).into()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

#[test]
fn is_pure_command_picks_up_registry_pure_trait() {
    let registry = CommandRegistry::build_default();
    // `expr` is marked pure / PURE_EVALUATION in the registry.
    assert!(is_pure_command(
        &registry,
        "expr",
        &["{1 + 2}".into()],
        None
    ));
    // `set` is variable-assigning — not pure.
    assert!(!is_pure_command(
        &registry,
        "set",
        &["x".into(), "1".into()],
        None
    ));
}

#[test]
fn is_pure_command_recurses_into_impure_substitution() {
    // O106: `expr` is pure, but an arg that embeds
    // an impure `[incr c]` substitution makes the whole invocation
    // impure — its value depends on the per-call side effect, so it
    // must not be hoisted as loop-invariant.
    let registry = CommandRegistry::build_default();
    assert!(
        !is_pure_command(&registry, "expr", &["[incr c]".into()], None),
        "outer pure `expr` wrapping impure `[incr c]` must be impure",
    );
    // A pure nested substitution keeps the outer call pure.
    assert!(
        is_pure_command(&registry, "expr", &["[expr {1 + 1}]".into()], None),
        "outer pure `expr` wrapping a pure subst stays pure",
    );
    // Deeper nesting: the impurity is two levels down.
    assert!(
        !is_pure_command(&registry, "expr", &["[expr [incr c]]".into()], None),
        "nested impure subst at any depth must propagate",
    );
}

/// A pure command (`expr`) gates to non-pure once its
/// canonical name appears in `traced_commands`.
#[test]
fn is_pure_command_with_traces_traced_command_not_pure() {
    let registry = CommandRegistry::build_default();
    let mut traced = std::collections::BTreeSet::new();
    traced.insert("foo".to_string());
    // `foo` is unknown to the registry but in the trace set —
    // never pure.
    assert!(!is_pure_command_with_traces(
        &registry,
        "foo",
        &["arg".into()],
        None,
        &traced,
        false,
    ));
}

/// `has_dynamic_trace=true` widens the gate to *every*
/// command (even ones the registry marks pure).
#[test]
fn is_pure_command_with_traces_dynamic_trace_inhibits_all() {
    let registry = CommandRegistry::build_default();
    let traced = std::collections::BTreeSet::new();
    // `expr` is pure per the registry, but `has_dynamic_trace=true`
    // means any call could have a trace registered at runtime.
    assert!(!is_pure_command_with_traces(
        &registry,
        "expr",
        &["{1 + 2}".into()],
        None,
        &traced,
        true,
    ));
}

/// Regression — untraced pure commands stay pure.
#[test]
fn is_pure_command_with_traces_untraced_command_still_pure() {
    let registry = CommandRegistry::build_default();
    let traced = std::collections::BTreeSet::new();
    assert!(is_pure_command_with_traces(
        &registry,
        "expr",
        &["{1 + 2}".into()],
        None,
        &traced,
        false,
    ));
}

/// `::ns::foo` and `ns::foo` look up identically in the
/// trace set (mirrors `populate_trace_facts`'s `::`-strip).
#[test]
fn is_pure_command_with_traces_qualified_name_canonicalised() {
    let registry = CommandRegistry::build_default();
    let mut traced = std::collections::BTreeSet::new();
    traced.insert("ns::foo".to_string());
    assert!(!is_pure_command_with_traces(
        &registry,
        "::ns::foo",
        &[],
        None,
        &traced,
        false,
    ));
    assert!(!is_pure_command_with_traces(
        &registry,
        "ns::foo",
        &[],
        None,
        &traced,
        false,
    ));
}

#[test]
fn opaque_native_call_kills_common_value_numbering_without_text_classification() {
    let registry = CommandRegistry::build_default();
    let native = crate::ir::native_call_for_test(b"opaque \xff");
    assert!(statement_writes_state_for_gvn(
        &registry,
        &native,
        registry.profile(),
        GvnLegalitySource::Common(None),
    ));
    let mut ssa = SsaFunction::trivial("::f", BlockId(0), vec!["entry".into()]);
    let statement = ssa_stmt_for(&mut ssa, native, None);
    assert_eq!(
        statement_occurrences(&registry, &statement, "entry", 0, registry.profile(), &ssa),
        [] as [crate::gvn::ExprOccurrence; 0]
    );
}

#[test]
fn statement_writes_state_true_for_barrier() {
    let registry = CommandRegistry::build_default();
    let barrier = Statement::Barrier {
        span: Span::new(0, 0),
        reason: "eval".into(),
        command: "eval".into(),
        canonical_command: None,
        args: vec!["script".into()],
        tokens: None,
    };
    assert!(statement_writes_state(&registry, &barrier, None));
}

#[test]
fn statement_writes_state_true_for_global_write() {
    let registry = CommandRegistry::build_default();
    // Global-scope `set` writes GLOBAL_STATE — must invalidate.
    let set_global = call_stmt("set", &["::foo::bar", "1"]);
    assert!(statement_writes_state(&registry, &set_global, None));
}

#[test]
fn statement_writes_state_false_for_proc_local_set() {
    let registry = CommandRegistry::build_default();
    // Proc-local `set x 1` only writes the local — no region.
    let set_local = call_stmt("set", &["x", "1"]);
    assert!(!statement_writes_state(&registry, &set_local, None));
}

#[test]
fn statement_writes_state_false_for_pure_expr() {
    let registry = CommandRegistry::build_default();
    let expr_stmt = call_stmt("expr", &["{1 + 2}"]);
    assert!(!statement_writes_state(&registry, &expr_stmt, None));
}

#[test]
fn statement_occurrences_skips_impure_commands() {
    let registry = CommandRegistry::build_default();
    // `set` is impure — statement_occurrences emits nothing.
    let stmt_ssa = SsaStatement {
        statement: call_stmt("set", &["x", "1"]),
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let occurrences = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
    assert_eq!(occurrences, [] as [crate::gvn::ExprOccurrence; 0]);
}

#[test]
fn statement_occurrences_emits_nothing_for_non_call_stmts() {
    let registry = CommandRegistry::build_default();
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    assert_eq!(
        statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa()).len(),
        0
    );
}

#[test]
fn is_worth_reporting_unknown_command_is_false() {
    let registry = CommandRegistry::build_default();
    assert!(!is_worth_reporting(&registry, "__nonexistent"));
}

// Embedded command-substitution scanning.

#[test]
fn scan_bracketed_commands_simple() {
    let out = scan_bracketed_commands("[llength $x]", fixture_config("tcl9.1"));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "llength");
    assert_eq!(out[0].1, vec!["$x".to_string()]);
}

#[test]
fn scan_bracketed_commands_nested() {
    let out = scan_bracketed_commands("[set v [lindex $x 0]]", fixture_config("tcl9.1"));
    // Outer pair + inner pair.
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].0, "set");
    assert_eq!(out[1].0, "lindex");
}

#[test]
fn scan_bracketed_commands_skips_braced_regions() {
    let out = scan_bracketed_commands("{[not a command]}", fixture_config("tcl9.1"));
    assert_eq!(
        out,
        [] as [(std::string::String, std::vec::Vec<std::string::String>); 0]
    );
}

#[test]
fn scan_bracketed_commands_handles_backslash_escape() {
    // `\[` shouldn't start a region.
    let out = scan_bracketed_commands("hello \\[world]", fixture_config("tcl9.1"));
    assert_eq!(
        out,
        [] as [(std::string::String, std::vec::Vec<std::string::String>); 0]
    );
}

#[test]
fn bracketed_command_keeps_braced_and_quoted_words() {
    let commands = scan_bracketed_commands("[list {a b c} \"d e\" f]", fixture_config("tcl9.1"));
    let [(cmd, args)] = commands.as_slice() else {
        panic!("expected one command")
    };
    assert_eq!(cmd, "list");
    assert_eq!(args, &["{a b c}".to_string(), "\"d e\"".into(), "f".into()]);
}

#[test]
fn statement_occurrences_includes_embedded_pure_cmd() {
    let registry = CommandRegistry::build_default();
    // set x [llength $y] — an impure `set` with an embedded
    // pure `llength`. The top-level Call is skipped (impure),
    // but the embedded `llength` should be reported.
    let stmt_ssa = SsaStatement {
        statement: Statement::Call {
            span: Span::new(0, 0),
            command: "set".into(),
            canonical_command: None,
            args: vec!["x".into(), "[llength $y]".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        },
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].expression_text, "llength $y");
}

#[test]
fn statement_occurrences_includes_embedded_in_assign_value() {
    let registry = CommandRegistry::build_default();
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignValue {
            span: Span::new(0, 0),
            name: "len".into(),
            name_braced: false,
            value: "[llength $y]".into(),
            value_needs_backsubst: false,
            tokens: None,
        },
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let occ = statement_occurrences(&registry, &stmt_ssa, "entry", 0, None, &bare_ssa());
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].expression_text, "llength $y");
}

#[test]
fn original_token_purity_requires_actual_availability_and_complete_children() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "dict create key value",
        registry,
        false,
        "tcl8.6",
    );
    let tokens = cu
        .ir_module
        .top_level
        .statements
        .iter()
        .find_map(Statement::tokens)
        .unwrap();
    let traced = std::collections::BTreeSet::new();
    assert!(super::super::is_pure_tokens_with_metadata_context(
        registry,
        Some(current.into()),
        tokens,
        &traced,
        false
    ));
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for metadata in [Some((&older).into()), Some(foreign.into()), None] {
        assert!(!super::super::is_pure_tokens_with_metadata_context(
            registry, metadata, tokens, &traced, false
        ));
    }
    let replaced = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc dict args {return MUTATED}; list [dict create key value]",
        registry,
        false,
        "tcl8.6",
    );
    let tokens = replaced
        .ir_module
        .top_level
        .statements
        .last()
        .and_then(Statement::tokens)
        .unwrap();
    assert!(!super::super::is_pure_tokens_with_metadata_context(
        registry,
        Some(current.into()),
        tokens,
        &traced,
        false
    ));
}
