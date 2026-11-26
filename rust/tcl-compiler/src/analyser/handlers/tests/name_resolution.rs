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

//! `resolve_proc_call` and `resolve_dynamic_word`.

use super::*;

// resolve_proc_call

#[test]
fn resolve_proc_call_absolute_qualified_name() {
    // ``::foo`` resolves directly when registered.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    let resolved = a.resolve_proc_call("::foo", &[]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::foo");
}

#[test]
fn resolve_proc_call_bare_name_walks_namespace_chain() {
    // ``foo`` declared inside ``ns1`` is found when resolved
    // from ``ns1``'s scope.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    // Resolve from inside ns1 — should find ::ns1::foo.
    let resolved = a.resolve_proc_call("foo", &[0]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::ns1::foo");
}

#[test]
fn resolve_proc_call_falls_back_to_global() {
    // Bare ``foo`` declared at global is found from any scope.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    // Resolve from inside ns1 — chain misses ::ns1::foo,
    // falls back to ::foo.
    let resolved = a.resolve_proc_call("foo", &[0]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::foo");
}

#[test]
fn resolve_proc_call_qualified_relative_name() {
    // ``a::b`` (qualified but not absolute) prepends ``::``.
    let mut a = Analyser::new();
    a.result.all_procs.insert(
        "::a::b".to_string(),
        super::super::ProcDef {
            source_name: None,
            name: "b".to_string(),
            qualified_name: "::a::b".to_string(),
            params: Vec::new(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span: span(0, 0),
            body_span: span(0, 0),
            doc: String::new(),
            param_traits: std::collections::HashMap::new(),
            caller_frame_params: std::collections::HashSet::new(),
            caller_frame_literals: std::collections::HashMap::new(),
            script_parameters: std::collections::HashMap::new(),
        },
    );
    let resolved = a.resolve_proc_call("a::b", &[]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::a::b");
}

#[test]
fn resolve_proc_call_relative_dotted_name_prefers_current_namespace() {
    // A relative dotted word (`ns2::inner`, containing `::` but not
    // starting with it) must resolve against the current namespace
    // first, not be rooted straight at global (confirmed against tclsh
    // 9.0.4 — see `bareword_resolution_candidates`).
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    for qname in ["::ns2::inner", "::ns::ns2::inner"] {
        a.result.all_procs.insert(
            qname.to_string(),
            super::super::ProcDef {
                source_name: None,
                name: "inner".to_string(),
                qualified_name: qname.to_string(),
                params: Vec::new(),
                params_computed: false,
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                name_span: span(0, 0),
                body_span: span(0, 0),
                doc: String::new(),
                param_traits: std::collections::HashMap::new(),
                caller_frame_params: std::collections::HashSet::new(),
                caller_frame_literals: std::collections::HashMap::new(),
                script_parameters: std::collections::HashMap::new(),
            },
        );
    }
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns"));
    let resolved = a.resolve_proc_call("ns2::inner", &[0]);
    assert_eq!(
        resolved.unwrap().qualified_name,
        "::ns::ns2::inner",
        "must prefer the current-namespace proc over the root one",
    );
    // Falls back to the root proc when there is no current-namespace
    // candidate.
    let resolved = a.resolve_proc_call("ns2::inner", &[]);
    assert_eq!(resolved.unwrap().qualified_name, "::ns2::inner");
}

#[test]
fn resolve_proc_call_does_not_walk_ancestor_namespaces() {
    // Real Tcl bareword resolution is exactly two levels — current
    // namespace, then global — absent an explicit `namespace path`. A
    // proc defined in a *grandparent* namespace must not resolve for a
    // bare call from a nested `::a::b::c` scope.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    let mut ns_a = Scope::new(ScopeKind::Namespace, "a");
    let mut ns_b = Scope::new(ScopeKind::Namespace, "b");
    ns_b.children.push(Scope::new(ScopeKind::Namespace, "c"));
    ns_a.children.push(ns_b);
    a.result.global_scope.children.push(ns_a);
    // scope_path [0] = ::a — define `foo` there.
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    // Resolving from ::a::b::c (scope_path [0, 0, 0]) must NOT reach
    // the grandparent's ::a::foo.
    assert!(
        a.resolve_proc_call("foo", &[0, 0, 0]).is_none(),
        "a grandparent-namespace proc must not resolve for a bare call",
    );
    // Control: resolving directly from ::a still finds it.
    assert!(a.resolve_proc_call("foo", &[0]).is_some());
}

#[test]
fn resolve_proc_call_unknown_name_returns_none() {
    let a = Analyser::new();
    assert!(a.resolve_proc_call("nope", &[]).is_none());
}

#[test]
fn resolve_proc_call_empty_name_returns_none() {
    let a = Analyser::new();
    assert!(a.resolve_proc_call("", &[]).is_none());
}

// resolve_dynamic_word

#[test]
fn resolve_dynamic_word_braced_literal_dollar_is_not_mistaken_for_dynamic() {
    // A brace-quoted word containing a literal `$`/`[` character is
    // legitimately STATIC — Tcl suppresses substitution inside
    // `{}` — so this must return it as-is via `resolve_const_word`'s
    // `Str`/`Esc` branch, even though `is_dynamic_word`'s naive text
    // scan alone would call it dynamic.
    let a = Analyser::new();
    let tok = Token {
        kind: TokenType::Str,
        span: span(0, 10),
        content_offset: 1,
        in_quote: false,
    };
    assert_eq!(
        a.resolve_dynamic_word("::pkg::$c", Some(tok), true, &[]),
        Some("::pkg::$c".to_string())
    );
}

#[test]
fn resolve_dynamic_word_missing_token_is_none() {
    // A dynamic word with no token to inspect (the handful of
    // existing handle_rename unit tests pass `arg_tokens: &[]`)
    // must fall through to None, not panic.
    let a = Analyser::new();
    assert_eq!(a.resolve_dynamic_word("$x", None, false, &[]), None);
}
