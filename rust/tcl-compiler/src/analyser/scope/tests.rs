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

use super::*;
use tcl_lexer::TokenType;

fn span(start: u32, end: u32) -> Span {
    Span::new(start, end)
}

fn make_analyser_with_namespace(ns_name: &str) -> Analyser {
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, ns_name));
    a
}

fn logical_value_token(analyser: &Analyser, written: &str) -> Token {
    let start = analyser.source.rfind(written).expect("retained source use");
    let mut token = tcl_lexer::Lexer::with_config(
        &analyser.source[start..],
        analyser.file_lexer_config().nested(),
    )
    .tokenise_all()
    .expect("valid retained source")
    .into_iter()
    .next()
    .unwrap();
    token.span = Span::new(
        token.span.start() + u32::try_from(start).unwrap(),
        token.span.end() + u32::try_from(start).unwrap(),
    );
    token
}

#[test]
fn logical_cross_namespace_candidates_use_actual_namespace_owners() {
    fn erase_labels(scope: &mut Scope) {
        scope.name = "unrelated report".to_owned();
        for child in &mut scope.children {
            erase_labels(child);
        }
    }

    let source =
        "namespace eval N {set {a{b}c} ONE; namespace eval Nested {set value TWO}}\nputs $";
    let mut analysis = Analyser::new().analyse(source, "tcl");

    erase_labels(&mut analysis.global_scope);
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    let advice = analysis
        .logical_variable_scope_advice(&image, config, u32::try_from(source.len()).unwrap())
        .unwrap();
    assert_eq!(
        advice.cross_namespace_variable_names(),
        vec!["::N::Nested::value", "::N::a{b}c"]
    );
    assert!(
        analysis
            .logical_variable_scope_advice(
                &tcl_lexer::SourceImage::document(&format!("#{source}")),
                config,
                0
            )
            .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        missing
            .logical_variable_scope_advice(&image, config, 0)
            .is_none()
    );
    let mut unavailable = analysis.clone();
    unavailable.global_scope.children[0].naming_scope = None;
    let advice = unavailable
        .logical_variable_scope_advice(&image, config, u32::try_from(source.len()).unwrap())
        .unwrap();
    assert_eq!(
        advice.cross_namespace_variable_names(),
        [] as [std::string::String; 0]
    );
}

#[test]
fn logical_write_card_correspondence_uses_original_word_without_native_recipe() {
    let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let span_start = u32::try_from(source.find("set x AGAIN").unwrap() + "set ".len()).unwrap();
    let advice = analyser
        .result
        .original_variable_write_advice
        .iter()
        .find(|advice| advice.span().start() == span_start)
        .expect("actual Registry-selected Logical write card");
    let span = advice.span();
    let input = advice.original_name_input().clone();
    let image = tcl_lexer::SourceImage::document(source);
    let config = analyser.file_lexer_config();
    assert!(
        analyser
            .result
            .logical_original_word_input_matches_source(&image, config, span, &input)
    );
    let changed = tcl_lexer::SourceImage::document(&source.replace("AGAIN", "OTHER"));
    assert!(
        !analyser
            .result
            .logical_original_word_input_matches_source(&changed, config, span, &input)
    );
    let mut wrong = config;
    wrong.strict_quoting = !wrong.strict_quoting;
    assert!(
        !analyser
            .result
            .logical_original_word_input_matches_source(&image, wrong, span, &input)
    );
    let shifted = Span::new(span.start() + 1, span.end() + 1);
    assert!(
        !analyser
            .result
            .logical_original_word_input_matches_source(&image, config, shifted, &input)
    );
    let mut absent = analyser.result.clone();
    absent.resolved_input = None;
    assert!(!absent.logical_original_word_input_matches_source(&image, config, span, &input));
    let retained = analyser.result.resolved_input.as_ref().unwrap();
    let foreign = std::sync::Arc::new(retained.borrowed_context_registry().with_command_store(
        std::sync::Arc::new(tcl_registry::CommandRegistry::build_default()),
    ));
    let mut changed_store = analyser.result.clone();
    changed_store.resolved_input = Some(super::super::ResolvedAnalysisInput::new(
        retained.analyser_profile(),
        retained.unit_profile(),
        foreign,
        retained.lexer_config(),
    ));
    assert!(
        !changed_store.logical_original_word_input_matches_source(&image, config, span, &input)
    );
    let mut native = Analyser::new();
    native.analyse_and_retain_result_for_test(source, "tcl8.6");
    assert!(!native.result.logical_original_word_input_matches_source(
        &image,
        native.file_lexer_config(),
        span,
        &input
    ));
}

#[test]
fn logical_const_value_uses_variable_frame_not_command_namespace() {
    let source = "set x ROOT\nnamespace eval N {set x NAMESPACE; proc p {} {set x LOCAL; uplevel 0 {puts $x}; uplevel #0 {puts $x}; uplevel 1 {puts $x}}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    for (context, expected) in [
        ("uplevel 0", Some("LOCAL")),
        ("uplevel #0", Some("ROOT")),
        ("uplevel 1", None),
    ] {
        let offset = source.find(context).unwrap();
        let start = source[offset..].find("$x").unwrap() + offset;
        let token = Token::with_content_offset(
            TokenType::Var,
            Span::new(
                u32::try_from(start).unwrap(),
                u32::try_from(start + 2).unwrap(),
            ),
            1,
        );
        assert_eq!(
            analyser
                .lookup_logical_const_substitution(token)
                .map(|(value, _)| value),
            expected,
            "{context}"
        );
    }
}

#[test]
fn logical_const_value_preserves_relative_qualified_namespace() {
    let source = "namespace eval N {namespace eval child {set x VALUE}; puts $child::x}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$child::x");
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("VALUE")
    );
}

#[test]
fn logical_const_value_requires_original_source_context_and_snapshot() {
    let source = "set x VALUE; puts $x";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$x");
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("VALUE")
    );
    let snapshot = analyser.snapshot();
    let input = analyser.result.resolved_input.as_ref().unwrap().clone();
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    analyser.result.resolved_input = Some(super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    ));
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    let foreign_store = std::sync::Arc::new(tcl_registry::CommandRegistry::build_default());
    let foreign = std::sync::Arc::new(
        input
            .borrowed_context_registry()
            .with_command_store(foreign_store),
    );
    analyser.result.resolved_input = Some(super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        foreign,
        input.lexer_config(),
    ));
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    analyser.result.resolved_input = Some(input);
    analyser.source = source.replace("VALUE", "OTHER").into();
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    analyser.source = source.to_owned().into();
    analyser.result.resolved_input = None;
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    analyser.restore(snapshot);
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("VALUE")
    );
    analyser
        .const_strings
        .get_mut(&Vec::new())
        .unwrap()
        .get_mut("x")
        .unwrap()
        .0 = "OTHER".to_owned();
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn logical_const_value_does_not_resolve_array_receiver_from_literal_element() {
    let source = "set which live; set {targets($which)} FALSE; set targets(live) CORRECT; puts $targets($which)";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$targets($which)");
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    let source = "set {scalar(open} SCALAR; puts ${scalar(open}";
    let mut scalar = Analyser::new();
    scalar.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&scalar, "${scalar(open}");
    assert_eq!(
        scalar
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("SCALAR")
    );
}

#[test]
fn logical_const_value_does_not_borrow_native_or_linked_cell_provenance() {
    let source = "set x ROOT; proc p {} {global x; puts $x}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$x");
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    let source = "set x VALUE; puts $x";
    let mut native = Analyser::new();
    native.analyse_and_retain_result_for_test(source, "tcl8.6");
    let token = logical_value_token(&native, "$x");
    assert!(!native.result.allows_lexical_declaration_advice());
    assert!(native.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn logical_const_value_does_not_guess_an_object_method_namespace() {
    let source = "namespace eval lexical {namespace eval lexical {set x DEFINING}; oo::class create C {method read {} {puts $lexical::x}}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$lexical::x");
    let image = tcl_lexer::SourceImage::document(source);
    let advice = analyser
        .result
        .logical_variable_scope_advice(&image, analyser.file_lexer_config(), token.span.start())
        .expect("actual method source frame");
    assert_eq!(
        advice.eligible_scopes().last().unwrap().kind,
        ScopeKind::Method
    );
    assert!(advice.namespace_variable_coordinate("lexical::x").is_none());
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn logical_const_value_refuses_non_dominating_and_future_writes() {
    for source in [
        "if {$flag} {set x BRANCH}; puts $x",
        "puts $x; set x FUTURE",
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        let token = logical_value_token(&analyser, "$x");
        assert!(
            analyser.lookup_logical_const_substitution(token).is_none(),
            "{source}"
        );
    }
}

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
fn set_const_string_retains_the_value() {
    let mut a = Analyser::new();
    a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("hello")
    );
}

#[test]
fn const_string_tables_keep_independent_values() {
    let mut a = make_analyser_with_namespace("ns1");
    a.set_const_string("x", "outer".to_string(), span(0, 5), &[]);
    a.set_const_string("x", "inner".to_string(), span(10, 15), &[0]);
    assert_eq!(
        a.const_strings
            .get(&vec![0])
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("inner")
    );
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("outer")
    );
}

#[test]
fn clear_const_string_removes_entry() {
    let mut a = Analyser::new();
    a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
    a.clear_const_string("x", &[]);
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

#[test]
fn const_string_table_starts_empty() {
    let a = Analyser::new();
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

#[test]
fn command_resolution_namespace_global_returns_root() {
    let a = Analyser::new();
    assert_eq!(a.command_resolution_namespace(&[]), "::");
}

#[test]
fn command_resolution_namespace_single_relative() {
    let a = make_analyser_with_namespace("ns1");
    assert_eq!(a.command_resolution_namespace(&[0]), "::ns1");
}

#[test]
fn command_resolution_namespace_absolute_rebases() {
    let mut a = make_analyser_with_namespace("outer");
    // Add an absolute child namespace.
    a.result.global_scope.children[0]
        .children
        .push(Scope::new(ScopeKind::Namespace, "::abs"));
    // Path [0, 0] should rebase at ::abs, not nest under outer.
    assert_eq!(a.command_resolution_namespace(&[0, 0]), "::abs");
}

/// A proc scope resolves in its own **defining** namespace, so a
/// qualified-name proc declared at the top level still answers `::ns1`
/// (a purely lexical walk would answer `::`).
#[test]
fn command_resolution_namespace_uses_a_procs_defining_namespace() {
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Proc, "::ns1::myproc"));
    assert_eq!(a.command_resolution_namespace(&[0]), "::ns1");
}

#[test]
fn command_resolution_namespace_nested_proc_keeps_the_enclosing_namespace() {
    let mut a = Analyser::new();
    let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
    ns.children
        .push(Scope::new(ScopeKind::Proc, "::ns1::myproc"));
    a.result.global_scope.children.push(ns);
    assert_eq!(a.command_resolution_namespace(&[0, 0]), "::ns1");
}

/// The Method-scope namespace rule: a `TclOO` method (flagged) resolves
/// globally; a snit/itcl method (unflagged) resolves in the class's
/// defining namespace, like a proc.
#[test]
fn method_scope_namespace_honours_oo_global_resolution() {
    let flagged = {
        let mut m = Scope::new(ScopeKind::Method, "::pkg::C::m");
        m.oo_global_resolution = true;
        m
    };
    assert_eq!(
        advance_command_resolution_namespace("::pkg", &flagged),
        "::"
    );
    let unflagged = Scope::new(ScopeKind::Method, "::pkg::C::m");
    assert_eq!(
        advance_command_resolution_namespace("::pkg", &unflagged),
        "::pkg::C",
    );
}

/// tclsh 8.6.16 / 9.0.4-pinned (G1 shape): a bare call in a `TclOO`
/// method body dispatches the GLOBAL helper, not the class's
/// defining-namespace helper — method bodies run in the object's
/// namespace and never search the class's namespace.
#[test]
fn tcloo_method_call_settles_to_the_global_helper() {
    let src = "namespace eval ::pkg {\n\
                       proc helper {} { return PKGHELPER }\n\
                       oo::class create C { method m {} { helper } }\n\
                   }\n\
                   proc ::helper {} { return GLOBALHELPER }\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl8.6");
    let inv = analysis
        .command_invocations
        .iter()
        .find(|i| i.name == "helper")
        .expect("method-body helper invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::helper"),
        "TclOO method bodies resolve bare calls globally (tclsh-pinned)",
    );
}

/// A builtin renamed away is not callable under its original name
/// (tclsh-pinned: C raises `invalid command name`), so settlement must
/// not demote a namespace-local guess to the dead builtin name.
#[test]
fn renamed_away_builtin_no_longer_settles_calls() {
    let src = "rename puts ::a::p\n\
                   namespace eval ::ns { puts x }\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl8.6");
    // `rename`'s own OLD argument (`puts`, line 0) is now itself a
    // recorded, correctly-resolved-to-the-still-live-builtin reference
    // — an earlier "puts" entry this test must not
    // mistake for the *call* site (`puts x`, line 1) it actually means
    // to check. The call site is always the *last* "puts"-named
    // invocation in source order.
    let inv = analysis
        .command_invocations
        .iter()
        .rev()
        .find(|i| i.name == "puts")
        .expect("namespaced puts invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::ns::puts"),
        "the dead builtin name must not win; the local-first guess stays",
    );
}

fn diag_codes(source: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(source, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

// finalise_invocation_resolutions — Tcl's existence-checked two-step
// resolution rule, applied post-walk.  Every case below is pinned
// against tclsh 8.6: the local-first candidate wins
// only when that command exists by the end of the file; otherwise the
// call falls back to the global candidate.

/// The `resolved_qualified_name` recorded for the invocation whose raw
/// call text is `name`.
fn resolved_for(source: &str, name: &str) -> Option<String> {
    let mut a = Analyser::new();
    let analysis = a.analyse(source, "tcl8.6");
    analysis
        .command_invocations
        .iter()
        .find(|inv| inv.name == name)
        .unwrap_or_else(|| panic!("no `{name}` invocation recorded"))
        .resolved_qualified_name
        .clone()
}

#[test]
fn relative_qualified_call_falls_back_to_global_when_local_absent() {
    // tclsh8.6: `inner::p` inside `outer` dispatches ::inner::p when
    // ::outer::inner::p does not exist.
    let src = "namespace eval ::inner {}\nproc ::inner::p {} {}\nnamespace eval outer { proc caller {} { inner::p } }\n";
    assert_eq!(resolved_for(src, "inner::p").as_deref(), Some("::inner::p"));
}

#[test]
fn relative_qualified_call_prefers_local_defined_later_in_file() {
    // tclsh8.6: a proc body's calls resolve at call time, so the local
    // candidate wins even when its `proc` runs after the caller was
    // defined — the walk order must not leak into the resolution.
    let src = concat!(
        "namespace eval ::inner {}\n",
        "proc ::inner::p {} {}\n",
        "namespace eval outer { proc caller {} { inner::p } }\n",
        "namespace eval outer { namespace eval inner {} ; proc inner::p {} {} }\n",
    );
    assert_eq!(
        resolved_for(src, "inner::p").as_deref(),
        Some("::outer::inner::p")
    );
}

#[test]
fn bare_call_falls_back_to_global_when_local_absent() {
    let src = "proc ::g {} {}\nnamespace eval ns2 { proc caller {} { g } }\n";
    assert_eq!(resolved_for(src, "g").as_deref(), Some("::g"));
}

#[test]
fn bare_call_prefers_local_defined_later_in_file() {
    let src = concat!(
        "proc ::g {} {}\n",
        "namespace eval ns2 { proc caller {} { g } }\n",
        "namespace eval ns2 { proc g {} {} }\n",
    );
    assert_eq!(resolved_for(src, "g").as_deref(), Some("::ns2::g"));
}

#[test]
fn builtin_call_inside_namespace_resolves_global() {
    // tclsh8.6: `puts` inside a namespace is the global builtin — the
    // local-first guess `::ns3::puts` names nothing, so the settled
    // resolution is `::puts` (as it was before the scope-aware walk).
    let src = "namespace eval ns3 { proc caller {} { puts hi } }\n";
    assert_eq!(resolved_for(src, "puts").as_deref(), Some("::puts"));
}

#[test]
fn builtin_shadowed_by_local_proc_stays_local() {
    // tclsh8.6: a namespace-local `puts` shadows the builtin for
    // unqualified calls from inside that namespace.
    let src = "namespace eval ns3 { proc puts {msg} {} ; proc caller {} { puts hi } }\n";
    assert_eq!(resolved_for(src, "puts").as_deref(), Some("::ns3::puts"));
}

#[test]
fn unknown_in_both_namespaces_keeps_local_first_guess() {
    // Neither ::ns4::mystery::call nor ::mystery::call is defined in
    // this file (a cross-file proc, or a typo W123 reports) — keep the
    // local-first candidate so cross-file consumers still see the
    // Tcl-priority guess.
    let src = "namespace eval ns4 { proc caller {} { mystery::call } }\n";
    assert_eq!(
        resolved_for(src, "mystery::call").as_deref(),
        Some("::ns4::mystery::call")
    );
}

#[test]
fn w215_fires_for_unreachable_quoted_name() {
    assert!(diag_codes("set \"a}b\" 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_braced_escaped_name() {
    // `{a\}b}` is braced, so `\}` is preserved verbatim and the
    // runtime name `a\}b` IS brace-substitutable.
    assert!(!diag_codes("set {a\\}b} 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_fires_for_array_index_with_paren() {
    assert!(diag_codes("set arr(a)b) 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_normal_name() {
    assert!(!diag_codes("set normal 1", "tcl").contains(&"W215".to_string()));
}

// A namespace-qualified `set` target with
// a dynamic trailing segment (`set ns::$k 1`) must not fire W215 —
// `word_piece`'s reconstruction re-braces an unbraced `$k` piece to
// `${k}` when rendering the word's display text (so an adjacent literal
// suffix can't run into it), and that reconstruction — not the source,
// not the runtime name — was what the reachability check inspected.
// Confirmed against tclsh 8.6.14: `set k client_addr; set
// ::ns::$k hello` writes the perfectly ordinary, `$`-reachable variable
// `::ns::client_addr`; nothing about it is unreachable.

#[test]
fn w215_quiet_for_namespace_qualified_dynamic_suffix() {
    // FP: the exact corpus shape (`rust/tcl-irule-test/tcl/runner.tcl`,
    // `foreach {k v} $values { set ::state::connection::$k $v }`).
    assert!(!diag_codes("set ::state::connection::$k $v", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_dynamic_suffix_without_namespace() {
    // FP: the bug is not namespace-specific — any unbraced word with an
    // embedded, non-leading `$var` piece is affected.
    assert!(!diag_codes("set prefix_$k 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_dynamic_suffix_with_command_substitution() {
    // FP: `[cmd]` pieces hit the same reconstruction path as `$var`.
    assert!(!diag_codes("set prefix_[cmd] 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_still_fires_for_static_typo_after_the_fix() {
    // TN: the reconstruction path must not blunt the check for the genuine
    // case it exists for — a fully static (no `$`/`[`) stray-delimiter typo.
    assert!(diag_codes("set \"a}b\" 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_suggests_close_in_scope_variable() {
    // TP: `"a}b"` is a stray-brace typo of the already-defined `ab`
    // (edit distance 1) — the message names it.
    let mut a = Analyser::new();
    let r = a.analyse("set ab 1\nset \"a}b\" 2\n", "tcl");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W215)
        .expect("W215");
    assert!(
        d.message.ends_with("; did you mean 'ab'?"),
        "expected an 'ab' suggestion: {:?}",
        d.message
    );
}

#[test]
fn w215_no_suggestion_when_nothing_is_close() {
    // FP guard: no defined variable sits within the edit budget —
    // the message keeps its plain form.
    let mut a = Analyser::new();
    let r = a.analyse("set totally 1\nset \"qq}zz\" 2\n", "tcl");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W215)
        .expect("W215");
    assert!(
        !d.message.contains("did you mean"),
        "no suggestion expected: {:?}",
        d.message
    );
}

#[test]
fn w215_quiet_for_backslash_continued_param_list() {
    // A parameter list wrapped across lines with `\`. Tcl
    // list-parses the braced param list, so `ddrtol\<newline>ddatol` is two
    // parameters (`ddrtol`, `ddatol`), not a single `ddrtol\` name — no
    // W215 unreachable-name warning should fire.
    let src = "proc p {a ddrtol\\\n        ddatol} { list $a $ddrtol $ddatol }";
    assert!(!diag_codes(src, "tcl").contains(&"W215".to_string()));

    // A TclOO `method` with a wrapped parameter list.
    let method_src = "oo::class create C {\n  method Fdjac2 {funct ifree ddrtol\\\n      ddatol} { list $ddrtol $ddatol }\n}\n";
    assert!(!diag_codes(method_src, "tcl").contains(&"W215".to_string()));
}

#[test]
fn w116_fires_for_stub_shadowing_builtin() {
    let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub set {a:var b}\n# tcl-lsp: stubs-end\n";
    assert!(diag_codes(src, "tcl").contains(&"W116".to_string()));
}

#[test]
fn w117_fires_for_stub_expr_shadowing_builtin() {
    let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub expr-func sin 1\n# tcl-lsp: stubs-end\n";
    assert!(diag_codes(src, "tcl").contains(&"W117".to_string()));
}

#[test]
fn sidecar_stub_shadows_never_point_into_the_source_document() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-sidecar-shadow-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("tcl.tcl.stubs"),
        "stub set {value}\nstub expr-func sin 1\n",
    )
    .unwrap();
    let source = root.join("main.tcl");
    let result = Analyser::new()
        .with_file_path(Some(source.display().to_string()))
        .analyse("set value 1\n", "tcl");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W116 | DiagCode::W117)),
        "sidecar shadow diagnostics must not be published on source spans: {:?}",
        result.diagnostics
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn w127_fires_for_value_outside_closed_set() {
    let src = "when HTTP_REQUEST { HTTP::version \"2.0\" }";
    assert!(diag_codes(src, "f5-irules").contains(&"W127".to_string()));
}

#[test]
fn w127_quiet_for_allowed_value() {
    let src = "when HTTP_REQUEST { HTTP::version \"1.1\" }";
    assert!(!diag_codes(src, "f5-irules").contains(&"W127".to_string()));
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

fn var(name: &str, def_span: Span) -> VarDef {
    VarDef {
        name: name.to_string(),
        definition_span: def_span,
        references: Vec::new(),
        warn_if_unused: false,
        array_indices: std::collections::BTreeSet::new(),
        link_target: None,
        link_target_span: None,
    }
}

/// TP — a namespace-qualified occurrence is
/// recorded with the `::`-rooted cell it names, whether it is written
/// absolutely or relative to the enclosing namespace, so the workspace
/// index can match it against a declaration in another document.
#[test]
fn qualified_var_occurrences_are_recorded_with_their_rooted_cell() {
    let src = "namespace eval app::colors {\n    variable palette red\n}\n\
                   puts $app::colors::palette\n\
                   puts $::app::colors::palette\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let cells: Vec<&str> = r
        .qualified_var_refs
        .iter()
        .map(|v| v.qualified_name.as_str())
        .collect();
    assert_eq!(
        cells,
        vec!["::app::colors::palette", "::app::colors::palette"],
        "both spellings name one cell",
    );
    for reference in &r.qualified_var_refs {
        assert!(matches!(
            reference.original_name_input,
            Some(crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(_)),
        ));
        assert!(reference.original_namespace.is_some());
    }
}

#[test]
// Implementation contract: naming.variable.byte-cell-correspondence
// docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
fn qualified_variable_inputs_keep_document_units_and_producer_kind() {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "namespace eval N {}; set {::N::v\0tail} 1; puts ${::N::v\0tail}";
        let result = Analyser::new().analyse(source, dialect);
        let reference = result
            .qualified_var_refs
            .iter()
            .find(|reference| {
                matches!(
                    reference.original_name_input,
                    Some(SignatureSourceNameInput::OriginalVariableRoot(_))
                )
            })
            .expect("original substitution root is retained");
        let Some(SignatureSourceNameInput::OriginalVariableRoot(root)) =
            &reference.original_name_input
        else {
            unreachable!()
        };
        let expected = if dialect == "jimtcl" {
            b"::N::v\0tail".as_slice()
        } else {
            b"::N::v\xc0\x80tail".as_slice()
        };
        assert_eq!(
            root.bytes(),
            expected,
            "{dialect}: original Document channel"
        );
        assert_eq!(
            root.source_image().channel(),
            tcl_lexer::SourceChannel::Document
        );
        assert!(root.original_word().is_none());
        assert!(reference.original_namespace.is_some());
        assert!(
            result.qualified_var_refs.iter().any(|reference| {
                reference.original_name_input.as_ref().is_some_and(|input| {
                    !matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_))
                        && input.bytes() == expected
                })
            }),
            "{dialect}: name operand keeps its independently retained value producer"
        );
    }
}

#[test]
fn original_static_variable_words_survive_unavailable_execution_without_new_authority() {
    let source = r"unknown; set {v\uD800} 1; set v\uD801 2";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let mut analyser = Analyser::new();
        let _ = analyser.analyse(source, profile);
        let first = u32::try_from(source.find(r"{v\uD800}").unwrap()).unwrap();
        let second = u32::try_from(source.find(r"v\uD801").unwrap()).unwrap();
        let first = analyser
            .original_static_source_name_at_span(Span::new(first, first + 9))
            .unwrap();
        assert_eq!(first.name_input().bytes(), br"v\uD800");
        let second = analyser
            .original_static_source_name_at_span(Span::new(second, second + 7))
            .unwrap();
        assert_eq!(second.name_input().bytes(), b"v\xed\xa0\x81");
        assert_eq!(
            second.name_input().source_image(),
            &tcl_lexer::SourceImage::document(source)
        );
        let previous = second.clone();
        let _ = analyser.analyse("set $computed 1", profile);
        assert!(
            analyser
                .original_static_source_name_at_span(previous.name_input().original_word().span())
                .is_none()
        );
    }
}

#[test]
fn original_variable_cursor_roots_keep_lexical_membership_and_full_source_currency() {
    let source =
        "set ns ::N; namespace exists $ns; namespace exists $missing; puts {${inert}}; # $comment";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        for (needle, expected) in [("$ns", b"ns".as_slice()), ("$missing", b"missing")] {
            let offset = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
            let root = analysis
                .original_variable_root_in_source(&image, config, offset)
                .unwrap_or_else(|| panic!("{profile}: actual lexical {needle}"));
            assert_eq!(root.bytes(), expected);
            assert_eq!(root.source_image(), &image);
            assert!(
                analysis
                    .original_variable_root_in_source(
                        &tcl_lexer::SourceImage::document(&source.replace("::N", "::M")),
                        config,
                        offset,
                    )
                    .is_none()
            );
        }
        for needle in ["${inert}", "$comment"] {
            let offset = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
            assert!(
                analysis
                    .original_variable_root_in_source(&image, config, offset)
                    .is_none()
            );
        }
    }
}

#[test]
fn original_conditional_variable_receivers_keep_absolute_source_geometry_and_unknown_coverage() {
    // Implementation contract: naming.variable.conditional-registry-receiver-geometry
    // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
    let source = r"namespace eval n\uD800 {}
namespace exists n\uD800
namespace eval N {}
set ::N::v\uD800 1
info exists ::N::v\uD800";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let declaration_offset = u32::try_from(source.find("set ::N::v").unwrap() + 4).unwrap();
        let read_offset = u32::try_from(source.rfind("::N::v").unwrap()).unwrap();
        let declaration = analysis
            .original_variable_symbol_in_source(&image, config, declaration_offset)
            .unwrap_or_else(|| panic!("{profile}: conditional absolute source declaration"));
        let reference = analysis
            .original_variable_symbol_in_source(&image, config, read_offset)
            .unwrap_or_else(|| panic!("{profile}: conditional absolute source reference"));
        assert!(declaration.is_declaration(), "{profile}: source write role");
        assert!(!reference.is_declaration(), "{profile}: source read role");
        assert_eq!(
            declaration.symbol(),
            reference.symbol(),
            "{profile}: exact source identity"
        );
        assert!(declaration.symbol().is_namespace());
        let advice = analysis
            .original_variable_write_advice
            .iter()
            .find(|advice| advice.span() == declaration.span())
            .unwrap_or_else(|| panic!("{profile}: original source write card"));
        let metadata = advice
            .conditional_metadata()
            .unwrap_or_else(|| panic!("{profile}: retained conditional applicability"));
        assert!(metadata.matches_source(&image, config));
        assert_ne!(metadata.obligations(), []);
        assert_eq!(
            declaration.original_name_input().bytes(),
            b"::N::v\xed\xa0\x80"
        );
        assert!(
            !analysis.original_variable_rename_is_complete(declaration.symbol()),
            "{profile}: unresolved command alternatives remain a coverage obligation"
        );
        assert!(
            analysis
                .original_variable_symbol_in_source(
                    &tcl_lexer::SourceImage::document(
                        &source.replace("namespace eval N", "namespace eval M")
                    ),
                    config,
                    declaration_offset,
                )
                .is_none(),
            "{profile}: complete source currency"
        );
    }
}

#[test]
fn original_conditional_variable_receivers_do_not_invent_bare_namespace_targets() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    // Implementation contract: naming.variable.conditional-registry-receiver-geometry
    // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
    let source = "set auto_index(unknown_command) {source lib.tcl}; unknown_command; namespace eval N {set bare 1; info exists bare}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        for offset in source
            .match_indices("bare")
            .map(|(offset, _)| u32::try_from(offset).unwrap())
        {
            assert!(
                analysis
                    .original_variable_symbol_in_source(&image, config, offset)
                    .is_none(),
                "{profile}: a conditional namespace card cannot select ordinary variable storage"
            );
        }
    }
}

#[test]
fn original_write_name_advice_keeps_source_cards_and_authentic_local_frames() {
    // Implementation contract: naming.variable.conditional-write-name-advice
    // docs/design/analysis/name-resolution-proofs/conditional-write-name-advice.md
    let source = r"set v\uD800 1; set v\uD801 2; proc p {} {set local 3; set local; unset local; global shared}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let inputs: Vec<_> = analysis
            .original_variable_write_advice
            .iter()
            .map(|advice| advice.original_name_input().bytes())
            .collect();
        assert!(
            inputs.contains(&b"v\xed\xa0\x80".as_slice()),
            "{profile}: first opaque source card"
        );
        assert!(
            inputs.contains(&b"v\xed\xa0\x81".as_slice()),
            "{profile}: distinct opaque source card"
        );
        let local = analysis
            .original_variable_write_advice
            .iter()
            .find(|advice| advice.original_name_input().bytes() == b"local")
            .unwrap_or_else(|| panic!("{profile}: own local write card"));
        assert!(
            matches!(
                local.original_frame().map(|frame| frame.frame().layout()),
                Some(crate::var_resolve::VariableExecutionFrame::Procedure { .. })
            ),
            "{profile}: original procedure owner"
        );
        assert_eq!(
            inputs.iter().filter(|&&input| input == b"local").count(),
            1,
            "reads and unsets are not source write declarations"
        );
        assert!(
            !inputs.contains(&b"shared".as_slice()),
            "alias source names remain separate"
        );
        assert_eq!(
            local.receiver_form(),
            tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined
        );
    }
}

#[test]
fn caught_caller_rebinding_cannot_retain_the_original_formal_symbol() {
    // Native proof: naming.variable.catch-result-alias-continuity
    // docs/design/analysis/name-resolution-proofs/catch-result-alias-continuity.md
    // Native ASCII controls independently show catch's result reaching the
    // rebound root slot. These assertions test source identity withdrawal.
    let source = "proc mutate {} {uplevel 1 {unset name; upvar #0 ::destination name}; error BOOM}\nproc probe {name} {catch {mutate} name; return $name}";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let formal = u32::try_from(source.find("{name}").unwrap() + 1).unwrap();
        let original = analysis
            .original_variable_symbol_at_offset(formal)
            .unwrap_or_else(|| panic!("{profile}: original formal declaration"));
        let read = u32::try_from(source.rfind("$name").unwrap() + 1).unwrap();
        assert!(
            analysis
                .original_variable_symbol_at_offset(read)
                .is_none_or(|current| current.symbol() != original.symbol()),
            "a caught caller mutation cannot donate the original formal cell"
        );
        assert!(!analysis.original_variable_rename_is_complete(original.symbol()));
    }
}

#[test]
fn logical_lambda_formals_join_body_source_symbols_without_native_or_edit_authority() {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolReceiver, SignatureSourceVariableSlot,
    };
    for source in [
        "apply {{first {second DEFAULT} args} {set first NEXT; return $first}} ONE",
        "namespace eval N {apply {{first {second DEFAULT} args} {return $first}} ONE}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        for name in ["first", "second", "args"] {
            let offset = u32::try_from(source.find(name).unwrap()).unwrap();
            let declaration = analysis
                .original_variable_symbol_in_source(&image, config, offset)
                .unwrap_or_else(|| panic!("actual source formal {name}: {source}"));
            assert!(declaration.is_declaration());
            assert_eq!(
                declaration.receiver(),
                OriginalVariableSymbolReceiver::FormalDeclaration
            );
            assert!(matches!(
                declaration.original_name_input(),
                SignatureSourceNameInput::OriginalLogicalListField(_)
            ));
            assert!(matches!(
                declaration.symbol().slot(),
                SignatureSourceVariableSlot::Local { .. }
            ));
            assert!(declaration.rename_span().is_none());
            assert!(declaration.renamed_input(b"renamed").is_none());
            assert!(!analysis.original_variable_rename_is_complete(declaration.symbol()));
            let mut wrong = config;
            wrong.strict_quoting = !wrong.strict_quoting;
            assert!(
                analysis
                    .original_variable_symbol_in_source(&image, wrong, offset)
                    .is_none()
            );
            assert!(
                analysis
                    .original_variable_symbol_in_source(
                        &tcl_lexer::SourceImage::document(&source.replace("ONE", "TWO")),
                        config,
                        offset,
                    )
                    .is_none()
            );
            if name == "first" {
                let read = analysis
                    .original_variable_symbol_in_source(
                        &image,
                        config,
                        u32::try_from(source.rfind("$first").unwrap() + 1).unwrap(),
                    )
                    .expect("actual body source read");
                assert_eq!(declaration.symbol(), read.symbol());
            }
        }
    }
}

#[test]
fn logical_lambda_formal_source_symbols_refuse_invalid_layouts_and_keep_activations_distinct() {
    for source in [
        "apply {{first} {return $first}}",
        "apply {{first first} {return $first}} ONE TWO",
        r"apply {{{fi\u0072st}} {return}} ONE",
        "proc apply args {}; apply {{first} {return $first}} ONE",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(
            source
                .find("first")
                .unwrap_or_else(|| source.find("fi\\u0072st").unwrap()),
        )
        .unwrap();
        assert!(
            analysis
                .original_variable_symbol_at_offset(offset)
                .is_none(),
            "{source}"
        );
    }
    let source = "apply {{item} {return $item}} ONE; apply {{item} {return $item}} TWO";
    let analysis = Analyser::new().analyse(source, "tcl");
    let offsets = source
        .match_indices("item")
        .map(|(offset, _)| u32::try_from(offset).unwrap())
        .collect::<Vec<_>>();
    let first = analysis
        .original_variable_symbol_at_offset(offsets[0])
        .unwrap();
    let first_read = analysis
        .original_variable_symbol_at_offset(offsets[1])
        .unwrap();
    let second = analysis
        .original_variable_symbol_at_offset(offsets[2])
        .unwrap();
    let second_read = analysis
        .original_variable_symbol_at_offset(offsets[3])
        .unwrap();
    assert_eq!(first.symbol(), first_read.symbol());
    assert_eq!(second.symbol(), second_read.symbol());
    assert_ne!(first.symbol(), second.symbol());
    assert!(!analysis.original_variable_rename_is_complete(first.symbol()));
    assert!(!analysis.original_variable_rename_is_complete(second.symbol()));
}

#[test]
fn original_formal_variable_symbols_keep_list_children_and_binding_topology() {
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolReceiver, SignatureSourceVariableSlot,
    };
    let source =
        r"proc p {v\uD800 {d\uD800 DEFAULT} args} {list [set v\uD800] [set d\uD800] [set args]}";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        for needle in [r"v\uD800", r"d\uD800", "args"] {
            let offsets: Vec<_> = source
                .match_indices(needle)
                .map(|(offset, _)| u32::try_from(offset).unwrap())
                .collect();
            let declaration = analysis
                .original_variable_symbol_at_offset(offsets[0])
                .unwrap_or_else(|| panic!("{profile}: formal {needle}"));
            let read = analysis
                .original_variable_symbol_at_offset(offsets[1])
                .unwrap_or_else(|| panic!("{profile}: read {needle}"));
            assert_eq!(declaration.symbol(), read.symbol(), "{profile}: {needle}");
            assert_eq!(
                declaration.receiver(),
                OriginalVariableSymbolReceiver::FormalDeclaration
            );
            assert!(declaration.is_declaration());
            assert!(matches!(
                declaration.symbol().slot(),
                SignatureSourceVariableSlot::Local { .. }
            ));
            let container = declaration
                .original_name_input()
                .original_static_list_container()
                .expect("authentic ParamList ancestry");
            assert_ne!(container.ordinals(), [] as [usize; 0]);
            assert!(
                analysis.original_variable_rename_is_complete(declaration.symbol()),
                "{profile}: complete original naming occurrences"
            );
            if needle == "args" {
                assert!(
                    declaration.renamed_input(b"new").is_none(),
                    "variadic binding cannot silently become a required formal"
                );
            } else {
                assert_eq!(declaration.renamed_input(b"new"), Some(b"new".to_vec()));
                assert!(
                    declaration.renamed_input(b"args").is_none(),
                    "required/default binding topology cannot change"
                );
            }
        }
    }
    let source = "proc p {a a {b DEFAULT}} {set a; set b}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let first = analysis
        .original_variable_symbol_at_offset(8)
        .expect("first duplicate formal");
    let second = analysis
        .original_variable_symbol_at_offset(10)
        .expect("second duplicate formal");
    assert_eq!(first.symbol(), second.symbol());
    assert_ne!(
        first
            .original_name_input()
            .original_static_list_container()
            .unwrap()
            .ordinals(),
        second
            .original_name_input()
            .original_static_list_container()
            .unwrap()
            .ordinals()
    );
    assert_eq!(first.renamed_input(b"new"), Some(b"new".to_vec()));
    assert_eq!(second.renamed_input(b"new"), Some(b"new".to_vec()));
    let source = "proc p {{args rest}} {set rest}";
    let analysis = Analyser::new().analyse(source, "jimtcl");
    let declaration = analysis
        .original_variable_symbol_at_offset(14)
        .expect("Jim renamed variadic local field");
    let read = analysis
        .original_variable_symbol_at_offset(26)
        .expect("same genuine variadic local");
    assert_eq!(declaration.symbol(), read.symbol());
    assert_eq!(
        declaration
            .original_name_input()
            .original_static_list_container()
            .unwrap()
            .ordinals(),
        &[0, 1]
    );
    assert_eq!(declaration.renamed_input(b"new"), Some(b"new".to_vec()));
}

#[test]
fn original_variable_alias_reads_keep_derived_and_independent_local_spellings() {
    let source = r"set ::g\uD800 VALUE; proc p {} {global g\uD800; set g\uD800; set $name 1}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let offsets: Vec<_> = source
            .match_indices(r"g\uD800")
            .map(|(offset, _)| u32::try_from(offset).unwrap())
            .collect();
        let declaration = analysis
            .original_variable_symbol_at_offset(offsets[1])
            .expect("authentic selected global declaration");
        let read = analysis
            .original_variable_symbol_at_offset(offsets[2])
            .unwrap_or_else(|| panic!("{profile}: original alias read"));
        assert_eq!(declaration.symbol(), read.symbol());
        assert!(read.rename_changes_spelling());
        assert_eq!(read.renamed_input(b"new"), Some(b"new".to_vec()));
        assert!(
            !analysis.original_variable_rename_is_complete(declaration.symbol()),
            "computed variable names are rename coverage obligations"
        );
        let complete_source = source.replace("; set $name 1", "");
        let complete = Analyser::new().analyse(&complete_source, profile);
        let declaration = complete
            .original_variable_symbol_at_offset(offsets[1])
            .unwrap();
        assert!(
            complete.original_variable_rename_is_complete(declaration.symbol()),
            "{profile}: known alias producer and every original read"
        );
    }
    let source = "set ::x VALUE; proc p {} {upvar #0 x local; set local; return $local}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let target = analysis
            .original_variable_symbol_at_offset(35)
            .expect("genuine #0 target");
        for offset in [48, 63] {
            let read = analysis
                .original_variable_symbol_at_offset(offset)
                .unwrap_or_else(|| panic!("{profile}: original independent alias read"));
            assert_eq!(read.symbol(), target.symbol());
            assert!(!read.rename_changes_spelling());
            assert_eq!(read.renamed_input(b"new"), Some(b"local".to_vec()));
        }
        assert!(
            analysis.original_variable_rename_is_complete(target.symbol()),
            "{profile}: original #0 local does not need spelling edits"
        );
    }
    let source = "set ::x VALUE; proc p {} {upvar 1 other local; set local}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let target = analysis.original_variable_symbol_at_offset(6).unwrap();
    assert!(
        !analysis.original_variable_rename_is_complete(target.symbol()),
        "missing authentic foreign frame target blocks coverage"
    );
}

#[test]
fn original_local_variable_symbols_keep_opaque_names_and_genuine_body_frames() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let source = r"proc p {} {set v\uD800 1; set v\uD801 2; set v\uD800}; proc q {} {set v\uD800 3; set v\uD800}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let offsets: Vec<_> = source
            .match_indices(r"v\uD800")
            .map(|(offset, _)| u32::try_from(offset).unwrap())
            .collect();
        let sites: Vec<_> = offsets
            .iter()
            .map(|&offset| {
                analysis
                    .original_variable_symbol_at_offset(offset)
                    .unwrap_or_else(|| panic!("{profile}: original local at {offset}"))
            })
            .collect();
        assert_eq!(
            sites[0].symbol(),
            sites[1].symbol(),
            "{profile}: one original body"
        );
        assert_eq!(
            sites[2].symbol(),
            sites[3].symbol(),
            "{profile}: second original body"
        );
        assert_ne!(
            sites[0].symbol(),
            sites[2].symbol(),
            "{profile}: distinct frame recipes"
        );
        let SignatureSourceVariableSlot::Local { simple, frame } = sites[0].symbol().slot() else {
            panic!("ordinary procedure local became namespace storage");
        };
        assert_eq!(simple.as_bytes(), b"v\xed\xa0\x80");
        assert!(!sites[0].symbol().is_namespace());
        assert_eq!(frame.lexer_config(), analysis.body_lexer_config.unwrap());
        assert!(sites[0].is_declaration());
        assert!(!sites[1].is_declaration());
        let other = analysis
            .original_variable_symbol_at_offset(
                u32::try_from(source.find(r"v\uD801").unwrap()).unwrap(),
            )
            .unwrap();
        assert_ne!(sites[0].symbol(), other.symbol());
        let proposed = sites[0].symbol().renamed(b"new").unwrap();
        let SignatureSourceVariableSlot::Local {
            frame: proposed_frame,
            simple,
        } = proposed.slot()
        else {
            unreachable!()
        };
        assert_eq!(proposed_frame, frame);
        assert_eq!(simple.as_bytes(), b"new");
        assert_eq!(sites[0].renamed_input(b"new"), Some(b"new".to_vec()));
        assert!(sites[0].renamed_input(b"other::new").is_none());
    }
}

// Native proof: naming.variable.namespace-store-frame-and-fallback
// docs/design/analysis/name-resolution-proofs/namespace-store-frame-and-fallback.md
#[test]
fn original_namespace_store_symbols_use_the_selected_activation_and_fallback() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "set ::x ROOT; namespace eval N {set x VALUE}";
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.rfind("x VALUE").unwrap()).unwrap();
        let occurrence = analysis
            .original_variable_symbol_at_offset(offset)
            .unwrap_or_else(|| panic!("{profile}: actual namespace set owner absent"));
        match occurrence.symbol().slot() {
            SignatureSourceVariableSlot::C { namespace, simple } => {
                assert_eq!(simple.as_bytes(), b"x");
                let expected = if matches!(profile, "tcl9.0" | "tcl9.1") {
                    tcl_core_types::ByteNamespacePath::from_segments([b"N".as_slice()])
                } else {
                    tcl_core_types::ByteNamespacePath::root()
                };
                assert_eq!(*namespace, expected, "{profile}: existing root selection");
            }
            SignatureSourceVariableSlot::Local { frame, simple } => {
                assert_eq!(profile, "jimtcl");
                assert_eq!(simple.as_bytes(), b"x");
                assert!(matches!(
                    frame.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                ));
            }
            SignatureSourceVariableSlot::Jim(_) => {
                panic!("bare Jim namespace set became global storage")
            }
        }
        let source = "namespace eval Fresh {set x VALUE}";
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.find("x VALUE").unwrap()).unwrap();
        let occurrence = analysis
            .original_variable_symbol_at_offset(offset)
            .unwrap_or_else(|| panic!("{profile}: fresh namespace set owner absent"));
        if profile == "jimtcl" {
            assert!(!occurrence.symbol().is_namespace());
        } else {
            let SignatureSourceVariableSlot::C { namespace, .. } = occurrence.symbol().slot()
            else {
                panic!("C fresh namespace store lost its namespace owner");
            };
            assert_eq!(
                *namespace,
                tcl_core_types::ByteNamespacePath::from_segments([b"Fresh".as_slice()])
            );
        }
        let source = "set ::x ROOT; namespace eval Explicit {variable x VALUE}";
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.find("x VALUE").unwrap()).unwrap();
        let occurrence = analysis
            .original_variable_symbol_at_offset(offset)
            .unwrap_or_else(|| panic!("{profile}: explicit namespace variable absent"));
        match occurrence.symbol().slot() {
            SignatureSourceVariableSlot::C { namespace, simple } => {
                assert_eq!(
                    *namespace,
                    tcl_core_types::ByteNamespacePath::from_segments([b"Explicit".as_slice()])
                );
                assert_eq!(simple.as_bytes(), b"x");
            }
            SignatureSourceVariableSlot::Jim(key) => assert_eq!(key.as_bytes(), b"Explicit::x"),
            SignatureSourceVariableSlot::Local { .. } => {
                panic!("explicit namespace variable became local")
            }
        }
    }
}

#[test]
fn original_variable_symbols_select_global_namespace_and_remote_frame_purposes() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let source = r"namespace eval T {}; namespace eval N {proc p {} {global g\uD800; variable v\uD800; namespace upvar ::T t\uD800 local; upvar #0 u\uD800 rootAlias; upvar 1 remote callerAlias}}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let at = |needle: &str| {
            analysis.original_variable_symbol_at_offset(
                u32::try_from(source.find(needle).unwrap()).unwrap(),
            )
        };
        for (needle, namespace, simple) in [
            (r"g\uD800", b"".as_slice(), b"g\xed\xa0\x80".as_slice()),
            (r"v\uD800", b"N".as_slice(), b"v\xed\xa0\x80".as_slice()),
            (r"t\uD800", b"T".as_slice(), b"t\xed\xa0\x80".as_slice()),
            (r"u\uD800", b"".as_slice(), b"u\xed\xa0\x80".as_slice()),
        ] {
            let site = at(needle).unwrap_or_else(|| {
                panic!(
                    "{profile}: {needle} {:#?}",
                    analysis.original_variable_symbols
                )
            });
            assert!(site.is_declaration(), "{profile}: {needle}");
            match site.symbol().slot() {
                SignatureSourceVariableSlot::C {
                    namespace: path,
                    simple: key,
                } => {
                    let expected = if namespace.is_empty() {
                        tcl_core_types::ByteNamespacePath::root()
                    } else {
                        tcl_core_types::ByteNamespacePath::from_segments([namespace])
                    };
                    assert_eq!(*path, expected, "{profile}: {needle}");
                    assert_eq!(key.as_bytes(), simple);
                }
                SignatureSourceVariableSlot::Local { .. } => {
                    panic!("namespace receiver selected a local frame")
                }
                SignatureSourceVariableSlot::Jim(key) => {
                    let mut expected = namespace.to_vec();
                    if !expected.is_empty() {
                        expected.extend_from_slice(b"::");
                    }
                    expected.extend_from_slice(simple);
                    assert_eq!(key.as_bytes(), expected, "{profile}: {needle}");
                }
            }
        }
        assert!(
            at("remote").is_none(),
            "a foreign caller frame is not the caller namespace"
        );
        assert!(
            at("callerAlias").is_none(),
            "local alias spelling is a separate symbol"
        );
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis
            .body_lexer_config
            .expect("actual retained body grammar");
        assert!(analysis.matches_original_source_image(&image, config));
        assert!(!analysis.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&format!("{source}; list changed")),
            config
        ));
        assert!(!analysis.matches_original_source_image(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            config
        ));
    }
}

// Native proof: naming.array-source.combined-set-receiver
// docs/design/analysis/name-resolution-proofs/array-source-combined-set-receiver.md
// Native proof: naming.array-source.scalar-element-storage
// docs/design/analysis/name-resolution-proofs/array-source-scalar-element-storage.md
// Native proof: naming.array-source.combined-read-enumeration
// docs/design/analysis/name-resolution-proofs/array-source-combined-read-enumeration.md
// Native proof: naming.array-source.combined-unset
// docs/design/analysis/name-resolution-proofs/array-source-combined-unset.md
#[test]
fn original_array_receiver_symbols_keep_combined_names_and_guest_error_independent() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    // Each receiver has its own original program point. In C Tcl an array
    // setter on a combined element name errors before later script words.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (command, declaration) in [
            ("array set ::N::a(b) {k V}", false),
            ("set ::N::a(b) VALUE", true),
            ("array get ::N::a(b)", false),
            ("array names ::N::a(b)", false),
            ("array unset ::N::a(b)", false),
        ] {
            let source = format!("namespace eval N {{}}; {command}");
            let analysis = Analyser::new().analyse(&source, profile);
            let offset = u32::try_from(source.find("::N::a(b)").unwrap()).unwrap();
            let site = analysis
                .original_variable_symbol_at_offset(offset)
                .unwrap_or_else(|| panic!("{profile}: original receiver for {command}"));
            assert_eq!(site.is_declaration(), declaration, "{profile}: {command}");
            match site.symbol().slot() {
                SignatureSourceVariableSlot::C { namespace, simple } => {
                    assert_eq!(
                        *namespace,
                        tcl_core_types::ByteNamespacePath::from_segments([b"N".as_slice()])
                    );
                    assert_eq!(simple.as_bytes(), b"a");
                }
                SignatureSourceVariableSlot::Local { .. } => {
                    panic!("absolute receiver selected a local frame")
                }
                SignatureSourceVariableSlot::Jim(key) => assert_eq!(key.as_bytes(), b"N::a"),
            }
        }
    }
}

/// TN — an **unqualified** occurrence is never recorded: it names whatever
/// the local scope chain supplies, which no other document can resolve.
#[test]
fn unqualified_var_occurrences_are_not_recorded_as_qualified_refs() {
    let src = "set counter 0\nincr counter\nputs $counter\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.qualified_var_refs.is_empty(),
        "bare names carry no cross-document identity: {:?}",
        r.qualified_var_refs,
    );
}

/// TP — the post-walk attach pass links a *relative*-qualified read to the
/// declaring namespace cell in the same document, including when the read
/// is written **before** the `namespace eval` that declares it.
///
/// Oracle (tclsh 9.0.4 / 8.6.16): `proc p {} { return $::n::v }; namespace
/// eval n { variable v 1 }; puts [p]` prints `1`.
#[test]
fn qualified_reads_attach_to_the_namespace_cell_regardless_of_order() {
    let src = "proc p {} { return $::n::v }\n\
                   namespace eval n {\n    variable v 1\n}\n\
                   puts $n::v\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let (_, var) = namespace_variables(&r.global_scope)
        .into_iter()
        .find(|(q, _)| q == "::n::v")
        .expect("::n::v declared");
    assert_eq!(
        var.references.len(),
        2,
        "both the earlier `$::n::v` and the later `$n::v` are references: {:?}",
        var.references,
    );
}

/// TP — `namespace_variables` enumerates exactly the cells
/// `lookup_var_in_namespace` can find, and never a proc local.
#[test]
fn namespace_variables_enumerates_namespace_cells_only() {
    let src = "namespace eval a::b {\n    variable v 1\n}\n\
                   proc a::b::f {} {\n    set localOnly 2\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let names: Vec<String> = namespace_variables(&r.global_scope)
        .into_iter()
        .map(|(q, _)| q)
        .collect();
    assert!(names.contains(&"::a::b::v".to_owned()), "{names:?}");
    assert!(
        !names.iter().any(|n| n.ends_with("::localOnly")),
        "a proc local is not a namespace cell: {names:?}",
    );
}

#[test]
fn lookup_var_in_namespace_finds_var_in_matching_top_level_namespace() {
    // TP — the base case both mined findings (idx=107, idx=115) reduce
    // to: `namespace eval ::simple { variable v ... }`, then `$::simple::v`
    // referenced elsewhere.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    ns_a.variables
        .insert("v".to_string(), var("v", span(10, 11)));
    root.children.push(ns_a);

    let found = lookup_var_in_namespace(&root, "::A", "v");
    assert_eq!(found.map(|v| v.definition_span), Some(span(10, 11)));
    assert!(lookup_var_in_namespace(&root, "::A", "missing").is_none());
}

#[test]
fn lookup_var_in_namespace_accumulates_through_nested_namespaces() {
    // TP — a namespace written 2+ levels deep must accumulate through
    // every enclosing `namespace eval`, exactly like command resolution
    // (`advance_command_resolution_namespace`, reused verbatim here).
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "A");
    let mut ns_b = Scope::new(ScopeKind::Namespace, "B");
    ns_b.variables
        .insert("v".to_string(), var("v", span(20, 21)));
    ns_a.children.push(ns_b);
    root.children.push(ns_a);

    assert_eq!(
        lookup_var_in_namespace(&root, "::A::B", "v").map(|v| v.definition_span),
        Some(span(20, 21))
    );
    // The intermediate namespace's own (empty) table must not be
    // mistaken for the leaf's.
    assert!(lookup_var_in_namespace(&root, "::A", "v").is_none());
}

#[test]
fn lookup_var_in_namespace_finds_var_defined_in_a_reopened_block() {
    // TP — the generalisation the mined findings' fix direction calls
    // for: a namespace can be `namespace eval`-reopened any number of
    // times (even lexically unrelated ones), so a single Scope tree
    // node must never be assumed to hold every variable a qualified
    // reference might mean. Two independent `::A` blocks here; only the
    // second declares `v`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.children.push(Scope::new(ScopeKind::Namespace, "::A"));
    let mut ns_a_reopened = Scope::new(ScopeKind::Namespace, "::A");
    ns_a_reopened
        .variables
        .insert("v".to_string(), var("v", span(30, 31)));
    root.children.push(ns_a_reopened);

    assert_eq!(
        lookup_var_in_namespace(&root, "::A", "v").map(|v| v.definition_span),
        Some(span(30, 31))
    );
}

#[test]
fn lookup_var_in_namespace_skips_proc_locals_even_when_namespace_matches() {
    // FP guard — a proc's own defining ("command-resolution") namespace
    // can coincide with a target namespace path (any proc's does, by
    // construction), but its *local* variable table is never the
    // namespace's cells: a plain `set v 1` inside `proc ::A::f` has no
    // relation to a same-named `::A::v` unless linked via `variable`/
    // `global` (which resolves through the ordinary scope-chain walk,
    // not this namespace-table lookup). Only `f`'s *own* locals exist
    // here — `::A` itself declares nothing — so a hit would prove the
    // guard is missing.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    let mut proc_f = Scope::new(ScopeKind::Proc, "f");
    proc_f
        .variables
        .insert("v".to_string(), var("v", span(40, 41)));
    ns_a.children.push(proc_f);
    root.children.push(ns_a);

    assert!(
        lookup_var_in_namespace(&root, "::A", "v").is_none(),
        "a proc-local variable must never satisfy a namespace-qualified lookup"
    );
}

#[test]
fn lookup_var_in_namespace_returns_none_for_unknown_namespace() {
    // TN — a target namespace that appears nowhere in the tree.
    let root = Scope::new(ScopeKind::Global, "::");
    assert!(lookup_var_in_namespace(&root, "::Nope", "v").is_none());
}

#[test]
fn lookup_var_in_namespace_finds_global_scope_variable() {
    // TP — the degenerate `target_ns == "::"` case: the root scope
    // itself is `::`, so an absolute `$::v` reference must resolve
    // directly against the root's own variable table.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables.insert("v".to_string(), var("v", span(0, 1)));
    assert_eq!(
        lookup_var_in_namespace(&root, "::", "v").map(|v| v.definition_span),
        Some(span(0, 1))
    );
}

// `finalise_invocation_resolutions`'s local-vs-global candidate choice.
// A "known" predicate that compared only the final deletion offset against
// the candidate's own establishing offset, never the call site, would make
// a namespaced local call textually *before* a later unconditional
// deletion wrongly lose to the global candidate. Confirmed against
// tclsh 8.6.14 throughout.

#[test]
fn local_call_before_later_deletion_resolves_local_not_global_codex_1009() {
    // TP: `foo::caller`'s own top-level
    // invocation runs before `rename foo::bar {}`, so its `bar` call
    // must still resolve to the local `::foo::bar`, not the global
    // `::bar` — confirmed against tclsh 8.6.14 (prints "local").
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nfoo::caller\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::foo::bar"),
        "a call before the deletion must still resolve to the local candidate"
    );
    assert!(
        bar_call.resolved_user_definition,
        "the position-sensitive winner is a live user definition at this call"
    );
}

#[test]
fn qualified_name_for_var_decl_finds_var_in_matching_top_level_namespace() {
    // TP — the reverse of `lookup_var_in_namespace_finds_var_in_matching_top_level_namespace`:
    // given the declaration's own span, recover the
    // qualified name an alias elsewhere would name it by.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    ns_a.variables
        .insert("v".to_string(), var("v", span(10, 11)));
    root.children.push(ns_a);

    assert_eq!(
        qualified_name_for_var_decl(&root, span(10, 11)),
        Some("::A::v".to_string())
    );
}

#[test]
fn local_call_after_deletion_still_falls_back_to_global_issue_973() {
    // FN guard: unlike the case above, `foo::caller` is
    // only ever invoked (at the top level) *after* `rename foo::bar
    // {}` runs, so the local `bar` is genuinely gone by the time the
    // call executes — it must still fall back to the global `::bar`.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    rename foo::bar {}\n    proc caller {} { return [bar] }\n}\nfoo::caller\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::bar"),
        "a call genuinely after the deletion must fall back to the global candidate"
    );
    assert!(
        bar_call.resolved_user_definition,
        "the global fallback is itself a live user definition"
    );
}

#[test]
fn qualified_name_for_var_decl_accumulates_through_nested_namespaces() {
    // TP — mirrors `lookup_var_in_namespace_accumulates_through_nested_namespaces`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "A");
    let mut ns_b = Scope::new(ScopeKind::Namespace, "B");
    ns_b.variables
        .insert("v".to_string(), var("v", span(20, 21)));
    ns_a.children.push(ns_b);
    root.children.push(ns_a);

    assert_eq!(
        qualified_name_for_var_decl(&root, span(20, 21)),
        Some("::A::B::v".to_string())
    );
}

#[test]
fn escape_hatch_requires_the_specific_enclosing_definitions_own_top_level_call() {
    // FN guard: an unrelated proc's top-level invocation elsewhere in
    // the file must not "lend" liveness to a different enclosing
    // definition that is itself never invoked — the escape hatch must
    // key off the *specific* body's own top-level call, not just
    // "some call happened somewhere before the deletion". `foo::bar`
    // is deleted, and `foo::caller` (the only body that calls it) is
    // never invoked anywhere — only the unrelated `baz::unrelated` is
    // — so the `bar` call must fall back to the global `::bar`.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nnamespace eval baz {\n    proc unrelated {} { return 1 }\n}\nbaz::unrelated\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::bar"),
        "foo::caller is never invoked, so the escape hatch must not apply"
    );
}

#[test]
fn escape_hatch_follows_a_chain_of_enclosing_definitions_issue_1015() {
    // FP guard: `foo::caller` is never invoked at the top
    // level — only `foo::entry` is — but `entry` calls `caller`, which
    // calls `bar`, all before the rename, so the local `::foo::bar`
    // stays the resolution. tclsh8.6/9.0 confirm the chain runs clean.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n    proc entry {} { return [caller] }\n}\nfoo::entry\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::foo::bar"),
        "foo::entry reaches foo::caller, which reaches bar, before the rename"
    );
}

#[test]
fn escape_hatch_terminates_on_a_never_entered_mutual_recursion_cycle_issue_1015() {
    // TP guard: `foo::ping` and `foo::pong` call each
    // other and nothing calls either, so neither is ever reached — the
    // reachability fixpoint must terminate and leave the escape hatch
    // shut, falling the `bar` call back to the global `::bar`.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc ping {} { pong\n        return [bar] }\n    proc pong {} { return [ping] }\n}\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::bar"),
        "neither cycle member is ever reached, so the escape hatch must not apply"
    );
}

#[test]
fn qualified_name_for_var_decl_skips_proc_locals() {
    // FP guard — a proc-local variable's declaration span must never
    // produce a qualified name: `ScopeKind::Proc` is excluded from the
    // walk exactly like `lookup_var_in_namespace_skips_proc_locals_even_when_namespace_matches`,
    // since a bare `set v 1` inside a proc has no relation to any
    // namespace cell unless linked via `variable`/`global` (a separate
    // `VarDef` with its own `link_target`, not this one).
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    let mut proc_f = Scope::new(ScopeKind::Proc, "f");
    proc_f
        .variables
        .insert("v".to_string(), var("v", span(40, 41)));
    ns_a.children.push(proc_f);
    root.children.push(ns_a);

    assert!(
        qualified_name_for_var_decl(&root, span(40, 41)).is_none(),
        "a proc-local variable's span must never resolve to a qualified name"
    );
}

#[test]
fn qualified_name_for_var_decl_returns_none_for_unknown_span() {
    // TN — a span that names no declaration anywhere in the tree.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables.insert("v".to_string(), var("v", span(0, 1)));
    assert!(qualified_name_for_var_decl(&root, span(99, 100)).is_none());
}

#[test]
fn qualified_name_for_var_decl_finds_global_scope_variable() {
    // TP — the degenerate root/`::` case: a variable declared directly
    // in the global scope qualifies as `::name`, not `::::name`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables.insert("v".to_string(), var("v", span(0, 1)));
    assert_eq!(
        qualified_name_for_var_decl(&root, span(0, 1)),
        Some("::v".to_string())
    );
}

#[test]
fn qualified_name_for_var_decl_does_not_double_prefix_a_literal_qualified_name() {
    // TP: `bind_value_word_assignment`/`define_var` never
    // re-qualify a name they're given (`normalise_var_name` only strips
    // a `$`/`${…}` wrapper and an array index), so a literal `set
    // ::tolComp val` stores `VarDef::name == "::tolComp"` verbatim, not
    // the bare tail `"tolComp"`. Prefixing that with `ns` again would
    // produce `"::::tolComp"`, matching no alias's `link_target`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("::tolComp".to_string(), var("::tolComp", span(50, 60)));
    assert_eq!(
        qualified_name_for_var_decl(&root, span(50, 60)),
        Some("::tolComp".to_string())
    );
}

#[test]
fn lookup_var_by_qualified_name_finds_a_bare_tail_namespace_variable() {
    // TP — the ordinary `variable`-declared namespace cell shape,
    // delegated straight through to `lookup_var_in_namespace`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    ns_a.variables
        .insert("v".to_string(), var("v", span(10, 11)));
    root.children.push(ns_a);

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::A::v").map(|v| v.definition_span),
        Some(span(10, 11))
    );
}

#[test]
fn lookup_var_by_qualified_name_finds_a_literal_qualified_top_level_set() {
    // TP — a plain `set ::tolComp val` at global scope stores its key
    // verbatim (`"::tolComp"`), which the bare-tail lookup alone
    // (`base_name == "tolComp"`) can never match; the literal-name
    // fallback must.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("::tolComp".to_string(), var("::tolComp", span(50, 60)));

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::tolComp").map(|v| v.definition_span),
        Some(span(50, 60))
    );
}

#[test]
fn lookup_var_by_qualified_name_finds_an_unqualified_top_level_set() {
    // TP — the other half: an *unqualified* `set tolComp val` at global
    // scope stores the bare key `"tolComp"`, found by the tail-based
    // lookup with no fallback needed.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("tolComp".to_string(), var("tolComp", span(50, 60)));

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::tolComp").map(|v| v.definition_span),
        Some(span(50, 60))
    );
}

#[test]
fn lookup_var_by_qualified_name_does_not_conflate_unrelated_literal_cells() {
    // FP guard — two literal-qualified cells in different namespaces
    // must never cross-match just because both are reachable via the
    // literal-name fallback.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("::A::x".to_string(), var("::A::x", span(1, 2)));
    root.variables
        .insert("::B::x".to_string(), var("::B::x", span(3, 4)));

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::A::x").map(|v| v.definition_span),
        Some(span(1, 2))
    );
    assert_eq!(
        lookup_var_by_qualified_name(&root, "::B::x").map(|v| v.definition_span),
        Some(span(3, 4))
    );
}

#[test]
fn lookup_var_by_qualified_name_returns_none_for_unknown_target() {
    // TN
    let root = Scope::new(ScopeKind::Global, "::");
    assert!(lookup_var_by_qualified_name(&root, "::Nope::v").is_none());
}
#[test]
fn navigation_uses_reached_body_definition_before_later_redefinition() {
    let source = "proc p {} {return FIRST}; proc run {} {p}; run; proc p {x} {return SECOND}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let result = Analyser::new().analyse(source, dialect);
        let call = result
            .command_invocations
            .iter()
            .find(|invocation| invocation.name == "p")
            .unwrap();
        let definition = call.resolved_definition.as_ref().expect(dialect);
        let allocation = definition.allocation();
        assert_eq!(allocation.site.offset, 0, "{dialect}");
        let declaration = result
            .proc_for_definition(definition, source)
            .unwrap_or_else(|| {
                panic!("{dialect}: retained definition {definition:?} has no authored declaration")
            });
        assert!(declaration.params.is_empty(), "{dialect}");
        assert_eq!(
            result
                .proc_def_in_effect_at("::p", call.range.start())
                .unwrap()
                .name_span,
            declaration.name_span,
            "{dialect}"
        );
        assert!(
            result
                .proc_for_definition(definition, "proc p {x} {return WRONG}")
                .is_none()
        );
    }
}

#[test]
fn definition_navigation_uses_retained_grammar_instead_of_display_dialect() {
    let source = "proc p {} {return FIRST}; p";
    let mut result = Analyser::new().analyse(source, "tcl8.4");
    let definition = result
        .command_invocations
        .iter()
        .find(|invocation| invocation.name == "p")
        .unwrap()
        .resolved_definition
        .clone()
        .unwrap();
    assert!(!result.body_lexer_config.unwrap().expand_syntax);
    result.dialect = "custom-display-name".to_owned();
    assert!(result.proc_for_definition(&definition, source).is_some());
    result.body_lexer_config = None;
    assert!(result.proc_for_definition(&definition, source).is_none());
}

#[test]
fn class_navigation_retains_selected_declaration_after_name_reuse() {
    let source = "oo::class create C {}; C create nav_before; rename C {}; oo::class create C {}; C create nav_after";
    let call_offsets = ["C create nav_before", "C create nav_after"]
        .map(|call| u32::try_from(source.find(call).unwrap()).unwrap());
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let calls = result
            .command_invocations
            .iter()
            .filter(|invocation| {
                invocation.name == "C" && call_offsets.contains(&invocation.range.start())
            })
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 2, "{dialect}");
        let declarations = calls
            .iter()
            .map(|invocation| {
                result
                    .class_for_definition(invocation.resolved_definition.as_ref().unwrap(), source)
                    .unwrap()
                    .name_span
            })
            .collect::<Vec<_>>();
        assert_ne!(declarations[0], declarations[1], "{dialect}");
        assert_eq!(result.superseded_classes["::C"].len(), 1);
        assert_eq!(result.all_classes["::C"].name_span, declarations[1]);
    }
}
