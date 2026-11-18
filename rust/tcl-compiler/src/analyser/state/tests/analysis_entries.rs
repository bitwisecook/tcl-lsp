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

//! The analysis entries: retained inputs and overlays, suppression directives, pre-segmented, chunked and incremental analysis.

use super::*;

/// A source shell keeps the analysed document and its resolved input and
/// interprets nothing: no invocation, declaration or diagnostic, where the
/// ordinary analysis of the same text records all three. Dialect-invariant;
/// run under Tcl 8.6.
#[test]
fn a_source_shell_keeps_the_document_and_interprets_nothing() {
    let source = "proc p {} { set x 1 }\np\nundefined_command_here\n";
    let shell = Analyser::new().source_shell().analyse(source, "tcl8.6");
    assert!(shell.is_source_shell());
    assert_eq!(shell.command_invocations.len(), 0);
    assert_eq!(shell.diagnostics.len(), 0);
    assert!(shell.resolved_input.is_some());
    let image = shell
        .retained_command_realm()
        .and_then(crate::realm::CommandBindingRealm::original_source_image)
        .expect("the shell retains the document");
    assert_eq!(image.bytes(), source.as_bytes());

    let analysed = Analyser::new().analyse(source, "tcl8.6");
    assert!(!analysed.is_source_shell());
    assert_ne!(analysed.command_invocations.len(), 0);
    assert_ne!(analysed.diagnostics.len(), 0);
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
    let supplied = super::super::super::input::ResolvedAnalysisInput::new(
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
        let input = super::super::super::input::ResolvedAnalysisInput::new(
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
    let input = super::super::super::input::ResolvedAnalysisInput::new(
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
