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
use crate::analyser::types::ScopeKind;

/// A source shell keeps the analysed document and its resolved input and
/// interprets nothing: no invocation, declaration or diagnostic, where the
/// ordinary analysis of the same text records all three. Dialect-invariant;
/// run under Tcl 8.6.
#[test]
fn a_source_shell_keeps_the_document_and_interprets_nothing() {
    let source = "proc p {} { set x 1 }\np\nundefined_command_here\n";
    let shell = Analyser::new().source_shell().analyse(source, "tcl8.6");
    assert!(shell.is_source_shell());
    assert!(shell.command_invocations.is_empty());
    assert!(shell.diagnostics.is_empty());
    assert!(shell.resolved_input.is_some());
    let image = shell
        .retained_command_realm()
        .and_then(crate::realm::CommandBindingRealm::original_source_image)
        .expect("the shell retains the document");
    assert_eq!(image.bytes(), source.as_bytes());

    let analysed = Analyser::new().analyse(source, "tcl8.6");
    assert!(!analysed.is_source_shell());
    assert!(!analysed.command_invocations.is_empty());
    assert!(!analysed.diagnostics.is_empty());
}

/// The composite-head split asks the shared owner where the
/// `${…}` name ends, so the dispatched variable and the word suffix move
/// with the release rather than always splitting at the first `}`.
///
/// The lexer merges `${ns}::setdef` into one `Var` token whose text is
/// `ns}::setdef`; a *pure* `${x}` arrives as `x`, its closer already
/// outside the span. Both shapes fall out of `braced_var_name_end`:
/// `Closed` is the composite case, `Unterminated` the pure one.
#[test]
fn split_braced_head_follows_the_release_close_rule() {
    let mut a = Analyser::new();

    // 9.x nests, so `${a{b}c}::setdef` dispatches on `a{b}c`.
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    assert_eq!(a.split_braced_head("ns}::setdef"), ("ns", "::setdef"));
    assert_eq!(a.split_braced_head("a{b}c}::setdef"), ("a{b}c", "::setdef"));
    // A pure `${…}` token has no closer inside its text: all name.
    assert_eq!(a.split_braced_head("a{b}c"), ("a{b}c", ""));
    assert_eq!(a.split_braced_head("x"), ("x", ""));
    // A simple `$obj` / `$ns::v` head is unchanged.
    assert_eq!(a.split_braced_head("ns::v"), ("ns::v", ""));

    // 8.x ends the name at the first literal `}`.
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    assert_eq!(a.split_braced_head("ns}::setdef"), ("ns", "::setdef"));
    assert_eq!(a.split_braced_head("a{b}c}::setdef"), ("a{b", "c}::setdef"));
}

#[test]
fn retained_analysis_fixture_keeps_the_returned_source_owner_for_private_queries() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let source = "eval $script";
    let mut analyser = Analyser::new();
    let returned = analyser.analyse(source, "tcl");
    assert!(returned.resolved_input.is_some());
    assert!(analyser.result.resolved_input.is_none());
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let input = analyser.result.resolved_input.as_ref().unwrap();
    assert!(analyser.result.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        input.lexer_config()
    ));
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config())
            .pop()
            .unwrap();
    assert!(
        analyser
            .original_diagnostic_call_at(command.argv[0], command.arg_tokens())
            .is_some()
    );
}

#[test]
fn unavailable_overlay_is_retained_before_every_analysis_walk() {
    let overlay = u64::MAX;
    let source = "proc must_not_be_walked {} {set x 1}; must_not_be_walked";
    let commands = crate::segmenter::segment_commands(source);
    let mut analyser = Analyser::new().with_pack_overlay(overlay);
    let missing = analyser
        .try_analysis_context()
        .expect_err("no registered overlay");
    assert_eq!(missing.overlay, overlay);
    let full = analyser.analyse(source, "tcl");
    let supplied = analyser.analyse_commands(source, &commands, "tcl", true);
    let (chunked, snapshots) = analyser.analyse_chunked(source, vec![commands.clone()], "tcl");
    assert!(snapshots.is_empty());
    let incremental = analyser.analyse_incremental(source, &commands, source, "tcl");
    let mut called = false;
    let per_item = analyser.analyse_per_item_with(source, "tcl", &mut |_| {
        called = true;
        unreachable!("an unavailable generation cannot dispatch a body")
    });
    assert!(!called);
    for result in vec![full, supplied, chunked, incremental, per_item].into_boxed_slice() {
        assert_eq!(result.analysis_context_unavailable.as_ref(), Some(&missing));
        assert!(result.resolved_input.is_none());
        assert!(result.all_procs.is_empty());
        assert_eq!(
            result.command_invocations,
            [] as [crate::signature_scan::types::SignatureCommandInvocation; 0]
        );
        assert_eq!(
            result.diagnostics,
            [] as [crate::analyser::types::Diagnostic; 0]
        );
        assert_eq!(
            result.global_scope.children,
            [] as [crate::analyser::types::Scope; 0]
        );
    }
}

#[test]
fn original_whole_source_recovery_keeps_file_config_and_typed_warning_fields() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original recovery/diagnostic streams, without entered-source admission.
    let source = "\u{feff}set x [foo bar\nputs done\n";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    for supplied in [None, Some(input.clone())] {
        let mut analyser = supplied.as_ref().map_or_else(Analyser::new, |input| {
            Analyser::new().with_resolved_input(input.clone())
        });
        analyser.analyse_and_retain_result_for_test(source, "tcl9.1");
        let mut commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.file_lexer_config(),
        );
        assert!(analyser.apply_ghost_recovery(source, &mut commands));
        assert_eq!(
            commands
                .iter()
                .map(crate::segmenter::SegmentedCommand::name)
                .collect::<Vec<_>>(),
            ["set", "puts"]
        );
        assert_eq!(commands[0].argv[0].span.start(), 3);
        assert_eq!(analyser.result.resolved_input.as_ref(), Some(&input));
    }
    for (source, kind, code) in [
        (
            "\u{feff}set x {abc}def\n",
            tcl_lexer::ParseIssueKind::ExtraAfterBrace,
            DiagCode::E204,
        ),
        (
            "\u{feff}set y \"abc\"def\n",
            tcl_lexer::ParseIssueKind::ExtraAfterQuote,
            DiagCode::E205,
        ),
        (
            "\u{feff}puts ${foo\n",
            tcl_lexer::ParseIssueKind::MissingVariableBrace,
            DiagCode::E206,
        ),
    ] {
        let config = input.lexer_config();
        let (_, warnings) = tcl_lexer::Lexer::with_config(source, config)
            .tokenise_all_with_warnings()
            .unwrap();
        let warning = warnings
            .iter()
            .find(|warning| warning.kind == kind)
            .unwrap();
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "display-only");
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap();
        assert_eq!(diagnostic.message, warning.message);
        assert_eq!(diagnostic.span, Span::new(warning.offset, warning.offset));
    }
}

#[test]
fn original_prepared_input_records_the_file_boundary_and_preserves_supplied_config() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original editing inputs; no runtime source or Native entry receipt.
    let source = "\u{feff}set name value";
    let mut default = Analyser::new();
    let prepared = default.prepare_analysis_input("tcl9.1").unwrap();
    assert_eq!(prepared.lexer_config(), default.file_lexer_config());
    assert_eq!(
        prepared.lexer_config().leading_bom,
        tcl_lexer::LeadingBom::Skip
    );
    assert_ne!(prepared.lexer_config(), default.lexer_config());
    let ordinary = Analyser::new().analyse(source, "tcl9.1");
    let retained = Analyser::new()
        .with_resolved_input(prepared.clone())
        .analyse(source, "presentation-only");
    assert_eq!(ordinary.resolved_input.as_ref(), Some(&prepared));
    assert_eq!(ordinary.body_lexer_config, Some(prepared.lexer_config()));
    assert!(ordinary.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        prepared.lexer_config(),
    ));
    assert!(retained.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        prepared.lexer_config(),
    ));
    assert_eq!(project_result(&ordinary), project_result(&retained));
    let command = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        prepared.lexer_config(),
    )
    .pop()
    .unwrap();
    assert_eq!(command.texts[0], "set");

    let environment = crate::environment_ingress::resolve_environment("tcl9.1");
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Content,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(environment.grammar())
    };
    let supplied = super::super::input::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        environment.default_context_registry(),
        config,
    );
    let mut selected = Analyser::new().with_resolved_input(supplied.clone());
    assert_eq!(selected.prepare_analysis_input("tcl8.4").unwrap(), supplied);
    let result = selected.analyse(source, "tcl8.4");
    assert_eq!(result.resolved_input.as_ref(), Some(&supplied));
    assert_eq!(result.body_lexer_config, Some(config));
    let command = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        result.resolved_input.as_ref().unwrap().lexer_config(),
    )
    .pop()
    .unwrap();
    assert_eq!(command.texts[0], "\u{feff}set");
}

#[test]
fn original_incremental_completeness_retains_the_supplied_parser_configuration() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Incremental source analysis must converge; this grants no entry.
    let environment = crate::environment_ingress::resolve_environment("tcl9.1");
    let previous = "puts before";
    let source = "puts \"${a{}\"";
    for braced_var in [
        tcl_lexer::BracedVarStyle::FirstClose,
        tcl_lexer::BracedVarStyle::Tcl9Nesting,
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var,
            ..tcl_lexer::LexerConfig::for_file_grammar(environment.grammar())
        };
        let input = super::super::input::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            environment.default_context_registry(),
            config,
        );
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(previous, 0, config);
        let result = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse_incremental(previous, &commands, source, "tcl9.1");
        let fresh = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl9.1");
        assert_eq!(result.resolved_input.as_ref(), Some(&input));
        assert_eq!(project_result(&result), project_result(&fresh));
    }
}

#[test]
fn retained_actual_input_does_not_borrow_a_presentation_overlay() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl");
    let input = super::super::input::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::default().with_grammar(environment.grammar()),
    );
    let mut analyser = Analyser::new()
        .with_resolved_input(input.clone())
        .with_pack_overlay(u64::MAX);
    let result = analyser.analyse("proc retained {} {}", "tcl9.1");
    assert!(result.analysis_context_unavailable.is_none());
    assert_eq!(result.resolved_input.as_ref(), Some(&input));
    assert!(result.all_procs.contains_key("::retained"));
}

#[test]
fn new_analyser_starts_at_global_scope_with_empty_state() {
    let a = Analyser::new();
    assert_eq!(a.result.global_scope.kind, ScopeKind::Global);
    assert_eq!(a.current_scope_path, [] as [usize; 0]);
    assert_eq!(&*a.source, "");
    assert_eq!(a.disabled_diagnostics.len(), 0);
    assert_eq!(a.conditional_depth, 0);
    assert_eq!(a.body_depth, 0);
    assert_eq!(a.last_comment, "");
    assert!(a.file_path.is_none());
    assert_eq!(a.command_aliases.len(), 0);
    assert_eq!(
        a.var_command_sites,
        [] as [crate::analyser::state::VarCommandSite; 0]
    );
    assert_eq!(
        a.cmd_command_sites,
        [] as [crate::analyser::state::CmdCommandSite; 0]
    );
    assert_eq!(a.const_strings.len(), 0);
    assert_eq!(a.regex_vars.len(), 0);
    assert!(a.builtin_names.is_none());
    assert!(a.builtin_dialect.is_none());
    assert!(a.current_event.is_none());
    assert_eq!(a.ensemble_namespaces.len(), 0);
    assert_eq!(a.objdefined_vars.len(), 0);
    assert!(!a.unresolved_commands_emitted);
}

#[test]
fn with_disabled_diagnostics_threads_through() {
    let disabled: HashSet<String> = ["W210", "W211"].iter().map(|s| (*s).to_string()).collect();
    let a = Analyser::with_disabled_diagnostics(disabled.clone());
    assert_eq!(a.disabled_diagnostics, disabled);
}

#[test]
fn analyse_records_top_level_proc() {
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set x 1 }", "tcl");
    assert!(r.all_procs.contains_key("::foo"));
}

#[test]
fn apply_body_proc_registers_in_global_namespace_not_caller() {
    // `apply`'s body runs in the *global* namespace (no lambda namespace
    // given), NOT the caller's — so a nested `proc` registers as `::helper`,
    // never `::foo::helper` (Tcl `apply` manual; matches the IR lowering).
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval foo { apply {{} { proc helper {} { return 1 } }} }",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::helper"),
        "apply body proc must register globally: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs.contains_key("::foo::helper"),
        "apply body proc must NOT inherit the caller namespace: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// TP — the tcllib pki idiom: `proc [namespace current]::_x {...}` inside
/// `::pki`.  The body's own definitions must home to `::pki`, not to a
/// phantom namespace made out of the substitution's source text.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, probe `s5_probe.tcl`):
///
/// ```text
/// inner exists: ::pki::_inner
/// helper homed at: ::pki::helper / global:
/// ```
#[test]
fn a_dynamically_named_procs_body_homes_to_the_lexical_namespace() {
    let src = "namespace eval ::pki {\n\
                       proc _outer {} {\n\
                           proc [namespace current]::_inner {} {\n\
                               proc helper {} { return HELPED }\n\
                           }\n\
                       }\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.all_procs.contains_key("::pki::helper"),
        "the body of a `[namespace current]`-named proc runs in ::pki: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs
            .keys()
            .any(|k| k.contains("[namespace current]") && k.ends_with("helper")),
        "no phantom substitution-text namespace may appear: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// TN — a statically-named proc is untouched: the scope name is the
/// resolved name verbatim, so an ordinary qualified-name proc still homes
/// its nested definitions to its own defining namespace.
#[test]
fn a_statically_named_procs_body_still_homes_to_its_defining_namespace() {
    let src = "proc ::pki::_outer {} { proc helper {} {} }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.all_procs.contains_key("::pki::helper"),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// A `$`-spelled dynamic name (`proc _$n {...}`) gives its body no proven
/// namespace: a value with `::` names a child namespace, and the body runs
/// there. tclsh 8.4.20, 8.6 and 9.0 with `suffix` `x` then `a::b` define
/// `::pki::helper` and `::pki::_a::helper`, so no single home is reported.
#[test]
fn a_dollar_named_procs_body_has_no_proven_home() {
    let src = "namespace eval ::pki { proc _$suffix {} { proc helper {} {} } }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.all_procs.keys().any(|name| name.ends_with("::helper")),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn apply_body_proc_honours_explicit_lambda_namespace() {
    // A lambda pinning a namespace (element 2) runs its body there. The
    // namespace exists first (tclsh 8.6/9.0: `namespace "::bar" not found`).
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::bar {}\napply {{} { proc helper {} { return 1 } } ::bar}",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::bar::helper"),
        "explicit lambda namespace must be honoured: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// TP — a *relative* (non-`::`-prefixed) lambda namespace element is
/// interpreted against the GLOBAL namespace, never the caller's.
///
/// `doc/apply.n`: "If given, namespace is interpreted relative to the
/// global namespace even if its name does not start with `::`";
/// `tclProc.c`'s `TclNRApplyObjCmd` `::`-prefixes the word before the
/// lookup. tclsh 9.0.4 oracle (probe `a2b_apply.tcl`), with `::sub`,
/// `::pin::sub`, and `::lex::sub` all defined:
///
/// ```text
/// lex-body current=::lex -> ns=::sub who=GLOBAL-SUB
/// proc current=::pin     -> ns=::sub who=GLOBAL-SUB
/// c2 current=::pin       -> ns=::sub who=GLOBAL-SUB
/// ```
#[test]
fn apply_relative_lambda_namespace_homes_globally_not_to_the_caller() {
    let src = "namespace eval ::sub {}\n\
                   namespace eval ::caller::sub {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } } sub}\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::sub::helper"),
        "relative lambda ns must resolve against ::, not the caller: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs.contains_key("::caller::sub::helper"),
        "relative lambda ns must NOT pin against the caller namespace: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    let (_, ns) = r
        .namespace_overrides
        .first()
        .expect("the lambda body records a namespace override");
    assert_eq!(ns, "::sub");
}

/// TN — an *absolute* namespace element is unaffected by the
/// global-relative rule: `::caller::sub` stays `::caller::sub`.
#[test]
fn apply_absolute_lambda_namespace_is_unchanged() {
    let src = "namespace eval ::sub {}\n\
                   namespace eval ::caller::sub {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } } ::caller::sub}\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::caller::sub::helper"),
        "absolute lambda ns must be honoured verbatim: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// Control — no namespace element at all still means the global
/// namespace, from inside a qualified-name proc as much as anywhere.
#[test]
fn apply_without_a_namespace_element_stays_global_inside_a_qualified_proc() {
    // `::caller` exists first (tclsh 8.6/9.0: `can't create procedure
    // "::caller::p": unknown namespace`).
    let src = "namespace eval ::caller {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::helper"),
        "an absent lambda ns means global: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn apply_with_namespace_element_records_a_namespace_override() {
    // TP (the core mechanism): a literal
    // `apply {{params} body ns}` must record a namespace_overrides
    // entry spanning the body, so `tcl-lsp-core`'s command-resolution
    // lookups can pin bareword calls inside the body to `ns` — the
    // `Scope` subtree `reconstruct_proc_scope` builds for `ns` is
    // rooted under body_span-less wrapper nodes the ordinary lexical
    // walk can never reach.
    let mut a = Analyser::new();
    let src = "apply {{} { cleanup done } ::real}";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(
        r.namespace_overrides.len(),
        1,
        "{:?}",
        r.namespace_overrides
    );
    let (span, ns) = &r.namespace_overrides[0];
    assert_eq!(ns, "::real");
    // The recorded span is the body token's raw span (opening brace
    // included, per this codebase's lexer-span convention — the
    // closing brace is excluded, one byte short of the token's end).
    assert_eq!(
        &src[span.start() as usize..span.end() as usize],
        "{ cleanup done "
    );
}

#[test]
fn apply_without_namespace_element_does_not_record_an_override() {
    // TN — a 2-element lambda (no namespace argument) defaults to
    // global, which `command_resolution_namespace_at` already reports
    // with no override present; pushing a `(span, "::")` entry would
    // be a no-op relative to today's behaviour, but the plan
    // deliberately still records it (uniform code path) — confirm the
    // recorded namespace is exactly `"::"`, not skipped or wrong.
    let mut a = Analyser::new();
    let r = a.analyse("apply {{} { cleanup done }}", "tcl8.6");
    assert_eq!(
        r.namespace_overrides.len(),
        1,
        "{:?}",
        r.namespace_overrides
    );
    assert_eq!(r.namespace_overrides[0].1, "::");
}

#[test]
fn unknown_lexer_warning_maps_to_e200() {
    // An unterminated `$arr(idx` array index makes the lexer emit a
    // "missing )" warning, which has no dedicated recovery code — it
    // must surface as the catch-all E200 (not be silently dropped).
    let mut a = Analyser::new();
    let r = a.analyse("set x $arr(idx\n", "tcl8.6");
    let e200 = r.diagnostics.iter().find(|d| d.code == DiagCode::E200);
    assert!(
        e200.is_some_and(|d| d.message == "missing )"),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_records_multiple_top_level_commands() {
    let mut a = Analyser::new();
    let r = a.analyse("set x 1\nproc foo {} {}\nglobal a b", "tcl");
    assert!(r.all_procs.contains_key("::foo"));
    assert!(r.global_scope.variables.contains_key("x"));
    assert!(r.global_scope.variables.contains_key("a"));
    assert!(r.global_scope.variables.contains_key("b"));
}

#[test]
fn analyse_records_instance_class_set_new() {
    // `set d [Dog new]` maps `d` -> `::Dog`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nset d [Dog new]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("d").map(String::as_str),
        Some("::Dog")
    );
}

/// A rooted `::set d [Dog new]` binds `d` as `set` does: the
/// instance tracking reads `set`'s handle-binding layout, which the
/// rooted spelling resolves to, rather than comparing the spelling `set`
/// (tclsh 8.6.18 to 9.1b0: `$d bark` dispatches to `::Dog`).
#[test]
fn analyse_records_instance_class_rooted_set_new() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\n::set d [Dog new]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("d").map(String::as_str),
        Some("::Dog")
    );
}

#[test]
fn analyse_records_instance_class_create_named() {
    // `Dog create rex` maps `rex` -> `::Dog`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nDog create rex\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("rex").map(String::as_str),
        Some("::Dog"),
    );
}

#[test]
fn analyse_records_instance_class_set_create() {
    // `set d [Dog create rex]` maps `d` -> `::Dog`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog {}\nset d [Dog create rex]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("d").map(String::as_str),
        Some("::Dog")
    );
}

#[test]
fn analyse_records_instance_class_through_a_var_headed_create() {
    // TP: tcllib's `httpd/httpd.tcl` flows the
    // constructor's class name through a single, unconditional `set`
    // one line earlier (`set class ::Derived; set obj [$class create
    // NAME]`) rather than writing the class as a literal bareword.
    // Verified against real tclsh9.0/8.6: `$obj`'s methods dispatch to
    // `Dog` either way, so the analyser must bind `obj` -> `::Dog`
    // exactly like the literal-bareword shape already does.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nset class Dog\nset obj [$class create rex]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("obj").map(String::as_str),
        Some("::Dog"),
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_records_instance_class_through_a_var_headed_new() {
    // TP — same gap, the `new` constructor spelling.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nset class Dog\nset obj [$class new]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("obj").map(String::as_str),
        Some("::Dog"),
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_rejects_a_var_headed_non_manufacturer_word() {
    // FP guard: the shape parser accepts any literal method so that the
    // registry can decide after `$class` resolves. A normal method must
    // not become a constructor merely because it follows a class handle.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Dog { method inspect {} { return value } }\nset class Dog\nset obj [$class inspect]\n",
            "tcl",
        );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_rejects_unexported_create_with_namespace_dispatch() {
    // TN/FP guard pinned against tclsh 9.0.4: `Dog
    // createWithNamespace x ::xns` fails with TCL LOOKUP METHOD. The
    // registry retains its structural layout for self-dispatch, but an
    // ordinary `$class` command cannot use it as construction evidence.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog {}\nset class Dog\nset obj [$class createWithNamespace x ::xns]\n",
        "tcl",
    );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_abstains_on_a_branch_ambiguous_class_var() {
    // TN — a class variable whose reaching definitions genuinely
    // disagree (one arm `Dog`, the other `Cat`) is unprovable at the
    // constructor call: binding *either* class would be a guess, so
    // the analyser must abstain (no `instance_classes` entry) rather
    // than pick one arbitrarily.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Dog {}\noo::class create Cat {}\nif {$flag} { set class Dog } else { set class Cat }\nset obj [$class create x]\n",
            "tcl",
        );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_abstains_on_a_genuinely_dynamic_class_var() {
    // TN — a class variable fed by a computed value (no exact
    // constant) can't be proven at all; must abstain exactly like the
    // pre-fix behaviour rather than binding a wrong/empty class.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog {}\nset class [someFactory]\nset obj [$class create x]\n",
        "tcl",
    );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_does_not_record_class_definition_as_instance() {
    // `oo::class create Dog` defines a class, not an
    // instance — `Dog` must not appear in instance_classes.
    let mut a = Analyser::new();
    let r = a.analyse("oo::class create Dog {}\n", "tcl");
    assert!(
        !r.instance_classes.contains_key("Dog"),
        "class definition leaked into instance_classes: {:?}",
        r.instance_classes,
    );
}

#[test]
fn analyse_ignores_instance_of_unknown_class() {
    // `set d [Widget new]` where Widget isn't a user class
    // records nothing.
    let mut a = Analyser::new();
    let r = a.analyse("set d [Widget new]\n", "tcl");
    assert_eq!(r.instance_classes.len(), 0, "{:?}", r.instance_classes);
}

#[test]
fn analyse_namespace_eval_opens_scope() {
    let mut a = Analyser::new();
    let r = a.analyse("namespace eval ns1 { }", "tcl");
    assert_eq!(r.global_scope.children.len(), 1);
    assert_eq!(r.global_scope.children[0].name, "ns1");
}

#[test]
fn analyse_empty_source_is_empty_result() {
    let mut a = Analyser::new();
    let r = a.analyse("", "tcl");
    assert_eq!(r.all_procs.len(), 0);
    assert_eq!(r.diagnostics, [] as [crate::analyser::types::Diagnostic; 0]);
}

#[test]
fn analyse_threads_file_suppression_into_disabled_diagnostics() {
    // ``# tcl-lsp: disable=W210,W211`` at the top of the file
    // must merge into ``self.disabled_diagnostics`` so
    // emitters honour the suppression.
    let mut a = Analyser::new();
    let _ = a.analyse("# tcl-lsp: disable=W210,W211\nproc foo {} {}\n", "tcl");
    assert!(a.disabled_diagnostics.contains("W210"));
    assert!(a.disabled_diagnostics.contains("W211"));
}

#[test]
fn original_next_line_noqa_matches_all_actual_analyser_ingresses() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {script:body}\n# tcl-lsp: stubs-end\nhold {\n# noqa: W210\n# comment-only\n}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let full = Analyser::new().analyse(source, profile.name);
    let (chunked, _) =
        Analyser::new().analyse_chunked(source, vec![commands.clone()], profile.name);
    let snapshot = Analyser::new().analyse_commands(source, &commands, profile.name, true);
    let isolated = Analyser::new().analyse_per_item(source, profile.name);
    assert!(full.suppressed_lines.get(&5).unwrap().contains("W210"));
    for result in [chunked, snapshot, isolated] {
        assert_eq!(result.suppressed_lines, full.suppressed_lines);
    }
}

#[test]
fn analyse_threads_next_line_noqa_into_suppressed_lines() {
    // A ``# noqa`` comment on its own line must seed
    // ``suppressed_lines`` for the *following* line via the
    // ``parse_noqa_line_suppressions`` pre-scan.  Line 0 carries
    // the ``# noqa`` directive so line 1 should be in the map.
    let mut a = Analyser::new();
    let r = a.analyse("# noqa\nset x 1\n", "tcl");
    let codes = r.suppressed_lines.get(&1).expect("line 1 entry");
    assert!(codes.contains("*"));
}

#[test]
fn analyse_chunked_threads_next_line_noqa_into_suppressed_lines() {
    // Same wiring through ``analyse_chunked`` — the LSP's
    // primary incremental entry point.
    use crate::segmenter::SegmentedCommand;
    let mut a = Analyser::new();
    let cmds: Vec<Vec<SegmentedCommand>> = vec![Vec::new()];
    let (r, _) = a.analyse_chunked("# noqa: W210\nset x 1\n", cmds, "tcl");
    let codes = r.suppressed_lines.get(&1).expect("line 1 entry");
    assert!(codes.contains("W210"));
}

#[test]
fn analyse_commands_threads_next_line_noqa_into_suppressed_lines() {
    // Same wiring through ``analyse_commands`` — the snapshot-
    // restore entry point.
    use crate::segmenter::SegmentedCommand;
    let mut a = Analyser::new();
    let cmds: Vec<SegmentedCommand> = Vec::new();
    let r = a.analyse_commands("# noqa: W210\nset x 1\n", &cmds, "tcl", true);
    let codes = r.suppressed_lines.get(&1).expect("line 1 entry");
    assert!(codes.contains("W210"));
}

#[test]
fn analyse_file_suppression_unions_with_constructor_codes() {
    // Constructor-provided codes must survive file-suppression
    // merging — the two sources are unioned, not replaced.
    use std::collections::HashSet;
    let preconfigured: HashSet<String> = ["W120"].iter().map(|s| (*s).to_string()).collect();
    let mut a = Analyser::with_disabled_diagnostics(preconfigured);
    let _ = a.analyse("# tcl-lsp: disable=W210\n", "tcl");
    assert!(a.disabled_diagnostics.contains("W120"));
    assert!(a.disabled_diagnostics.contains("W210"));
}

#[test]
fn analyse_runs_dedupe_and_disabled_filter_at_end() {
    // End-to-end: ``proc set {} {}`` emits W113.
    // ``# tcl-lsp: disable=W113`` at the top of the source
    // should silence it via ``apply_disabled_diagnostics``.
    let mut a = Analyser::new();
    let r = a.analyse("# tcl-lsp: disable=W113\nproc set {} {}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
        "W113 should be silenced by file-suppression directive",
    );
}

#[test]
fn w113_silent_on_overridable_library_procs() {
    // FP-STY-13: the script-defined Tcl *library* procedures
    // (`unknown`, `history`, `auto_*`, `parray`, `pkg_mkIndex`,
    // `tcl_*` word helpers) are documented as user-replaceable
    // overlays, not C built-ins — redefining one must not fire
    // W113.
    let mut a = Analyser::new();
    for name in [
        "unknown",
        "history",
        "auto_execok",
        "auto_load",
        "auto_mkindex",
        "auto_qualify",
        "auto_reset",
        "parray",
        "pkg_mkIndex",
        "tcl_findLibrary",
        "tcl_wordBreakAfter",
        "tcl_endOfWord",
    ] {
        let r = a.analyse(&format!("proc {name} args {{ return }}\n"), "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
            "W113 should be silent on overridable library proc {name:?}",
        );
    }
    // TP control: a genuine C built-in still fires.
    let r = a.analyse("proc set {} {}\n", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
        "W113 must still fire when redefining a C built-in (`set`)",
    );
}

#[test]
fn w307_array_set_const_element_suppression() {
    // TN: `array set state {-command puts}; $state(-command) hi` — the
    // literal element value `puts` is a known command, so the callback-key
    // heuristic must not fire W307. Harvested from the `array set` literal.
    let mut a = Analyser::new();
    let has_w307 = a
        .analyse(
            "proc f {} { array set state {-command puts}; $state(-command) hi }\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        !has_w307,
        "array-element holding a known command must not fire W307",
    );
    // TP control: a non-command literal value still fires W307.
    let mut b = Analyser::new();
    let has_w307 = b
        .analyse(
            "proc f {} { array set state {-command notACommand}; $state(-command) hi }\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        has_w307,
        "array-element holding a non-command must still fire W307",
    );
}

/// Helper: the RBS-family diagnostic codes (W210 read-before-set, W213
/// unset-no-complain, W214 unused-param) emitted for `src`.
fn rbs_codes(src: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W210" | "W213" | "W214"))
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn w220_must_alias_kill_same_array_element() {
    // FP-DS-06: two writes to the *same* literal-key array element with no
    // intervening read of it make the first dead — the later-version read
    // `$a(k)` reads the second write, not the first.  Must-alias kill
    // overrides the element-observed suppression.
    let mut a = Analyser::new();
    assert!(
        a.analyse(
            "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "two writes to a(k) with no intervening read: first is dead",
    );
    // Control: different keys are independent — no dead store.
    let mut b = Analyser::new();
    assert!(
        !b.analyse(
            "proc f {} {\n    set a(k) 1\n    set a(j) 2\n    return $a(k)\n}",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "writes to different array elements are not dead stores",
    );
    // Control: an intervening read of the same element cancels the kill.
    let mut c = Analyser::new();
    assert!(
        !c.analyse(
            "proc f {} {\n    set a(k) 1\n    puts $a(k)\n    set a(k) 2\n    return $a(k)\n}",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "an intervening read of a(k) keeps the first write live",
    );
}

#[test]
fn dead_store_recovers_rmw_read_in_command_sub() {
    // FP-DS-01: `[incr i $j]` buried in a substitution reads `i`'s prior
    // value, so the feeding `set i 0` is alive — no dead-store (W220/O109)
    // or unused (W211/O126).
    let mut a = Analyser::new();
    let codes: Vec<_> = a
            .analyse(
                "proc f {} {\n    set i 0\n    foreach j {1 2 3} { lappend r [incr i $j] }\n    return $r\n}\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W220" | "W211" | "O109" | "O126"))
            .map(|d| d.code.to_string())
            .collect();
    assert_eq!(
        codes.len(),
        0,
        "RMW read in cmd-sub keeps set i 0 alive: {codes:?}"
    );
    // TP control: no read-modify-write of `i` — the first assignment is
    // truly dead, so a dead-store hint must still fire.
    let mut b = Analyser::new();
    assert!(
        b.analyse("proc f {} { set i 0\n set i 5\n return $i }", "tcl")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "a genuine dead store must still fire",
    );
}

#[test]
fn w210_dynamic_target_upvar_alias_read_is_silent() {
    // `upvar 1 $varName local` aliases `local` to the caller
    // variable named by `$varName` — the standard Tcl pass-by-reference
    // idiom. Reading `$local` is not read-before-set: it errors only when
    // the *caller* variable is missing, exactly the runtime condition that
    // would make the *literal*-target twin (`upvar 1 caller local`) error
    // too. tclsh 8.6/9.0 confirm the two forms are semantically identical,
    // so the analyser treats them alike — both silent.
    let codes = rbs_codes("proc foo {varName} {\n  upvar 1 $varName local\n  puts $local\n}\n");
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "dynamic-target upvar alias read must be silent: {codes:?}"
    );
    // The literal-target twin is silent too.
    let codes = rbs_codes("proc foo {} {\n  upvar 1 caller local\n  puts $local\n}\n");
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "literal-target upvar: {codes:?}"
    );
    // TP control: an *unrelated* local — not the alias, never set — is not a
    // scope alias, so it still fires W210.
    let codes = rbs_codes("proc foo {varName} {\n  upvar 1 $varName local\n  puts $unrelated\n}\n");
    assert!(
        codes.iter().any(|c| c == "W210"),
        "unrelated non-alias local must still fire W210: {codes:?}"
    );
    // An `[info exists local]` guard suppresses the read (always did).
    let codes = rbs_codes(
        "proc foo {varName} {\n  upvar 1 $varName local\n  if {[info exists local]} { puts $local }\n}\n",
    );
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "guarded dynamic upvar: {codes:?}"
    );
    // A *write* through the alias is observable (goes to the caller var),
    // so it is not a dead store even with a dynamic target.
    let mut a = Analyser::new();
    assert!(
        !a.analyse(
            "proc foo {v} {\n  upvar 1 $v local\n  set local 5\n}\n",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220),
        "write through a dynamic upvar alias is observable, not a dead store",
    );
}

#[test]
fn w210_gets_in_while_condition_writes_loop_var() {
    // FP-RBS-03: `[gets $fp line]` in the while condition writes `line`;
    // the body's `$line` reads must not fire W210.
    let codes = rbs_codes(
        "proc f {fp} {\n  while {[gets $fp line] >= 0} {\n    set n [string length $line]\n    puts \"$line ($n chars)\"\n  }\n}\n",
    );
    assert_eq!(codes.len(), 0, "gets-in-condition writes line: {codes:?}");
}

#[test]
fn w210_catch_in_expr_writes_result_var() {
    // FP-RBS-06: `[catch {…} tmp]` inside `[expr {…}]` writes `tmp`; the
    // same-expression `|| $tmp` read must not fire W210.
    let codes = rbs_codes(
        "proc f {sock} {\n  set eof [expr {[catch {eof $sock} tmp] || $tmp}]\n  return $eof\n}\n",
    );
    assert_eq!(codes.len(), 0, "catch-in-expr writes tmp: {codes:?}");
}

#[test]
fn w214_namespace_eval_body_does_not_recover_caller_param() {
    // FP-RBS-10: a `namespace eval ::ns {…}` body runs in the namespace
    // frame, so `$x` there is NOT a read of the caller's parameter `x` —
    // W214 (unused param) must still fire.  The `eval {…}` form runs in
    // the caller frame, so its `$x` read DOES recover the param (no W214).
    let mut a = Analyser::new();
    assert!(
        a.analyse(
            "proc g {x} {\n  namespace eval ::ns { puts \"hello $x\" }\n}\n",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W214),
        "namespace-eval body must not recover the caller's param read",
    );
    let mut b = Analyser::new();
    assert!(
        !b.analyse("proc f {x} {\n  eval { puts $x }\n}\n", "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214),
        "eval body runs in the caller frame; $x read recovers the param",
    );
}

#[test]
fn w307_in_method_body_dispatch_fires_and_suppresses() {
    // TclOO method bodies are now walked in a `Method` scope, so their
    // `[cmd] method` dispatch sites are recorded for W307.
    // TP: `[format notACommand] run` returns a string, not an object.
    let mut a = Analyser::new();
    let fires = a
        .analyse(
            "oo::class create C { method m {} { [format notACommand] run } }",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(fires, "in-method [format ...] dispatch must fire W307");

    // Control: `[D new] run` where D is a known class returns an Object —
    // suppressed (and `run` resolves on D, so no W308 either).
    let mut b = Analyser::new();
    let fires = b
        .analyse(
            "oo::class create D { method run {} { return ok } }\n\
                 oo::class create C { method m {} { [D new] run } }\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        !fires,
        "in-method [D new] dispatch on known class must not fire W307"
    );
}

#[test]
fn w307_my_method_literal_vs_object_return() {
    // TP: `[my plain] run` where `plain` returns a literal — the return is
    // a plain string, so the outer dispatch fires W307.
    let mut a = Analyser::new();
    let fires = a
        .analyse(
            "oo::class create C { method plain {} { return notACommand }\n\
                 method m {} { [my plain] run } }",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(fires, "[my plain] returning a literal must fire W307");

    // Control: `[my obj] run` where `obj` returns `[D new]` — object
    // handle, suppressed.
    let mut b = Analyser::new();
    let fires = b
        .analyse(
            "oo::class create D { method run {} { return ok } }\n\
                 oo::class create C { method obj {} { return [D new] }\n\
                 method m {} { [my obj] run } }",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(!fires, "[my obj] returning an object must not fire W307");
}

#[test]
fn method_body_params_and_instance_vars_not_flagged() {
    // Walking method bodies must not false-fire read-before-set / unused
    // on the method's formal parameters or the class's instance variables.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create C {\n\
             variable count\n\
             method add {n} { incr count $n; return $count }\n\
             }\n",
        "tcl",
    );
    let offenders: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W210" | "W214"))
        .map(|d| d.code.to_string())
        .collect();
    assert_eq!(
        offenders.len(),
        0,
        "method params / instance vars must not fire W210/W214: {offenders:?}",
    );
}

#[test]
fn w307_namespaced_ensemble_composed_name_resolution() {
    // TN: `${ns}::dowork` where `ns` is the const `mypkg` and
    // `::mypkg::dowork` is a known proc — the composed name resolves, so
    // no W307.
    let mut a = Analyser::new();
    let has = a
        .analyse(
            "namespace eval ::mypkg { proc dowork {arg} {} }\n\
                 proc f {} { set ns mypkg; ${ns}::dowork arg }\n",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(!has, "resolvable namespaced ensemble must not fire W307");
    // TP control: composed name with no known proc still fires.
    let mut b = Analyser::new();
    let has = b
        .analyse(
            "proc f {} { set ns mypkg; ${ns}::unknownproc arg }\n",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        has,
        "unresolvable composed namespaced ensemble must fire W307"
    );
}

#[test]
fn w307_interproc_dict_with_const_element_suppression() {
    // TN: `dict with d { $cmd hi }` where the caller passes `{cmd puts}` —
    // the dict-with unpacks `cmd` to the known command `puts`, so the
    // body's `$cmd hi` dispatch must not fire W307.  The literal is
    // harvested from `d`'s call-site-propagated v0 SCCP const.
    let mut a = Analyser::new();
    let has_w307 = a
        .analyse(
            "proc f {d} { dict with d { $cmd hi } }\nf {cmd puts}\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        !has_w307,
        "dict-with-unpacked known command must not fire W307"
    );
    // TP control: a non-command unpacked value still fires.
    let mut b = Analyser::new();
    let has_w307 = b
        .analyse(
            "proc f {d} { dict with d { $cmd hi } }\nf {cmd notACommand}\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        has_w307,
        "dict-with-unpacked non-command must still fire W307"
    );
}

#[test]
fn w307_known_class_new_var_suppression() {
    // TN: `set x [C new]; $x run` where C is a known oo::class — `x` holds
    // an Object of class C, so the `$x run` dispatch resolves through the
    // method check (run exists on C) instead of firing W307.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create C { method run {} { return ok } }\n\
             proc f {} { set x [C new]; $x run }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "var assigned from a known-class constructor must not fire W307: {:?}",
        r.diagnostics,
    );
}

#[test]
fn w216_brace_then_paren_name_vs_value_positions() {
    // FP-STY-12: the braced indirect-array idiom `${name}(idx)` in a
    // *variable-name* position (`set`/`unset`/`incr`/`append`/`lappend`/
    // `vwait`/`info exists`) is the legitimate "array name held in a
    // scalar" access — neither W216 nor W212 fires.
    let mut a = Analyser::new();
    for src in [
        "set token ::http::1\nset ${token}(status) eof\n",
        "info exists ${token}(-pipeline)\n",
        "unset ${tok}(socketcoro)\n",
        "vwait ${token}(status)\n",
        "incr ${arr}(n)\n",
        "append ${arr}(buf) x\n",
        "lappend ${arr}(list) item\n",
    ] {
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W216 || d.code == DiagCode::W212),
            "indirect-array idiom must not fire W216/W212: {src:?} -> {:?}",
            r.diagnostics,
        );
    }
    // TP control: in a *value* position `${arr}(x)` is a broken read for
    // `$arr(x)` — W216 still fires.
    for src in ["puts ${arr}(x)\n", "set y ${arr}(x)\n"] {
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W216),
            "value-position ${{arr}}(x) must fire W216: {src:?} -> {:?}",
            r.diagnostics,
        );
    }
    // TP control: bare `set $x` / `set ${x}` (no `(idx)` suffix) is the
    // dynamic-name foot-gun, not the indirect idiom — W212 still fires.
    for src in ["set $x v\n", "set ${x} v\n"] {
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W212),
            "bare dynamic-name must still fire W212: {src:?} -> {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn analyse_dedupes_back_to_back_identical_diagnostics() {
    // Two identical W113 emissions for the same proc name
    // should collapse to one.
    let mut a = Analyser::new();
    let r = a.analyse("proc set {} {}\nproc set {} {}\n", "tcl");
    // Re-defining ``set`` twice means handle_proc emits W113
    // twice — but the second emission is at a *different*
    // span (different proc-name token), so dedupe leaves
    // them both; the test that follows pins the actual count.
    let w113s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W113)
        .collect();
    assert_eq!(
        w113s.len(),
        2,
        "two distinct ``proc set`` definitions → two distinct W113s at different spans",
    );
}

#[test]
fn analyse_records_source_text_for_handler_re_slicing() {
    // Handlers re-slice ``self.source`` via spans returned by the
    // segmenter; the field must be populated at the top of
    // ``analyse``.
    let mut a = Analyser::new();
    let _ = a.analyse("set x 1", "tcl");
    assert_eq!(&*a.source, "set x 1");
}

#[test]
fn analyse_records_dialect_for_w113_and_emitter_use() {
    // Handlers (W113 shadow check, dialect-only emitters) read
    // ``self.dialect()`` directly.  The field must be populated at
    // the top of ``analyse``.
    let mut a = Analyser::new();
    let _ = a.analyse("", "f5-irules");
    assert_eq!(a.dialect(), "f5-irules");
    assert!(a.profile.is_irules());
}

#[test]
fn builtin_command_names_caches_per_dialect() {
    // First lookup populates the cache; subsequent lookups
    // with the same dialect return the same set.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::plain_tcl();
    // ``set`` is a core built-in across all dialects.
    assert!(a.builtin_command_names().contains("set"));
    // The set is the one `exists` oracle's registry tier — the names the
    // resolved context actually provides, not the unfiltered store name
    // set. Cache
    // invalidation: switching dialect rebuilds onto the new context.
    a.profile = tcl_dialect::DialectProfile::irules();
    assert!(
        a.builtin_command_names().contains("HTTP::header"),
        "the iRules surface is provided under f5-irules"
    );
    assert!(
        !a.builtin_command_names().contains("exec"),
        "a compiler-disabled, interpreter-absent core command does not \
             exist under the closed iRules world (R-c; pre-P1a the unfiltered \
             store set believed in it)"
    );
    assert!(
        a.builtin_command_names().contains("vwait"),
        "a §4b interpreter-present (compiler-refused) builtin still exists \
             for the oracle"
    );
}

#[test]
fn analyse_commands_pre_segmented_records_proc() {
    // ``analyse_commands`` is the incremental entry — same
    // dispatcher as ``analyse``, but without re-segmentation.
    // Smoke-test that a pre-segmented chunk records its proc.
    use crate::segmenter::segment_commands;
    let source = "proc foo {} {}";
    let commands = segment_commands(source);
    let mut a = Analyser::new();
    let r = a.analyse_commands(source, &commands, "tcl", true);
    assert!(r.all_procs.contains_key("::foo"));
}

#[test]
fn analyse_commands_fires_stray_close_bracket() {
    // The incremental / chunked path runs the E100 stray-`]`
    // check too, matching `analyse`.
    use crate::segmenter::segment_commands;
    let source = "puts foo]";
    let commands = segment_commands(source);
    let mut a = Analyser::new();
    let r = a.analyse_commands(source, &commands, "tcl", true);
    assert_eq!(
        r.diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E100)
            .count(),
        1,
        "expected one E100; got {:?}",
        r.diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn analyse_chunked_returns_per_chunk_snapshots() {
    // ``analyse_chunked`` returns one snapshot per chunk.
    // Two chunks → two snapshots; the second snapshot
    // captures cumulative state.
    use crate::segmenter::segment_commands;
    let source = "set x 1\nproc foo {} {}";
    // Chunks carry the document's own offsets.
    let mut commands = segment_commands(source).into_iter();
    let chunk1 = vec![commands.next().unwrap()];
    let chunk2: Vec<_> = commands.collect();
    let mut a = Analyser::new();
    let (r, snapshots) = a.analyse_chunked(source, vec![chunk1, chunk2], "tcl");
    assert_eq!(snapshots.len(), 2);
    // After chunk 1, x is in scope.
    assert!(snapshots[0].result.global_scope.variables.contains_key("x"));
    // After chunk 2, foo is in all_procs.
    assert!(snapshots[1].result.all_procs.contains_key("::foo"));
    // The final result has both.
    assert!(r.global_scope.variables.contains_key("x"));
    assert!(r.all_procs.contains_key("::foo"));
}

#[test]
fn analyse_commands_finalise_false_skips_diagnostic_tail() {
    // When ``finalise=false``, the dedupe/disabled-codes
    // filters don't run — useful for partial-snapshot paths
    // where the tail is deferred.
    use crate::segmenter::segment_commands;
    let source = "proc set {} {}"; // would normally trip W113
    let commands = segment_commands(source);
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let r = a.analyse_commands(source, &commands, "tcl", false);
    // W113 was emitted by handle_proc but the tail didn't
    // run, so apply_disabled_diagnostics / dedupe didn't
    // touch the diag list.  The diag is still there.
    assert!(r.diagnostics.iter().any(|d| d.code == DiagCode::W113));
}

#[test]
fn default_constructs_via_new() {
    let a = Analyser::default();
    assert_eq!(a.current_scope_path.len(), 0);
    assert_eq!(a.body_depth, 0);
}

// -- tcllib `<NS>::import <ALIAS>` wrapper detection

#[test]
fn analyse_records_tcllib_import_wrapper_as_conjectured() {
    let mut a = Analyser::new();
    let r = a.analyse("term::ansi::send::import vt\n", "tcl");
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].ns, "::vt");
    assert_eq!(conjectured[0].pattern, "::term::ansi::send::*");
}

#[test]
fn analyse_tcllib_import_wrapper_alias_relative_to_current_namespace() {
    // ``some::ns::import alias`` inside ``namespace eval outer``
    // creates ``::outer::alias``, not ``::alias``.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval outer { term::ansi::send::import vt }\n",
        "tcl",
    );
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].ns, "::outer::vt");
    assert_eq!(conjectured[0].pattern, "::term::ansi::send::*");
}

#[test]
fn analyse_tcllib_import_wrapper_absolute_alias_keeps_leading_colons() {
    // ``::alias`` argument is taken as an absolute namespace —
    // current-namespace prefixing is skipped.
    let mut a = Analyser::new();
    let r = a.analyse("term::ansi::send::import ::abs::vt\n", "tcl");
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].ns, "::abs::vt");
}

#[test]
fn analyse_tcllib_import_wrapper_skips_substituted_alias() {
    // ``$var`` / ``[cmd]`` aliases can't be statically resolved —
    // the guard requires the alias to contain no ``$`` or ``[``.
    let mut a = Analyser::new();
    let r1 = a.analyse("term::ansi::send::import $alias\n", "tcl");
    assert!(r1.namespace_imports.iter().all(|i| !i.conjectured));
    let mut a = Analyser::new();
    let r2 = a.analyse("term::ansi::send::import [build]\n", "tcl");
    assert!(r2.namespace_imports.iter().all(|i| !i.conjectured));
}

#[test]
fn analyse_tcllib_import_wrapper_requires_single_argument() {
    // ``X::import alias extras`` is a non-wrapper call — the
    // wrapper idiom takes exactly one alias word.
    let mut a = Analyser::new();
    let r = a.analyse("term::ansi::send::import vt extra\n", "tcl");
    assert!(r.namespace_imports.iter().all(|i| !i.conjectured));
}

#[test]
fn analyse_tcllib_import_wrapper_qualifies_unprefixed_source_ns() {
    // Wrapper command names without a leading ``::`` still
    // resolve to absolute source namespaces — the helper
    // prepends the missing ``::``.
    let mut a = Analyser::new();
    let r = a.analyse("foo::import vt\n", "tcl");
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].pattern, "::foo::*");
}

// -- ``command_aliases`` tests
//
// Pin the no-transitive-chain behaviour of command aliases.

#[test]
fn analyse_alias_chain_records_each_step_independently() {
    // ``a -> b`` and ``b -> expr`` are recorded as two
    // independent entries — neither side resolves
    // transitively to ``expr``.
    let mut a = Analyser::new();
    let r = a.analyse("interp alias {} a {} b\ninterp alias {} b {} expr\n", "tcl");
    let alias_a = r.command_aliases.get("::a").expect("::a recorded");
    assert_eq!(alias_a.target, "b");
    assert_eq!(alias_a.extras, [] as [std::string::String; 0]);
    let alias_b = r.command_aliases.get("::b").expect("::b recorded");
    assert_eq!(alias_b.target, "expr");
    assert_eq!(alias_b.extras, [] as [std::string::String; 0]);
}

#[test]
fn analyse_alias_redefinition_overwrites_target() {
    // The second declaration wins.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp alias {} myop {} expr\ninterp alias {} myop {} puts\n",
        "tcl",
    );
    let alias = r.command_aliases.get("::myop").expect("::myop recorded");
    assert_eq!(alias.target, "puts");
    assert_eq!(alias.extras, [] as [std::string::String; 0]);
}

#[test]
fn analyse_alias_qualified_name_recorded() {
    // ``interp alias {} ::ns::myop {} expr`` records under the
    // fully-qualified key.
    let mut a = Analyser::new();
    let r = a.analyse("interp alias {} ::math::= {} expr\n", "tcl");
    let alias = r
        .command_aliases
        .get("::math::=")
        .expect("::math::= recorded");
    assert_eq!(alias.target, "expr");
}

#[test]
fn analyse_alias_dynamic_name_not_recorded() {
    // ``$n`` in the alias name field doesn't resolve statically
    // — ``::=`` must not appear in ``command_aliases``.
    let mut a = Analyser::new();
    let r = a.analyse("set n \"=\"\ninterp alias {} $n {} expr\n", "tcl");
    assert!(!r.command_aliases.contains_key("::="));
}

// -- ``switch -regexp`` literal-pattern recording
//
// The pattern arms whose token is a literal are recorded as
// ``RegexPattern { command = "switch" }``; ``default`` and
// var / cmd-sub patterns are skipped (variable patterns are
// handled by the regex-vars resolution instead).

#[test]
fn analyse_switch_regexp_form1_records_literal_patterns() {
    // Form 1: pattern/body pairs inline.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val \"^foo\" { puts foo } \"^bar\" { puts bar }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 2);
    assert_eq!(switch_pats[0].pattern, "^foo");
    assert_eq!(switch_pats[1].pattern, "^bar");
}

#[test]
fn analyse_switch_regexp_form2_records_literal_patterns() {
    // Form 2: braced body with pattern/body pairs.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val { ^foo { puts foo } ^bar { puts bar } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 2);
    assert_eq!(switch_pats[0].pattern, "^foo");
    assert_eq!(switch_pats[1].pattern, "^bar");
}

#[test]
fn analyse_braced_expect_clause_regexp_records_literal_and_const_var_patterns() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "set pat {(a+)+$}\nexpect {-regexp {^lit$} { puts lit } -re $pat { puts var }}\n",
        "expect",
    );
    let patterns: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "expect")
        .collect();
    assert!(
        patterns.iter().any(|pattern| pattern.pattern == "^lit$"),
        "literal per-clause regexp missing: {patterns:?}"
    );
    assert!(
        patterns.iter().any(|pattern| pattern.pattern == "(a+)+$"),
        "const variable per-clause regexp missing: {patterns:?}"
    );
}

#[test]
fn analyse_switch_regexp_skips_default_arm() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val { ^foo { puts foo } default { puts none } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 1);
    assert_eq!(switch_pats[0].pattern, "^foo");
}

#[test]
fn analyse_switch_without_regexp_records_nothing() {
    // No ``-regexp`` flag — patterns are glob, not regex.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -- $val { foo { puts foo } bar { puts bar } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(
        switch_pats,
        [] as [&crate::analyser::types::RegexPattern; 0]
    );
}

#[test]
fn analyse_switch_regexp_skips_unresolved_var_pattern() {
    // ``$pat`` arm with no defining ``set`` — no const value
    // available, so the arm is dropped.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val { $pat { puts hit } ^lit { puts lit } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 1);
    assert_eq!(switch_pats[0].pattern, "^lit");
}

// -- ``regex-vars`` const-string propagation
//
// Verify that ``$var`` regex pattern arguments resolve to the
// literal stored by a preceding ``set var "..."`` (regexp /
// regsub and switch -regexp Form 2).

#[test]
fn analyse_regexp_resolves_var_pattern_to_const_string() {
    let mut a = Analyser::new();
    let r = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
    // Two records: the use site (the ``$p`` token) and the
    // defining ``set`` value.
    let regexp_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "regexp")
        .collect();
    assert_eq!(regexp_pats.len(), 2);
    assert!(regexp_pats.iter().all(|p| p.pattern == "^foo"));
}

#[test]
fn analyse_regsub_resolves_var_pattern_to_const_string() {
    let mut a = Analyser::new();
    let r = a.analyse("set p {a+}\nregsub -all $p $line - out\n", "tcl");
    let regsub_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "regsub")
        .collect();
    assert_eq!(regsub_pats.len(), 2);
    assert!(regsub_pats.iter().all(|p| p.pattern == "a+"));
}

/// A braced switch body substitutes nothing: `$p` there is the literal
/// pattern `$p`, never `p`'s value. tclsh 8.4.20, 8.6 and 9.0 send `foo`
/// to `default`.
#[test]
fn analyse_switch_regexp_braced_pattern_is_not_the_variable_value() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "set p {^foo}\nswitch -regexp -- $val { $p { puts foo } ^bar { puts bar } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    let pats: Vec<&str> = switch_pats.iter().map(|p| p.pattern.as_str()).collect();
    assert!(!pats.contains(&"^foo"), "got {pats:?}");
    assert!(pats.contains(&"^bar"), "got {pats:?}");
}

#[test]
fn analyse_regex_var_unresolved_records_nothing() {
    // No defining ``set`` — Var has no const value.  The
    // pattern arg is dropped.
    let mut a = Analyser::new();
    let r = a.analyse("regexp $p $line\n", "tcl");
    assert_eq!(
        r.regex_patterns,
        [] as [crate::analyser::types::RegexPattern; 0]
    );
}

// -- W105 unbraced-body emitter

#[test]
fn analyse_emits_w105_for_unbraced_if_body_with_substitution() {
    let mut a = Analyser::new();
    let r = a.analyse("if {$cond} \"puts $x\"\n", "tcl");
    let w105: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W105)
        .collect();
    assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
    // Substitution-bearing bodies are flagged at error severity.
    assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
}

#[test]
fn analyse_skips_w105_for_braced_if_body() {
    // Braced ``{ ... }`` body — no W105.
    let mut a = Analyser::new();
    let r = a.analyse("if {$cond} { puts $x }\n", "tcl");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W105));
}

#[test]
fn analyse_skips_w105_for_command_substitution_body() {
    // A whole-word `[…]` command-substitution body is the safe
    // list-building idiom — `check_unbraced_body` exempts `Cmd`
    // tokens, so no W105.
    let mut a = Analyser::new();
    let r = a.analyse("eval [list set y $x]\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W105),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_skips_w105_for_single_var_body() {
    // FP-STY-14: a body that is a *single bare variable* (`$body`,
    // `$cmd`, `$script`) is a script-valued reference — the variable
    // already holds the script — not an inline code block.  Bracing it
    // (`{$body}`) would turn the reference into the literal text, so the
    // W105 quick-fix is wrong and must not fire: `while {$cond} $body`
    // → no W105, as for `eval $cmd`, `proc $n $a $body`, `uplevel $script`.
    let mut a = Analyser::new();
    for src in [
        "while {$cond} $body\n",
        "eval $cmd\n",
        "after 0 $coroName\n",
        "proc $fakeName $arglist $body\n",
        "foreach name $nameList $body\n",
        "uplevel $script\n",
    ] {
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W105),
            "W105 should be silent on single-var body {src:?}, got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn analyse_emits_w105_for_quoted_interpolated_body() {
    // TP control: a *quoted* body with interpolation (`eval "do $script"`)
    // really is an inline script woven from substitutions — it can and
    // should be braced, so W105 still fires at ERROR severity.
    let mut a = Analyser::new();
    let r = a.analyse("eval \"do $script\"\n", "tcl");
    let w105: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W105)
        .collect();
    assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
    assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
}

// -- W110 string-compare-in-expr emitter

#[test]
fn analyse_emits_w110_for_string_eq_in_if_condition() {
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert!(w110[0].message.contains("eq"), "got {:?}", w110[0].message);
    assert!(matches!(w110[0].severity, crate::analyser::Severity::Hint));
}

#[test]
fn analyse_emits_w110_for_string_ne_in_if_condition() {
    let mut a = Analyser::new();
    let r = a.analyse("if {$x != \"bar\"} {puts no}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert!(w110[0].message.contains("ne"), "got {:?}", w110[0].message);
}

#[test]
fn w110_span_anchors_on_the_operator() {
    // Range precision: the diagnostic anchors on the `==` itself, not
    // the whole condition.
    let src = "if {$x == \"foo\"} {puts yes}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    let expected = src.find("==").unwrap();
    assert_eq!(
        (w110[0].span.start() as usize, w110[0].span.end() as usize),
        (expected, expected + 2),
        "W110 must cover exactly the operator, got {:?}",
        w110[0].span
    );
    // The fix rewrites the operator in place and leaves the braces and
    // operands as written.
    assert_eq!(w110[0].fixes.len(), 1, "fix expected: {:?}", w110[0]);
    let fix = &w110[0].fixes[0];
    assert_eq!(
        fix.span, w110[0].span,
        "the fix covers exactly the operator"
    );
    assert_eq!(fix.new_text, "eq");
}

#[test]
fn w110_span_anchors_on_the_matched_operator_not_the_first() {
    // `$a == $b` (variable compare — not flagged) precedes the
    // string compare `$c == "x"`; the anchor must be the SECOND `==`.
    let src = "if {$a == $b && $c == \"x\"} {puts yes}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    let expected = src.rfind("==").unwrap();
    assert_eq!(
        (w110[0].span.start() as usize, w110[0].span.end() as usize),
        (expected, expected + 2),
        "W110 must anchor the matched (second) `==`, got {:?}",
        w110[0].span
    );
    // The fix rewrites only the proven operator; the variable compare,
    // which may be numeric, keeps its `==`.
    assert_eq!(w110[0].fixes.len(), 1, "fix expected: {:?}", w110[0]);
    let fix = &w110[0].fixes[0];
    let (s, e) = (fix.span.start() as usize, fix.span.end() as usize);
    let applied = format!("{}{}{}", &src[..s], fix.new_text, &src[e..]);
    assert_eq!(applied, "if {$a == $b && $c eq \"x\"} {puts yes}\n");
}

#[test]
fn analyse_no_w110_for_numeric_compare() {
    // ``$x == 42`` — numeric literal on the right, no string
    // operand, should not fire.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == 42} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w110_for_eq_operator() {
    // ``$x eq "foo"`` is the correct form — no W110.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x eq \"foo\"} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w110_includes_eq_code_fix() {
    // Single ``==`` against a string literal — the blanket
    // rewrite should run and produce a fix containing ``eq``.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert_eq!(w110[0].fixes.len(), 1, "got {:?}", w110[0].fixes);
    assert!(
        w110[0].fixes[0].new_text.contains("eq"),
        "got {:?}",
        w110[0].fixes[0].new_text
    );
}

#[test]
fn analyse_no_w110_for_variable_only_compare() {
    // ``$a == $b`` — both operands are variables, may hold
    // ints, no W110.
    let mut a = Analyser::new();
    let r = a.analyse("if {$a == $b} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w110_for_numeric_string_literal() {
    // ``$x == "42"`` compares numerically when `x` is `42.0`, where `eq`
    // would not, so suggesting `eq` would change the result.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"42\"} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w110_fires_for_boolean_string_literal() {
    // ``$x == "true"`` — `==` never reads a boolean word as a number, so
    // this compare is already a string compare.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"true\"} {puts yes}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fires_through_unary_negation() {
    // ``!($x == "foo")`` — W110 walks through ExprUnary.
    let mut a = Analyser::new();
    let r = a.analyse("if {!($x == \"foo\")} {puts no}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fix_leaves_an_unproven_compare_alone() {
    // ``$a == $b || $x == "foo"`` — only the second ``==`` is proven a
    // string compare, so the fix rewrites that one and keeps the first.
    let mut a = Analyser::new();
    let src = "if {$a == $b || $x == \"foo\"} {puts y}\n";
    let r = a.analyse(src, "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert_eq!(w110[0].fixes.len(), 1, "got {:?}", w110[0].fixes);
    let fix = &w110[0].fixes[0];
    let (s, e) = (fix.span.start() as usize, fix.span.end() as usize);
    let applied = format!("{}{}{}", &src[..s], fix.new_text, &src[e..]);
    assert_eq!(applied, "if {$a == $b || $x eq \"foo\"} {puts y}\n");
}

#[test]
fn analyse_w110_fires_on_while_condition() {
    // ``while {EXPR} {body}`` — EXPR-role is at index 0.
    let mut a = Analyser::new();
    let r = a.analyse("while {$x == \"foo\"} { break }\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fires_on_top_level_expr_command() {
    // ``expr {$x == "foo"}`` — top-level invocation of
    // ``expr`` exercises the EXPR-role dispatch on the
    // single braced arg.  (Nested ``[expr ...]`` command
    // substitutions are recorded as invocations but the
    // analyser doesn't currently re-enter them for per-
    // command checks; that's a separate concern.)
    let mut a = Analyser::new();
    let r = a.analyse("expr {$x == \"foo\"}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fires_on_multi_arg_expr_command() {
    // ``expr $x == "foo"`` (no braces, multiple argv slots): the args
    // are joined with spaces — the *substituted* word values, whose
    // quote delimiters Tcl's word splitting already stripped — so the
    // expression is ``$x == foo`` with ``foo`` a bareword, not an
    // ``ExprString``.  W110 (string ``==``) therefore does NOT fire on
    // the unbraced form (only W100 does); it fires on the *braced*
    // ``expr {$x == "foo"}`` where the string literal survives.
    let mut a = Analyser::new();
    let r = a.analyse("expr $x == \"foo\"\n", "tcl");
    let unbraced: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(
        unbraced.len(),
        0,
        "unbraced expr must not fire W110, got {:?}",
        r.diagnostics
    );

    let mut a2 = Analyser::new();
    let r2 = a2.analyse("set z [expr {$x == \"foo\"}]\n", "tcl");
    let braced: Vec<_> = r2
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(
        braced.len(),
        1,
        "braced expr must fire W110, got {:?}",
        r2.diagnostics
    );
}

#[test]
fn call_by_name_var_not_flagged_unused_or_dead() {
    // A caller-local passed *by name* to a proc that upvar-writes
    // its param (`fill tag`) must not be flagged W211 (unused) or
    // W220 (dead store).
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc ::fill {vn} { upvar 1 $vn v\nset v 1 }\n\
             proc ::f {} { set tag init\nset tag x\nfill tag }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics.iter().all(|d| {
            !((d.code == DiagCode::W211 || d.code == DiagCode::W220) && d.message.contains("tag"))
        }),
        "call-by-name var `tag` must not be flagged unused/dead, got {:?}",
        r.diagnostics,
    );

    // Negative control: a genuinely unused local is still flagged
    // (the suppression didn't disable W211 wholesale).
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("proc ::g {} { set unused 1\nputs hi }\n", "tcl8.6");
    assert!(
        r2.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W211 && d.message.contains("unused")),
        "a genuinely unused var should still be flagged, got {:?}",
        r2.diagnostics,
    );
}

#[test]
fn dynamic_name_write_does_not_dead_store_the_name_var() {
    // `set $p 1` writes the variable *named by* `$p` and *reads* `p`
    // (verified against tclsh 8.4–9.0).  It must not produce a spurious
    // W220 dead-store on the parameter `p` itself.
    for body in ["set $p 1", "set ${p} 1", "set $p [expr {1}]"] {
        let mut a = Analyser::new();
        let r = a.analyse(&format!("proc ::f {{p}} {{ {body} }}\n"), "tcl");
        assert!(
            r.diagnostics
                .iter()
                .all(|d| !(matches!(d.code.as_str(), "W211" | "W220" | "W214")
                    && d.message.contains("'p'"))),
            "dynamic-name write `{body}` wrongly flagged `p`, got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn dynamic_name_out_var_does_not_suppress_caller_dead_store() {
    // A param used only as a *dynamic name* out-var (`scan`/`lassign`/
    // `regexp`/`regsub` target, or `set $p`) names a callee-local
    // variable — it does NOT write back to the caller's frame, so a
    // caller's literal arg is genuinely dead and must still fire W211 /
    // W220.
    let cases = [
        "proc maybe {target} { scan 42 %d $target }",
        "proc maybe {target} { lassign {1} $target }",
        "proc maybe {target} { regexp {(.)} a -> $target }",
        "proc maybe {target} { regsub a a b $target }",
        "proc maybe {target} { set $target 1 }",
    ];
    for callee in cases {
        let src = format!("{callee}\nproc caller {{}} {{ set x 1; maybe x }}\n");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| (d.code == DiagCode::W211 || d.code == DiagCode::W220)
                    && d.message.contains("'x'")),
            "caller's dead `x` must still fire for `{callee}`, got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn upvar_writeback_still_suppresses_caller_dead_store() {
    // Control: a genuine `upvar`-write-back callee DOES consume the
    // caller's variable, so the dead-store is correctly suppressed.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc fill {outvar} { upvar $outvar out\nset out 42 }\n\
             proc top {} { set result {}\nfill result\nreturn $result }\n",
        "tcl",
    );
    assert!(
        r.diagnostics
            .iter()
            .all(|d| !((d.code == DiagCode::W211 || d.code == DiagCode::W220)
                && d.message.contains("result"))),
        "upvar write-back must suppress the caller dead-store, got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w128_fires_on_call_to_renamed_command() {
    // A builtin renamed/deleted away earlier in the file → the
    // later call falls through to `unknown` → W128.
    let mut a = Analyser::new();
    let r = a.analyse("rename string {}\nstring toupper x\n", "tcl");
    let w128: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W128)
        .collect();
    assert_eq!(w128.len(), 1, "expected one W128, got {:?}", r.diagnostics);

    // Negative: a normal builtin call with no rename → no W128.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("string toupper x\n", "tcl");
    assert!(
        r2.diagnostics.iter().all(|d| d.code != DiagCode::W128),
        "unexpected W128: {:?}",
        r2.diagnostics,
    );

    // Negative: an ordinary unknown external command (never rebound)
    // resolves opaque but must not fire W128.
    let mut a3 = Analyser::new();
    let r3 = a3.analyse("someunknowncmd a b\n", "tcl");
    assert!(
        r3.diagnostics.iter().all(|d| d.code != DiagCode::W128),
        "unexpected W128: {:?}",
        r3.diagnostics,
    );
}

#[test]
fn analyse_w110_no_fire_on_for_clean_condition() {
    // ``for {set i 0} {$i < 10} {incr i} {body}`` — no ``==``
    // anywhere, but ensure the EXPR-role dispatch on ``for``
    // doesn't crash and produces no W110.
    let mut a = Analyser::new();
    let r = a.analyse("for {set i 0} {$i < 10} {incr i} { break }\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w110_fires_on_for_condition() {
    // ``for {set i 0} {$x == "foo"} {incr i} {body}`` —
    // the EXPR-role dispatch runs *before* the hook handlers,
    // so W110 on a ``for`` condition fires whichever walk
    // owns the bodies.
    let mut a = Analyser::new();
    let r = a.analyse("for {set i 0} {$x == \"foo\"} {incr i} { break }\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

// -- W302 catch-without-result-var emitter

#[test]
fn analyse_emits_w302_for_catch_without_result_var() {
    let mut a = Analyser::new();
    let r = a.analyse("catch { puts hi }\n", "tcl");
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w302[0].message.contains("silently swallows errors"),
        "got {:?}",
        w302[0].message
    );
    assert!(matches!(w302[0].severity, crate::analyser::Severity::Hint));
}

#[test]
fn analyse_no_w302_when_catch_has_result_var() {
    // ``catch BODY result`` — result variable is present, so
    // errors aren't silently swallowed.
    let mut a = Analyser::new();
    let r = a.analyse("catch { puts hi } result\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w302_when_catch_has_options_var() {
    // ``catch BODY result options`` — both optional vars
    // present.  Still no W302.
    let mut a = Analyser::new();
    let r = a.analyse("catch { puts hi } result options\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w302_for_multi_token_catch_body() {
    // ``catch pre$x`` — a multi-token-word body is a dynamic
    // body (not a single-token word), so it is not treated as a
    // braced catch body and W302 never fires.  The emitter gates
    // on the body being a single-token word.
    let mut a = Analyser::new();
    let r = a.analyse("catch pre$x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w302_anchors_at_catch_keyword() {
    // W302 highlights just the ``catch`` command token — the
    // narrowest span that identifies the issue — rather than the
    // whole ``catch {…}`` statement
    // (which also dropped the closing brace under the lexer's
    // inner-end convention).
    let mut a = Analyser::new();
    let src = "catch { puts hi }\n";
    let r = a.analyse(src, "tcl");
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1);
    let span = w302[0].span;
    let text = &src[span.start() as usize..span.end() as usize];
    assert_eq!(text, "catch", "W302 should span only the catch keyword");
}

// -- W302 quick-fix insertion anchor
//
// The diagnostic anchors at the `catch` keyword, so the fix must carry
// its **own** span: the point past the *body's* closing delimiter.  Each
// test applies the fix to the source and asserts the resulting text, so a
// wrong anchor cannot pass by matching only the inserted string.

/// Apply W302's `nth` fix to `src` and return the rewritten source.
fn apply_w302_fix(src: &str, nth: usize, dialect: &str) -> String {
    let mut a = Analyser::new();
    let r = a.analyse(src, dialect);
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1, "expected one W302, got {:?}", r.diagnostics);
    let fix = w302[0]
        .fixes
        .get(nth)
        .unwrap_or_else(|| panic!("no fix {nth}: {:?}", w302[0].fixes));
    let mut out = src.to_string();
    out.replace_range(
        fix.span.start() as usize..fix.span.end() as usize,
        &fix.new_text,
    );
    out
}

#[test]
fn w302_result_fix_inserts_after_the_body() {
    // The issue's reproducer.  Anchoring at the diagnostic's end would
    // produce `catch result {error oops}`, which C Tcl reads as a catch
    // of the script `result` — completion code 1, result
    // `invalid command name "result"`.
    assert_eq!(
        apply_w302_fix("catch {error oops}\n", 0, "tcl9.0"),
        "catch {error oops} result\n"
    );
}

#[test]
fn w302_options_fix_inserts_after_the_body() {
    assert_eq!(
        apply_w302_fix("catch {error oops}\n", 1, "tcl9.0"),
        "catch {error oops} result options\n"
    );
}

#[test]
fn w302_fix_handles_a_multiline_body() {
    let src = "catch {\n    error oops\n}\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {\n    error oops\n} result\n"
    );
}

#[test]
fn w302_fix_handles_a_trailing_comment() {
    let src = "catch {error oops} ;# ignore\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {error oops} result ;# ignore\n"
    );
}

#[test]
fn w302_fix_handles_a_semicolon_separated_command() {
    let src = "catch {error oops}; puts done\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {error oops} result; puts done\n"
    );
}

#[test]
fn w302_fix_handles_nested_substitutions_in_the_body() {
    let src = "catch {puts [expr {1 + [foo $x]}]}\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {puts [expr {1 + [foo $x]}]} result\n"
    );
}

#[test]
fn w302_fix_does_not_overshoot_an_empty_body() {
    // `catch {}`'s body span already covers its own closer, so a naive
    // `span.end() + 1` anchor would insert one byte past the brace.
    let src = "catch {}\n";
    assert_eq!(apply_w302_fix(src, 0, "tcl9.0"), "catch {} result\n");
}

#[test]
fn w302_fix_handles_a_leading_qualified_head() {
    // `::catch` resolves to the same registry spec (and the same
    // `AnalyserHookId::Catch`), so it is diagnosed and fixed identically.
    let src = "::catch {error oops}\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "::catch {error oops} result\n"
    );
}

#[test]
fn w302_offers_only_the_result_variable_under_tcl84() {
    // Tcl 8.4's `catch script ?varName?` has no options-dictionary
    // argument, so the second fix must not be offered there — the count
    // comes from the dialect-applicable synopsis, not from a constant.
    let mut a = Analyser::new();
    let r = a.analyse("catch {error oops}\n", "tcl8.4");
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1, "got {:?}", r.diagnostics);
    assert_eq!(
        w302[0].fixes.len(),
        1,
        "8.4 documents one optional word: {:?}",
        w302[0].fixes
    );
    assert_eq!(w302[0].fixes[0].new_text, " var");
}

#[test]
fn w302_fixes_are_never_bulk_applicable() {
    // Capturing the result writes a new variable in the caller's frame,
    // which a program already using that name observes — so the fix is
    // hardening, not a semantics-preserving rewrite.
    let mut a = Analyser::new();
    let r = a.analyse("catch {error oops}\n", "tcl9.0");
    let w302 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W302)
        .expect("W302");
    assert_ne!(w302.fixes, [] as [crate::irules_checks::CodeFix; 0]);
    assert!(
        w302.fixes.iter().all(|f| !f.safety.is_bulk_applicable()),
        "got {:?}",
        w302.fixes
    );
}

#[test]
fn w302_carries_no_fix_for_a_malformed_catch() {
    // An unterminated body never reaches a well-formed insertion point;
    // whatever the recovery path emits, no W302 fix may point outside
    // the buffer.
    let mut a = Analyser::new();
    let src = "catch {error oops\n";
    let r = a.analyse(src, "tcl9.0");
    for diag in r.diagnostics.iter().filter(|d| d.code == DiagCode::W302) {
        for fix in &diag.fixes {
            assert!(
                fix.span.end() as usize <= src.len(),
                "fix span {:?} outside the buffer",
                fix.span
            );
        }
    }
}

// -- W001 unknown-subcommand emitter

#[test]
fn analyse_emits_w001_for_unknown_string_subcommand() {
    let mut a = Analyser::new();
    let r = a.analyse("string bogus $x\n", "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w001[0].message.contains("'bogus'") && w001[0].message.contains("'string'"),
        "got {:?}",
        w001[0].message
    );
    assert!(matches!(
        w001[0].severity,
        crate::analyser::Severity::Warning
    ));
}

#[test]
fn analyse_w001_includes_did_you_mean_suggestion() {
    // ``string lenght`` — single-char typo for ``length``,
    // edit distance 2.  ``suggest_similar`` should surface
    // ``length``.
    let mut a = Analyser::new();
    let r = a.analyse("string lenght $x\n", "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w001[0].message.contains("did you mean 'length'"),
        "got {:?}",
        w001[0].message
    );
    assert!(
        w001[0].fixes.iter().any(|f| f.new_text == "length"),
        "got {:?}",
        w001[0].fixes
    );
}

#[test]
fn analyse_no_w001_for_known_subcommand() {
    // ``string length $x`` — known subcommand.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("string length $x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_dynamic_subcommand_position() {
    // ``string $sub $x`` — runtime-resolved subcommand;
    // can't statically check.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("string $sub $x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_command_substitution_in_subcommand_position() {
    // ``string [pick] $x`` — ``[…]`` in the subcommand
    // position is also a runtime-resolved value.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("string [pick] $x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_simple_command() {
    // ``set x 1`` — ``set`` has no SubcommandSig.  No W001.
    let mut a = Analyser::new();
    let r = a.analyse("set x 1\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w001_for_unknown_command() {
    // ``unknownthing foo`` — registry doesn't know the
    // command, so no signature lookup, no W001.  (W123
    // owns the unknown-command diagnostic.)
    let mut a = Analyser::new();
    let r = a.analyse("unknownthing foo\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w001_anchors_at_subcommand_token_only() {
    // The squiggle sits tightly on the offending word alone — not the
    // command name too — matching the "did you mean" fix's replacement
    // range and the KCS doc's documented "squiggle under the subcommand
    // token" behaviour.
    let mut a = Analyser::new();
    let src = "string bogus $x\n";
    let r = a.analyse(src, "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1);
    let span = w001[0].span;
    let text = &src[span.start() as usize..span.end() as usize];
    assert_eq!(text, "bogus", "got {text:?}");
}

#[test]
fn analyse_emits_w001_for_unknown_dict_subcommand() {
    // ``dict`` is also a SubcommandSig command — confirm
    // dispatch isn't ``string``-specific.
    let mut a = Analyser::new();
    let r = a.analyse("dict froob $d $k\n", "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w001[0].message.contains("'dict'"),
        "got {:?}",
        w001[0].message
    );
}

#[test]
fn analyse_w001_fix_replaces_wrapped_literal_subcommand() {
    // Wrapper tokens (``Str`` braced ``{lenght}`` / ``Esc``
    // quoted ``"lenght"``) carry the opening delimiter via
    // ``content_offset`` and the lexer span excludes the
    // closing delimiter; the W001 code-fix targets the
    // content range so the replacement preserves the
    // wrapping ``}`` / ``"`` rather than leaving a stray
    // trailing delimiter behind.
    for (src, expected) in [
        ("string {lenght} $x\n", "string {length} $x\n"),
        ("string \"lenght\" $x\n", "string \"length\" $x\n"),
        ("string lenght $x\n", "string length $x\n"),
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "src={src:?} got {:?}", r.diagnostics);
        assert!(
            w001[0].message.contains("did you mean 'length'"),
            "src={src:?} got {:?}",
            w001[0].message
        );

        let fix = w001[0]
            .fixes
            .iter()
            .find(|f| f.new_text == "length")
            .expect("expected replacement fix to 'length'");

        let mut fixed = src.to_string();
        let start = fix.span.start() as usize;
        let end = fix.span.end() as usize;
        fixed.replace_range(start..end, &fix.new_text);

        assert_eq!(fixed, expected, "src={src:?} fixes={:?}", w001[0].fixes);
        assert!(!fixed.contains("lenght"), "src={src:?} fixed={fixed:?}");
    }
}

#[test]
fn analyse_w001_diagnostic_span_matches_fix_span() {
    // The squiggle and the "did you mean" quick-fix must target the
    // identical range — otherwise accepting the fix visibly leaves part
    // of the squiggled text unchanged.
    let mut a = Analyser::new();
    let src = "string lenght $x\n";
    let r = a.analyse(src, "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    let fix = w001[0]
        .fixes
        .first()
        .expect("expected a 'did you mean' fix");
    assert_eq!(w001[0].span, fix.span, "got {:?}", w001[0]);
}

#[test]
fn analyse_w001_no_diagnostic_at_all_when_shadowed_by_proc() {
    // A same-file `proc string {...}` must suppress W001 (and its
    // subcommand-level W002 sibling) completely — not just avoid the
    // specific message text. The FP-STY-17 reproducers live in
    // `analyser/diagnostics/fp/sty.rs::fp_sty_17_*`.
    let mut a = Analyser::new();
    let src = "proc string {op args} { return $op }\nstring reverse hello\n";
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W001 | DiagCode::W002)),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w001_shadow_suppression_is_scoped_to_the_shadowed_name() {
    // Shadowing `string` must not suppress W001 for a genuinely unknown
    // subcommand on a different, unshadowed ensemble in the same file.
    let mut a = Analyser::new();
    let src = "proc string {op args} { return $op }\ninfo bogus\n";
    let r = a.analyse(src, "tcl");
    let w001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W001)
        .collect();
    assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
    assert!(w001[0].message.contains("'info'"), "got {:?}", w001[0]);
}

#[test]
fn analyse_no_w001_for_expanded_literal_subcommand() {
    // `{*}{create a b}` splices the elements `create`, `a`, `b` into the
    // argument list (confirmed against tclsh 8.6.14) — the raw source
    // text "create a b" must never be compared against the subcommand
    // set. The FP-STY-18 reproducers live in
    // `analyser/diagnostics/fp/sty.rs::fp_sty_18_*`.
    let mut a = Analyser::new();
    let r = a.analyse("dict {*}{create a b}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w001_still_fires_for_unexpanded_unknown_subcommand() {
    // TP control for the `{*}`-expansion skip: the identical subcommand
    // set, without expansion, still fires on a genuine typo.
    let mut a = Analyser::new();
    let r = a.analyse("dict bogus a b\n", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "got {:?}",
        r.diagnostics
    );
}

// -- E004 malformed-if emitter
//
// Fires ``Severity::Error`` when an ``if`` invocation's structural
// shape doesn't match
// ``if COND BODY ?elseif COND BODY ...? ?else BODY?``.  Detection
// reads the registry's walk of `if`'s clause grammar through
// `CommandRegistry::clause_shape_defect` — the grammar itself is not
// reimplemented here (see `emit_e004_clause_shape_diagnostic`).
// Every case is cross-checked against tclsh 8.6 and Tcl 9.0.4's
// `TclNRIfObjCmd` source; see the truth table in
// `tcl-registry/src/commands/tcl/if_.rs`'s own tests for the
// grammar-level cases this integration layer doesn't repeat.

fn e004_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::E004)
        .collect()
}

fn span_text(src: &str, span: tcl_lexer::Span) -> &str {
    &src[span.start() as usize..span.end() as usize]
}

// -- TP: genuinely malformed shapes, with the precise message and a
// tight span (not the whole `if` statement).

#[test]
fn tp_bare_if_names_the_invoked_command_in_the_message() {
    // ``if`` alone — no expression at all.  Message and span both
    // name "if" itself (real Tcl: `no expression after "if"
    // argument`), and this must be the *only* diagnostic for the
    // line — no redundant generic arity error alongside it (see
    // `no_duplicate_e002_alongside_e004`).
    let src = "if\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No expression after \"if\" argument");
    assert_eq!(span_text(src, e004[0].span), "if");
}

#[test]
fn tp_condition_without_body_names_the_condition_text() {
    // ``if {1}`` — real Tcl: `no script following "1" argument`.
    let src = "if {1}\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"1\" argument");
    assert_eq!(span_text(src, e004[0].span), "{1}");
}

#[test]
fn tp_then_keyword_without_body_names_then() {
    // ``if {1} then`` — real Tcl: `no script following "then"
    // argument`.
    let src = "if {1} then\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"then\" argument");
    assert_eq!(span_text(src, e004[0].span), "then");
}

#[test]
fn tp_bare_else_without_body_names_else() {
    // ``if {1} { a } else`` — real Tcl: `no script following
    // "else" argument`.
    let src = "if {1} { a } else\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"else\" argument");
    assert_eq!(span_text(src, e004[0].span), "else");
}

#[test]
fn tp_elseif_without_expr_names_elseif() {
    // ``if {1} { a } elseif`` — real Tcl: `no expression after
    // "elseif" argument`.
    let src = "if {1} { a } elseif\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No expression after \"elseif\" argument");
    assert_eq!(span_text(src, e004[0].span), "elseif");
}

#[test]
fn tp_elseif_condition_without_body_names_the_condition_text() {
    // ``if {1} { a } elseif {2}`` — real Tcl: `no script
    // following "2" argument`.
    let src = "if {1} { a } elseif {2}\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"2\" argument");
    assert_eq!(span_text(src, e004[0].span), "{2}");
}

#[test]
fn tp_extra_words_after_explicit_else_anchors_only_the_extra_words() {
    // ``if {1} { a } else { b } extra`` — the span covers just
    // "extra", not the whole statement.
    let src = "if {1} { a } else { b } extra\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(
        e004[0].message,
        "Extra words after \"else\" clause in \"if\" command"
    );
    assert_eq!(span_text(src, e004[0].span), "extra");
    assert!(matches!(e004[0].severity, crate::analyser::Severity::Error));
}

#[test]
fn tp_extra_words_after_implicit_else_anchors_only_the_extra_word() {
    // ``if {1} { a } { b } { c }`` — implicit else (no ``else``
    // keyword); "c" is the first extra word.
    let src = "if {1} { a } { b } { c }\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(span_text(src, e004[0].span), "{ c }");
}

#[test]
fn tp_qualified_double_colon_if_is_checked_too() {
    // ``::if`` names the same global command as ``if`` — registry
    // name resolution strips the leading ``::`` for every command,
    // so the dispatch-generic hook lookup picks this up without any
    // `if`-specific handling in the compiler.
    let src = "::if {1} { a } { b } { c }\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
}

// -- FP fixed: a leading `else`/`elseif` is a well-formed `if` whose
// condition happens to be that bareword — not a malformed `if`.
// Verified against tclsh 8.6: both raise `invalid bareword "else"` /
// `"elseif"` at *expression*-evaluation time, never a `wrong # args`
// structural error.

#[test]
fn fp_leading_else_is_not_malformed() {
    let r_diags = e004_diags("if else { x }\n");
    assert_eq!(r_diags.len(), 0, "got {r_diags:?}");
}

#[test]
fn fp_leading_elseif_is_not_malformed() {
    let r_diags = e004_diags("if elseif { x }\n");
    assert_eq!(r_diags.len(), 0, "got {r_diags:?}");
}

#[test]
fn fp_else_in_elseif_condition_slot_is_not_malformed() {
    // ``if {1} { a } elseif else { b }`` — "else" sits in the
    // *elseif's* condition slot, never keyword-matched there
    // either (verified against tclsh 8.6: runs body "a", not a
    // wrong-#args error).
    let r_diags = e004_diags("if {1} { a } elseif else { b }\n");
    assert_eq!(r_diags.len(), 0, "got {r_diags:?}");
}

// -- TN: well-formed shapes never flagged.

#[test]
fn tn_single_clause_if() {
    assert_eq!(
        e004_diags("if {1} { a }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_if_else() {
    assert_eq!(
        e004_diags("if {1} { a } else { b }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_if_elseif_else_chain() {
    assert_eq!(
        e004_diags("if {$a} { x } elseif {$b} { y } else { z }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_if_with_then_keyword() {
    assert_eq!(
        e004_diags("if {1} then { a }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_implicit_else_single_body() {
    // ``if {1} { a } { b }`` — one implicit-else body, no keyword.
    assert_eq!(
        e004_diags("if {1} { a } { b }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_inside_tcloo_method_body() {
    // `if` structural checking is generic body-walking — it must
    // fire the same way inside a TclOO method body as at top level.
    let src = "oo::class create C {\n  method m {} {\n    if {1} { a } { b } { c }\n  }\n}\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
}

// Original move lineage retains the selected clause grammar.
#[test]
fn renamed_if_keeps_its_original_clause_shape() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let src = "rename if myif\nmyif {1} { a } { b } { c }\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
}

// -- Redundant-diagnostic fix: `if`'s registry `arity` floor no
// longer produces a second, generic E002 alongside the precise
// E004 for the same defect.

#[test]
fn no_duplicate_e002_alongside_e004() {
    for src in ["if\n", "if {1}\n"] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::E002),
            "src={src:?} got {:?}",
            r.diagnostics
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::E004),
            "src={src:?} got {:?}",
            r.diagnostics
        );
    }
}

// -- Code fixes.

fn apply_fix(src: &str, fix: &crate::analyser::types::CodeFix) -> String {
    let mut fixed = src.to_string();
    fixed.replace_range(
        fix.span.start() as usize..fix.span.end() as usize,
        &fix.new_text,
    );
    fixed
}

#[test]
fn fix_merges_extra_words_into_the_final_body() {
    let src = "if {1} { a } { b } { c }";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } {{ b } { c }}");
    // The fixed source must itself be shape-well-formed.
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_merges_extra_words_after_explicit_else() {
    let src = "if {1} { a } else { b } extra";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } else {{ b } extra}");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_removes_dangling_elseif_clause() {
    let src = "if {1} { a } elseif";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    assert_eq!(e004[0].fixes[0].new_text, "");
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } ");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_removes_dangling_else_with_no_body() {
    let src = "if {1} { a } else";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } ");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_removes_dangling_elseif_condition_without_body() {
    let src = "if {1} { a } elseif {2}";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } ");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn no_fix_offered_when_the_first_clause_never_completed() {
    // ``if`` / ``if {1}`` — there is no well-formed prefix to fall
    // back to, so no fix is offered (never a guessed body).
    for src in ["if", "if {1}", "if {1} then"] {
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "src={src:?}");
        assert_eq!(
            e004[0].fixes.len(),
            0,
            "src={src:?} got {:?}",
            e004[0].fixes
        );
    }
}

// -- W304 missing-option-terminator emitter
//
// Resolution profile lives in ``tcl-registry``; tristate severity
// / two-diagnostic origin / code-fix logic lives in
// ``analyser/diagnostics.rs``.

fn w304_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W304)
        .collect()
}

#[test]
fn analyse_emits_w304_for_regexp_pattern_variable() {
    let diags = w304_diags("regexp $pattern $text\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
    assert!(
        diags[0].message.to_lowercase().contains("option-injection"),
        "got {:?}",
        diags[0].message
    );
    assert!(matches!(
        diags[0].severity,
        crate::analyser::Severity::Suggestion
    ));
}

#[test]
fn analyse_no_w304_for_regexp_safe_literal_pattern() {
    // Pattern starts with `(` — non-dynamic, doesn't start with
    // `-`, so the OFF gate suppresses regardless of command.
    let diags = w304_diags("regexp {(a+)+$} $text\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_regexp_literal_dash_pattern() {
    // ``regexp {-[0-9]+} $text`` — the literal pattern starts
    // with `-` so the positional scanner treats it as an
    // unknown option and lands on the next positional
    // (``$text``).  The diagnostic still fires; severity comes
    // from the dynamic-var INFO path because the diag anchors on
    // ``$text``, not the pattern literal.
    let diags = w304_diags("regexp {-[0-9]+} $text\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_exec_literal_dash_after_first_positional() {
    // ``exec foo -bad`` — ``first_positional_without_terminator``
    // treats ``foo`` (index 0) as the first positional, so the
    // OFF gate suppresses W304 there (non-dynamic, doesn't start
    // with ``-``).  The later literal ``-bad`` is not
    // re-considered as a candidate "first positional" argument,
    // so no diagnostic fires.
    //
    // This pins the scanner / first-positional behaviour for
    // ``exec`` rather than exercising the literal-dash WARN
    // branch.  The WARN branch is covered by
    // `analyse_emits_w304_for_regexp_literal_dash_pattern`
    // (literal pattern starting with `-`) and
    // `analyse_w304_constant_propagation_dash_value_warns`
    // (variable resolved via constant-prop to a `-`-prefixed
    // value).
    let diags = w304_diags("exec foo -bad\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_regexp_with_terminator() {
    let diags = w304_diags("regexp -- $pattern $text\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_regexp_with_option_value_then_variable() {
    // ``-start`` consumes the next arg as its value; the first
    // positional after it is the pattern variable.
    let diags = w304_diags("regexp -start 0 $pattern $text\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_regsub_variable() {
    let diags = w304_diags("regsub $pattern $text X out\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_subst() {
    // ``subst`` does not declare a ``--`` option — registry-
    // level filter suppresses W304 entirely.
    let diags = w304_diags("subst $template\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_exec_variable() {
    let diags = w304_diags("exec $cmd\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_exec_with_terminator() {
    let diags = w304_diags("exec -- $cmd\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_glob_safe_literal() {
    // ``*.tcl`` does not start with `-`; OFF gate suppresses.
    let diags = w304_diags("glob *.tcl\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_string_match() {
    // ``string match`` does not support ``--`` — registry filter.
    let diags = w304_diags("string match $pattern $value\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_lsearch() {
    // ``lsearch`` does not declare ``--`` either.
    let diags = w304_diags("lsearch -exact $domain c\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_file_delete_variable() {
    // ``file delete`` is subcommand-scoped — profile.scan_start
    // == 1 to skip the ``delete`` keyword.
    let diags = w304_diags("file delete $path\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_file_delete_with_terminator() {
    let diags = w304_diags("file delete -- $path\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_load_variable() {
    let diags = w304_diags("load $fileName\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_w304_constant_propagation_emits_info_with_origin() {
    // ``set X "datagroup"; exec $X`` — the variable resolves to a
    // literal value that doesn't start with `-`, so severity drops
    // to INFO and a second "origin" diagnostic anchors at the
    // literal's range.  (The two-arg braced ``switch $X { ... }``
    // form is exempt from W304 entirely — FP-NAB-05 — so an
    // option-bearing command that still flags is used here.)
    let src = "set totp_key_storage \"datagroup\"\n\
                   exec $totp_key_storage\n";
    let diags = w304_diags(src);
    assert_eq!(diags.len(), 2, "got {diags:?}");

    let main = diags
        .iter()
        .find(|d| !d.fixes.is_empty())
        .expect("main diag");
    let origin = diags
        .iter()
        .find(|d| d.fixes.is_empty())
        .expect("origin diag");

    assert!(
        matches!(main.severity, crate::analyser::Severity::Suggestion),
        "main severity {:?}",
        main.severity
    );
    assert!(
        main.message.contains("totp_key_storage") && main.message.contains("datagroup"),
        "main message {:?}",
        main.message
    );

    let highlighted = &src[main.span.start() as usize..main.span.end() as usize];
    assert_eq!(highlighted, "$totp_key_storage", "got {highlighted:?}");

    // The origin diag points at the ``"datagroup"`` literal in
    // the preceding ``set``.
    let origin_text = &src[origin.span.start() as usize..origin.span.end() as usize];
    assert!(
        origin_text.contains("datagroup"),
        "origin span text {origin_text:?}"
    );
}

#[test]
fn analyse_w304_constant_propagation_dash_value_warns() {
    // The variable resolves to ``-something`` — escalates to
    // WARNING.
    let src = "set evil \"-rf\"\n\
                   exec $evil /\n";
    let diags = w304_diags(src);
    assert!(!diags.is_empty(), "got {diags:?}");
    let main = diags
        .iter()
        .find(|d| !d.fixes.is_empty())
        .expect("main diag");
    assert!(matches!(main.severity, crate::analyser::Severity::Warning));
}

// -- W101 eval-string-concat emitter
//
// Canonical-list-idiom suppression lives in the registry
// (``is_canonical_list_command``); substitution-detection
// approximation lives in the analyser.

fn w101_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W101)
        .collect()
}

#[test]
fn analyse_emits_w101_for_eval_with_variable() {
    let diags = w101_diags("eval \"puts $x\"\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
    assert!(
        diags[0].message.to_lowercase().contains("injection"),
        "got {:?}",
        diags[0].message
    );
    assert!(matches!(
        diags[0].severity,
        crate::analyser::Severity::Warning
    ));
}

#[test]
fn analyse_no_w101_for_eval_braced_script() {
    let diags = w101_diags("eval {puts hello}\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_multiple_braced() {
    let diags = w101_diags("eval {set x 1} {puts $x}\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w101_for_eval_with_command_subst() {
    // ``eval [build_cmd]`` — single CMD token, but `build_cmd`
    // isn't a canonical-list-producing command.
    let diags = w101_diags("eval [build_cmd]\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_literal_no_substitution() {
    // ``eval puts hello`` — both args are bare literals; no
    // substitution at any level.
    let diags = w101_diags("eval puts hello\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_list_idiom() {
    // ``eval [list ...]`` — ``list`` produces a canonical list,
    // safe re-parse.
    let diags = w101_diags("eval [list set $varname $value]\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_linsert_idiom() {
    // ``linsert`` returns TclType::List → canonical.
    let diags = w101_diags("eval [linsert $cmdlist 0 extraarg]\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_split_idiom() {
    let diags = w101_diags("eval [split $line :]\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w101_for_eval_concat_idiom() {
    // ``concat`` is the explicit non-canonical exclusion —
    // strips one level of grouping, not safe for re-parse.
    let diags = w101_diags("eval [concat $script $args]\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_non_eval_commands() {
    // The emitter is gated on ``cmd_name == "eval"`` — other
    // substitution-bearing commands are handled elsewhere (W301
    // covers uplevel; W312 covers interp eval).
    let diags = w101_diags("uplevel 1 \"puts $x\"\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_w101_anchors_at_first_arg_token() {
    let src = "eval \"puts $x\"\n";
    let diags = w101_diags(src);
    assert_eq!(diags.len(), 1);
    let span = diags[0].span;
    let text = &src[span.start() as usize..span.end() as usize];
    // First arg is the quoted string ``"puts $x"`` — the
    // representative token's span anchors the diagnostic.
    assert!(text.contains("puts") || text.contains("$x"), "got {text:?}");
}

#[test]
fn analyse_w101_rejects_multi_command_subscript() {
    // ``[list a; set x $user]`` — multi-command script can't be
    // proven safe (last command's result wins, and that's
    // ``set``, not ``list``).
    let diags = w101_diags("eval [list a\\; set x $user]\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_literal_multi_token_word() {
    // ``eval foo{bar}`` is a multi-token word (Esc + Str joined,
    // ``single_token_word == false``) that contains no Var/Cmd
    // substitution — W101 only fires on actual VAR/CMD tokens, so
    // this must not trigger it.  The check is a brace/backslash-aware
    // source-byte scan that looks for unescaped ``$`` / ``[``
    // outside ``{...}``.
    let diags = w101_diags("eval foo{bar}\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_backslash_escaped_dollar() {
    // ``eval "no\$x"`` — the ``\$`` is a backslash-escape, so
    // the lexer produces a single ESC token with no Var.  The
    // word-span scan must skip the next byte after ``\`` to
    // avoid mis-detecting the literal ``$``.
    let diags = w101_diags("eval no\\$x\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_w304_code_fix_inserts_terminator() {
    let src = "exec $cmd\n";
    let diags = w304_diags(src);
    assert_eq!(diags.len(), 1, "got {diags:?}");
    let fix = diags[0]
        .fixes
        .first()
        .expect("expected an insert-terminator fix");
    let mut applied = src.to_string();
    let start = fix.span.start() as usize;
    let end = fix.span.end() as usize;
    applied.replace_range(start..end, &fix.new_text);
    assert_eq!(applied, "exec -- $cmd\n", "got {applied:?}");
}

// -- body-walk and nested-cmd recovery
//
// Verify ``when EVENT { body }`` recurses (the registry
// ``arg_role_resolver`` records BODY at the last index)
// and that braced expr args (``Str`` tokens) have their
// outer braces unwrapped before the nested-``[cmd]`` scan
// (otherwise the scanner skips the entire braced region
// opaquely).

#[test]
fn compound_cmd_word_descends_substitution_fragments_only() {
    // A command substitution that is the *first* fragment of a
    // compound word (`[foo]bar`, `[foo]$x`) merges with the trailing
    // literal into one argv token spanning the whole word.  The
    // nested-invocation recorder must descend only the `[…]`
    // fragment(s) — not the merged span — so it records the real
    // inner head (`foo`) rather than a bogus one (`foo]bar`).
    type Inv = (&'static str, u32, u32);
    let cases: &[(&str, &[Inv])] = &[
        ("puts [foo]bar\n", &[("foo", 6, 9)]),
        ("set x [foo]$suffix\n", &[("foo", 7, 10)]),
        // Substitution as the command head of a compound word.
        ("[foo]bar hi\n", &[("foo", 1, 4)]),
        // Two substitutions in one word — both must be found.
        ("puts [foo]bar[baz]\n", &[("foo", 6, 9), ("baz", 14, 17)]),
        // `;`-separated commands inside a compound substitution.
        ("puts [aa; bb]cc\n", &[("aa", 6, 8), ("bb", 10, 12)]),
    ];
    for (src, expected) in cases {
        let mut a = Analyser::new();
        let r = a.analyse(src, "f5-irules");
        let got: Vec<(String, u32, u32)> = r
            .command_invocations
            .iter()
            .map(|c| (c.name.clone(), c.range.start(), c.range.end()))
            .collect();
        for &(name, start, end) in *expected {
            assert!(
                got.iter()
                    .any(|(n, s, e)| n == name && *s == start && *e == end),
                "src {src:?}: expected ({name:?}, {start}, {end}) in {got:?}",
            );
        }
        // No over-read: a *descended* substitution head must never
        // carry the literal suffix after the matching `]` (`foo]bar`,
        // `foo]${suffix}`).  A whole-word head recorded verbatim by
        // `process_command` (`[foo]bar`) legitimately starts with `[`
        // and is excluded.
        assert!(
            !got.iter()
                .any(|(n, _, _)| n.contains(']') && !n.starts_with('[')),
            "src {src:?}: a descended head leaked the literal suffix (over-read): {got:?}",
        );
    }
}

#[test]
fn analyse_when_body_records_inner_command_invocations() {
    // ``when HTTP_REQUEST { body }`` — ``call`` and the
    // target ``myhelper`` should appear in
    // ``command_invocations`` from the body recursion.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc myhelper {} {}\nwhen HTTP_REQUEST { call myhelper }\n",
        "f5-irules",
    );
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"call"), "got {names:?}");
    assert!(names.contains(&"myhelper"), "got {names:?}");
}

#[test]
fn analyse_braced_expr_arg_records_inner_substitution() {
    // ``if { [HTTP::uri] eq "/foo" } { ... }`` — the
    // ``[HTTP::uri]`` substitution inside the braced expr
    // arg must surface in ``command_invocations``.  Without
    // the ``Str`` unwrap, the nested-cmd scanner sees the
    // outer ``{`` and skips the entire braced region
    // opaquely.
    let mut a = Analyser::new();
    let r = a.analyse("if { [HTTP::uri] eq \"/foo\" } { puts ok }\n", "f5-irules");
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"HTTP::uri"), "got {names:?}");
}

#[test]
fn analyse_records_every_command_in_a_substitution() {
    // A command substitution can hold more than one command (`;`- /
    // newline-separated), and substitutions nest.  The CST descent
    // records every inner command head (`set x [foo; bar]` ->
    // {foo, bar, set}).
    // A head is recorded in `word_piece` form for every command,
    // incl. a `$var` head (`${var}`), a `"quoted"` head (unquoted),
    // and a compound head (`set x [$cmd a; bar]` -> {${cmd}, bar,
    // set}).
    let cases: &[(&str, &[&str])] = &[
        ("set x [foo; bar]\n", &["set", "foo", "bar"]),
        ("puts [a; b; c]\n", &["puts", "a", "b", "c"]),
        ("set x [foo [bar; baz]]\n", &["set", "foo", "bar", "baz"]),
        ("set x [foo $y; bar]\n", &["set", "foo", "bar"]),
        ("set x [$cmd a; bar]\n", &["set", "${cmd}", "bar"]),
        ("set x [foo; $y arg]\n", &["set", "foo", "${y}"]),
        ("set x [\"q\" a; bar]\n", &["set", "q", "bar"]),
        // A control-flow command inside a substitution has body
        // arguments whose commands are inner invocations too
        // (descend_command resolves them via the registry).
        // `[if {$c} {puts hi}]` -> {if, puts}.
        ("set x [if {$c} {puts hi}]\n", &["set", "if", "puts"]),
        (
            "set x [foreach a $l {log $a}]\n",
            &["set", "foreach", "log"],
        ),
        ("set x [eval {one; two}]\n", &["set", "eval", "one", "two"]),
        // A command substitution inside an *expr* argument of a
        // command nested in a substitution is an invocation too
        // (collect_expr_substitutions). `[if {[check]} {fwd}]`
        // -> {if, check, fwd}; `[expr {[bar] + 1}]` -> {expr, bar}.
        (
            "set x [if {[check]} {fwd}]\n",
            &["set", "if", "check", "fwd"],
        ),
        ("set x [expr {[bar] + 1}]\n", &["set", "expr", "bar"]),
        (
            "set x [while {[cond]} {act}]\n",
            &["set", "while", "cond", "act"],
        ),
        // argv_texts[0] is recorded for every command, incl. a
        // `[subst]` head (recorded *and* descended) and a `{braced}`
        // head (its inner text).
        ("set x [[gen] arg]\n", &["set", "[gen]", "gen"]),
        ("puts [[a] [b]]\n", &["puts", "[a]", "a", "b"]),
    ];
    for (src, expected) in cases {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        for want in *expected {
            assert!(
                names.contains(want),
                "{src:?}: missing {want:?}, got {names:?}"
            );
        }
    }
}

#[test]
fn analyse_records_switch_arm_bodies_not_patterns() {
    // The `switch … {pat body …}` list-form arg is a Tcl *list*,
    // not a script.  The arm *bodies* are scripts (their commands
    // are invocations), but the *patterns* are not — descending
    // the whole list as a script would mis-record a pattern
    // (`a`/`b`) as a command head.  Parse the pairs and descend
    // each body.  A `default` keyword / `-` fall-through is a
    // pattern, not a body.  Result: {cmd1, cmd2, set, switch}.
    let mut a = Analyser::new();
    let r = a.analyse("set x [switch $v {a {cmd1} b {cmd2}}]\n", "tcl8.6");
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"switch"), "got {names:?}");
    assert!(names.contains(&"cmd1"), "arm body missing: {names:?}");
    assert!(names.contains(&"cmd2"), "arm body missing: {names:?}");
    assert!(
        !names.contains(&"a"),
        "pattern recorded as command: {names:?}"
    );
    assert!(
        !names.contains(&"b"),
        "pattern recorded as command: {names:?}"
    );
}

#[test]
fn analyse_braced_data_word_is_not_over_recorded() {
    // A `[...]` inside a braced *data* word is literal (braces
    // suppress substitution), so it must not be recorded — only an
    // *expr* arg's substitutions are.  And a command whose name is
    // itself a substitution (`[x] hi`) must still be descended (the
    // head token is iterated too).
    let cases: &[(&str, &[&str], &[&str])] = &[
        // (source, must-contain, must-NOT-contain)
        ("set x {[noeval]}\n", &["set"], &["noeval"]),
        ("set d {literal data}\n", &["set"], &["data", "literal"]),
        ("proc p {} {[x] hi}\n", &["proc", "x"], &[]),
        ("if {[chk]} {puts ok}\n", &["if", "chk", "puts"], &[]),
    ];
    for (src, want, unwant) in cases {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        for w in *want {
            assert!(names.contains(w), "{src:?}: missing {w:?}, got {names:?}");
        }
        for u in *unwant {
            assert!(
                !names.contains(u),
                "{src:?}: over-recorded {u:?}, got {names:?}"
            );
        }
    }
}

#[test]
fn analyse_regexp_var_added_to_regex_vars_set() {
    // Side effect: the var name is recorded in
    // ``regex_vars`` so downstream consumers (var-as-regex
    // hint, future W*-codes) can find the defining set.
    let mut a = Analyser::new();
    let _ = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
    // The const-string scope is the global scope (path = []).
    assert!(a.regex_vars.contains(&(Vec::new(), "p".to_string())));
}

#[test]
fn analyse_alias_with_prepended_args_recorded() {
    // Prepended args after the target are stored on
    // ``extras``.
    let mut a = Analyser::new();
    let r = a.analyse("interp alias {} logerr {} puts stderr\n", "tcl");
    let alias = r
        .command_aliases
        .get("::logerr")
        .expect("::logerr recorded");
    assert_eq!(alias.target, "puts");
    assert_eq!(alias.extras.as_slice(), &["stderr"]);
}

#[test]
fn analyse_tcllib_import_wrapper_does_not_fire_on_namespace_import() {
    // The wrapper detector must not trip on Tcl's own
    // ``namespace import`` — that's handled by
    // ``handle_namespace_import_command`` and is never
    // conjectured.
    let mut a = Analyser::new();
    let r = a.analyse("namespace import ::foo::bar\n", "tcl");
    assert!(r.namespace_imports.iter().all(|i| !i.conjectured));
}

#[test]
fn analyse_chunked_seeds_file_suppression_minus_one_sentinel() {
    // ``analyse`` populates ``result.suppressed_lines[-1]`` with
    // the file-level ``# tcl-lsp: disable=`` set; verify
    // ``analyse_chunked`` does the same so consumers see the
    // file-wide directives via the same surface regardless of
    // which entry point dispatched.
    use crate::segmenter::SegmentedCommand;
    let mut a = Analyser::new();
    let cmds: Vec<Vec<SegmentedCommand>> = vec![Vec::new()];
    let (r, _) = a.analyse_chunked("# tcl-lsp: disable=W210,W211\nset x 1\n", cmds, "tcl");
    let codes = r.suppressed_lines.get(&-1).expect("-1 sentinel");
    assert!(codes.contains("W210"));
    assert!(codes.contains("W211"));
}

#[test]
fn analyse_commands_seeds_file_suppression_minus_one_sentinel() {
    // Same assertion through ``analyse_commands`` — the
    // snapshot-restore entry point.
    use crate::segmenter::SegmentedCommand;
    let mut a = Analyser::new();
    let cmds: Vec<SegmentedCommand> = Vec::new();
    let r = a.analyse_commands(
        "# tcl-lsp: disable=W210,W211\nset x 1\n",
        &cmds,
        "tcl",
        true,
    );
    let codes = r.suppressed_lines.get(&-1).expect("-1 sentinel");
    assert!(codes.contains("W210"));
    assert!(codes.contains("W211"));
}

// Incremental analysis differential oracle

/// A projection of `AnalysisResult` capturing the observable
/// identity an incremental analysis must preserve: sorted
/// `(code, start, end)` diagnostics, proc names, global vars, and
/// `(invocation name, start)` pairs.
type ResultProjection = (
    Vec<(String, u32, u32)>,
    Vec<String>,
    Vec<String>,
    Vec<(String, u32)>,
);

fn project_result(r: &AnalysisResult) -> ResultProjection {
    let mut diags: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.span.start(), d.span.end()))
        .collect();
    diags.sort();
    let mut procs: Vec<_> = r.all_procs.keys().cloned().collect();
    procs.sort();
    let mut vars: Vec<_> = r.global_scope.variables.keys().cloned().collect();
    vars.sort();
    let mut invs: Vec<_> = r
        .command_invocations
        .iter()
        .map(|i| (i.name.clone(), i.range.start()))
        .collect();
    invs.sort();
    (diags, procs, vars, invs)
}

struct IncrLcg(u64);
impl IncrLcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 33) as u32
    }
}

#[test]
#[ignore = "seeded edit-storm oracle (350 edit/analyse pairs); the deterministic \
                incremental gates stay in CI — the *_incremental_matches_fresh* family \
                and per_item_corpus::per_item_matches_analyse_over_repo_samples; \
                run explicitly with --ignored"]
fn analyse_incremental_matches_full_under_fuzz() {
    // The acceptance gate: a random edit applied to a base document,
    // then `analyse_incremental` must produce a result observably
    // identical to a full `analyse` of the edited text — diagnostics,
    // procs, globals, and recorded command invocations all match.
    let bases = [
        "set x 1\nputs hi\nproc p {} { return 1 }\np\nset y 2\n",
        "namespace eval n {\n  variable v 1\n  proc q {} { return $v }\n}\nn::q\n",
        "if {$a} {\n  puts a\n} else {\n  puts b\n}\nset z 3\nfoo $undef\n",
        "proc add {a b} { return [expr {$a + $b}] }\nputs [add 1 2]\nset s {x y z}\n",
        "set i 0\nwhile {$i < 10} { incr i }\nputs done\n# trailing comment\n",
    ];
    let inserts = [
        "",
        "z",
        "puts Z\n",
        "set q 9\n",
        " ",
        "\n",
        "x",
        "proc r {} {}\n",
        "incr i\n",
        "1",
    ];
    let mut rng = IncrLcg(0xD1FF_ACE5_1234_9876);
    let mut checked = 0usize;
    for base in bases {
        let prev_cmds = crate::segmenter::segment_commands(base);
        let blen = base.len();
        for _ in 0..70 {
            let mut s = (rng.next_u32() as usize) % (blen + 1);
            while !base.is_char_boundary(s) {
                s -= 1;
            }
            let mut e = s + (rng.next_u32() as usize) % (blen + 1 - s);
            while !base.is_char_boundary(e) {
                e += 1;
            }
            let ins = inserts[(rng.next_u32() as usize) % inserts.len()];
            let new = format!("{}{}{}", &base[..s], ins, &base[e..]);

            let mut af = Analyser::new();
            let full = af.analyse(&new, "tcl8.6");
            let mut ai = Analyser::new();
            let inc = ai.analyse_incremental(base, &prev_cmds, &new, "tcl8.6");
            assert_eq!(
                project_result(&inc),
                project_result(&full),
                "incremental != full for base {base:?} edit [{s},{e}) ins {ins:?} -> {new:?}",
            );
            checked += 1;
        }
    }
    assert!(checked > 300, "fuzz corpus too small: {checked}");
}

#[test]
fn analyse_incremental_falls_back_on_incomplete_and_stubs() {
    let base = "set x 1\nputs hi\n";
    let prev = crate::segmenter::segment_commands(base);
    // Incomplete (unterminated brace) -> full-analyse fallback, still
    // identical to a direct full analyse.
    let incomplete = "set x 1\nproc p {} {\n";
    let mut ai = Analyser::new();
    let inc = ai.analyse_incremental(base, &prev, incomplete, "tcl8.6");
    let mut af = Analyser::new();
    let full = af.analyse(incomplete, "tcl8.6");
    assert_eq!(project_result(&inc), project_result(&full));
    // Stub directive -> fallback path.
    let stubbed = "# tcl-lsp: stub mycmd\nmycmd a b\n";
    let mut ai2 = Analyser::new();
    let inc2 = ai2.analyse_incremental(base, &prev, stubbed, "tcl8.6");
    let mut af2 = Analyser::new();
    let full2 = af2.analyse(stubbed, "tcl8.6");
    assert_eq!(project_result(&inc2), project_result(&full2));
}
