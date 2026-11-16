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

//! The scope tree itself: `current_scope`, `define_var`, `record_var_read`, the configured scanners and the scope walk.

use super::*;

#[test]
fn current_scope_defaults_to_global() {
    let a = Analyser::new();
    assert_eq!(a.current_scope().name, "::");
    assert_eq!(a.current_scope().kind, ScopeKind::Global);
}

#[test]
fn current_scope_follows_path() {
    let mut a = make_analyser_with_namespace("ns1");
    a.current_scope_path = vec![0];
    assert_eq!(a.current_scope().name, "ns1");
    assert_eq!(a.current_scope().kind, ScopeKind::Namespace);
}

#[test]
fn configured_scope_scanners_preserve_irules_brace_boundary_and_84_literals() {
    // Mutation proof for both recursive scope sites: the
    // configured segmenter sees the iRules `}{` boundary as a second
    // argument, while the 8.4 config keeps `{*}` literal.
    let source = "set out [expr {$x + [set y]}] cmd {a}{b}";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("f5-irules").unwrap();
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "f5-irules");
    let input = analysis.resolved_input.as_ref().unwrap();
    let config = input.lexer_config();
    let registry = input.borrowed_context_registry().commands();
    let cmd = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .into_iter()
        .next()
        .expect("fixture command");
    assert_eq!(
        cmd.args().len(),
        5,
        "configured iRules words: {:?}",
        cmd.texts
    );
    let mut reads = Vec::new();
    collect_script_command_reads(source, &analysis, &cmd, registry, config, &mut reads);
    assert!(reads.iter().any(|r| r.name == "x"));
    assert!(reads.iter().any(|r| r.name == "y"));

    let eight_four = tcl_lexer::LexerConfig::for_dialect("tcl8.4");
    let old = crate::segmenter::segment_commands_with_offset_and_config("cmd {*}$x", 0, eight_four);
    assert!(old[0].expand_word.is_none(), "8.4 must keep {{*}} literal");
    let first_close = "set x ${a{b}c}\n";
    let tokens =
        tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(first_close), eight_four)
            .tokenise_all()
            .expect("8.4 first-close vector lexes");
    let map = tcl_lexer::SourceMap::new(first_close);
    assert_eq!(
        tokens
            .iter()
            .find(|token| token.kind == tcl_lexer::TokenType::Var)
            .map(|token| map.token_text(*token)),
        Some("a{b"),
        "8.4 uses the first-close ${{...}} rule"
    );
}

#[test]
fn define_var_inserts_into_scope_and_all_variables() {
    let mut a = Analyser::new();
    let tok = Token::new(TokenType::Esc, span(0, 4));
    a.define_var("x", tok, &[], true, None);
    assert!(a.result.global_scope.variables.contains_key("x"));
    assert!(a.result.all_variables.contains_key("::::x"));
    let v = &a.result.global_scope.variables["x"];
    assert!(v.warn_if_unused);
    assert_eq!(v.definition_span, span(0, 4));
}

#[test]
fn define_var_existing_only_escalates_warn_flag() {
    let mut a = Analyser::new();
    let tok = Token::new(TokenType::Esc, span(0, 4));
    a.define_var("x", tok, &[], false, None);
    assert!(!a.result.global_scope.variables["x"].warn_if_unused);
    a.define_var("x", tok, &[], true, None);
    assert!(a.result.global_scope.variables["x"].warn_if_unused);
    // Definition span unchanged — first definition wins.
    assert_eq!(
        a.result.global_scope.variables["x"].definition_span,
        span(0, 4)
    );
}

#[test]
fn define_var_uses_explicit_definition_span() {
    let mut a = Analyser::new();
    let tok = Token::new(TokenType::Esc, span(0, 4));
    a.define_var("x", tok, &[], true, Some(span(10, 15)));
    assert_eq!(
        a.result.global_scope.variables["x"].definition_span,
        span(10, 15)
    );
}

#[test]
fn record_var_read_local_scope() {
    let mut a = Analyser::new();
    let tok = Token::new(TokenType::Esc, span(0, 4));
    a.define_var("x", tok, &[], true, None);
    a.record_var_read("x", span(20, 21), &[]);
    assert_eq!(
        a.result.global_scope.variables["x"].references,
        vec![span(20, 21)]
    );
}

#[test]
fn record_var_read_global_fallback_for_qualified_name() {
    // Var defined in global scope, read from a child namespace
    // via qualified name — should fall back to global.
    let mut a = make_analyser_with_namespace("ns1");
    let tok = Token::new(TokenType::Esc, span(0, 4));
    a.define_var("::x", tok, &[], true, None);
    a.record_var_read("::x", span(20, 21), &[0]);
    assert_eq!(
        a.result.global_scope.variables["::x"].references,
        vec![span(20, 21)]
    );
}

#[test]
fn record_var_read_no_match_silent() {
    let mut a = Analyser::new();
    // No var defined; read does nothing.
    a.record_var_read("x", span(20, 21), &[]);
    // No panic, no references recorded anywhere.
    assert!(a.result.global_scope.variables.is_empty());
}

#[test]
fn walk_scopes_from_root_visits_all() {
    let mut a = Analyser::new();
    let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
    ns.children.push(Scope::new(ScopeKind::Proc, "::ns1::p"));
    a.result.global_scope.children.push(ns);
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns2"));
    let paths = a.walk_scopes_from(&[]);
    assert_eq!(paths, vec![vec![], vec![0], vec![0, 0], vec![1]]);
}

/// Depth coverage: `walk_scopes_helper` recurses once per nested
/// [`Scope`] child, with no depth cap of its own. Transitively bounded to
/// `analyser::commands::MAX_BODY_DEPTH` (256) by the analyser pass
/// that builds the `Scope` tree today, so this is defence-in-depth /
/// consistency with every other full-tree walker in this crate, not a
/// currently-reproducible crash. 2000 levels is comfortably past this
/// new cap; the assertion is that `walk_scopes_from` returns at all,
/// not what it returns.
#[test]
fn deeply_nested_scopes_survive_walk_scopes_from() {
    const DEPTH: usize = 2000;
    let mut a = Analyser::new();
    let mut leaf = Scope::new(ScopeKind::Proc, "::leaf");
    for i in 0..DEPTH {
        let mut wrapper = Scope::new(ScopeKind::Namespace, format!("ns{i}"));
        wrapper.children.push(leaf);
        leaf = wrapper;
    }
    a.result.global_scope.children.push(leaf);
    let paths = a.walk_scopes_from(&[]);
    assert_ne!(paths, [] as [std::vec::Vec<usize>; 0]);
}
