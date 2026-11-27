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
use crate::command_binding::SourceCommandTransitionObligation;
use crate::registry_invocation::source_structure::OriginalRegistrySource;

fn subjects(
    result: &super::super::AnalysisResult,
) -> Vec<&ConditionalInterpreterVisibilitySubject> {
    result
        .diagnostics
        .iter()
        .filter_map(super::super::Diagnostic::conditional_interpreter_visibility)
        .collect()
}

#[test]
fn logical_lambda_cursor_frames_retain_actual_call_formals_and_nearest_scope() {
    fn lambda_scope(scope: &mut crate::analyser::Scope) -> Option<&mut crate::analyser::Scope> {
        if scope.requires_original_lambda_source_context {
            return Some(scope);
        }
        scope.children.iter_mut().find_map(lambda_scope)
    }
    let source = "apply {{} {set first 1; }}; apply {{} {set second 2; }}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    let first_offset = u32::try_from(source.find("first").unwrap()).unwrap();
    let second_offset = u32::try_from(source.find("second").unwrap()).unwrap();
    let first = analysis
        .original_variable_frame_in_source(&image, config, first_offset)
        .expect("actual first source call and empty formals");
    let second = analysis
        .original_variable_frame_in_source(&image, config, second_offset)
        .expect("actual second source call and empty formals");
    assert_ne!(first, second);
    assert!(first.source().text.try_text().unwrap().contains("first"));
    let mut report_changed = analysis.clone();
    lambda_scope(&mut report_changed.global_scope).unwrap().name =
        "unrelated report label".to_owned();
    assert_eq!(
        report_changed.original_variable_frame_in_source(&image, config, first_offset),
        Some(first.clone())
    );
    lambda_scope(&mut report_changed.global_scope)
        .unwrap()
        .original_logical_lambda_namespace = None;
    assert!(
        report_changed
            .original_variable_frame_in_source(&image, config, first_offset)
            .is_none()
    );
    let mut changed_config = config;
    changed_config.strict_quoting = !changed_config.strict_quoting;
    assert!(
        analysis
            .original_variable_frame_in_source(&image, changed_config, first_offset)
            .is_none()
    );
    assert!(
        analysis
            .original_variable_frame_in_source(
                &tcl_lexer::SourceImage::document(&source.replace("first", "other")),
                config,
                first_offset,
            )
            .is_none()
    );
    for source in [
        "proc outer {} {apply {{arg} {set inner 1}}}",
        "proc outer {} {apply {{{arg DEFAULT}} {set inner 1}} A B}",
        "proc outer {} {apply {{{arg D\\u0045FAULT}} {set inner 1}}}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let cursor = u32::try_from(source.find("inner").unwrap()).unwrap();
        assert!(
            analysis
                .original_variable_frame_in_source(&image, config, cursor)
                .is_none(),
            "{source}"
        );
    }
    let source = "apply {{} {set value {inert}; set done 1}}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let config = analysis.body_lexer_config.unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    assert!(
        analysis
            .original_variable_frame_in_source(
                &image,
                config,
                u32::try_from(source.find("inert").unwrap()).unwrap()
            )
            .is_none()
    );
    assert!(
        analysis
            .original_variable_frame_in_source(
                &image,
                config,
                u32::try_from(source.find("done").unwrap()).unwrap()
            )
            .is_some()
    );
    let source = "apply {{} {}}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    assert!(
        analysis
            .original_variable_frame_in_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
                u32::try_from(source.len() - 2).unwrap()
            )
            .is_some()
    );
}

#[test]
fn logical_lambda_formal_fields_keep_actual_defaults_names_and_current_source() {
    let source = "apply {{a {b B} args} {return $a} {$literal}} ONE";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let input = analyser.resolved_analysis_input();
    let command = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    )
    .remove(0);
    let receipt = analyser
        .original_interp_apply_body(command.argv[0], command.arg_tokens(), &[])
        .unwrap();
    let elements = analyser
        .parse_apply_lambda_elements(command.args(), command.arg_tokens())
        .unwrap();
    let formals = receipt
        .logical_formals_for_body_token(source, &input, elements[1].0, elements[0].0)
        .expect("selected source-only formals from the same complete lambda roster");
    assert_eq!(
        formals
            .parameters()
            .iter()
            .map(|p| (p.name.as_str(), p.default.as_deref()))
            .collect::<Vec<_>>(),
        [("a", None), ("b", Some("B")), ("args", None)]
    );
    for (name, expected) in formals.name_sources().iter().zip(["a", "b", "args"]) {
        assert_eq!(name.name(), expected.as_bytes());
        assert_eq!(&source[name.source_span().unwrap().as_range()], expected);
    }
    assert_eq!(formals.parameter_field().original_word(), &receipt.lambda);
    let mut foreign = elements[0].0;
    foreign.span = Span::new(0, 0);
    assert!(
        receipt
            .logical_formals_for_body_token(source, &input, elements[1].0, foreign)
            .is_none()
    );
    let mut changed = input.clone();
    changed.config.strict_quoting = !changed.config.strict_quoting;
    assert!(
        receipt
            .logical_formals_for_body_token(source, &changed, elements[1].0, elements[0].0)
            .is_none()
    );
    assert!(
        receipt
            .logical_formals_for_body_token(
                &source.replace("{b B}", "{b C}"),
                &input,
                elements[1].0,
                elements[0].0
            )
            .is_none()
    );
    let reference = analyser
        .result
        .namespace_refs
        .iter()
        .find(|reference| reference.original_name == "$literal")
        .expect("literal list-field data is not a variable substitution");
    assert_eq!(&source[reference.span.as_range()], "$literal");
    assert_eq!(reference.qualified_name, "::$literal");
    assert!(reference.original_name_input.is_none());
    for source in [
        r"apply {{{na\u006de}} {return}}",
        r"apply {{{a D\u0045FAULT}} {return}}",
        "apply {{{a B C}} {return}}",
        "apply {{{a(k)}} {return}}",
        "apply {{{n::a}} {return}}",
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        let input = analyser.resolved_analysis_input();
        let command = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .remove(0);
        let receipt = analyser
            .original_interp_apply_body(command.argv[0], command.arg_tokens(), &[])
            .expect("namespace geometry remains independent of formal validity");
        assert!(
            receipt
                .logical
                .as_ref()
                .unwrap()
                .formal_parameters()
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn lambda_handler_requires_the_selected_parameter_and_body_fields() {
    let source = "apply {{arg} {return $arg}} value";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let input = analyser.resolved_analysis_input();
    let command = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    )
    .remove(0);
    let receipt = analyser
        .original_interp_apply_body(command.argv[0], command.arg_tokens(), &[])
        .unwrap();
    let elements = analyser
        .parse_apply_lambda_elements(command.args(), command.arg_tokens())
        .unwrap();
    assert!(
        receipt
            .for_lambda_tokens(source, &input, elements[0].0, elements[1].0)
            .is_some()
    );
    let mut foreign_parameter = elements[0].0;
    foreign_parameter.span = Span::new(0, 1);
    assert!(
        receipt
            .for_lambda_tokens(source, &input, foreign_parameter, elements[1].0)
            .is_none()
    );
}

fn retained(scope: &super::super::types::Scope) -> Option<&OriginalInterpreterLambdaBody> {
    scope
        .original_logical_lambda_namespace
        .as_deref()
        .or_else(|| scope.children.iter().find_map(retained))
}

#[test]
fn logical_lambda_namespace_retains_original_fields_and_scope_applicability() {
    for (source, expected) in [
        ("apply {{} {set local 1}}", "::"),
        ("apply {{} {set local 1} sub}", "::sub"),
        ("apply {{} {set local 1} {$literal}}", "::$literal"),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        let input = analyser.resolved_analysis_input();
        let command = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .remove(0);
        let receipt = analyser
            .original_interp_apply_body(command.argv[0], command.arg_tokens(), &[])
            .expect("actual Logical call, source field roster and namespace recipe");
        let elements = analyser
            .parse_apply_lambda_elements(command.args(), command.arg_tokens())
            .unwrap();
        let token = elements[1].0;
        let (namespace, domain) = receipt.for_body_token(source, &input, token).unwrap();
        assert!(domain.is_none());
        assert_eq!(
            namespace
                .source_spelling(analyser.declaration_name_policy())
                .as_deref(),
            Some(expected)
        );
        assert!(receipt.has_logical_source_namespace());
        let logical = receipt.logical.as_ref().unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = logical.source_words().source()
        else {
            panic!("held original Logical applicability");
        };
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(
            advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability)
        );
        let scope_receipt =
            retained(&analyser.result.global_scope).expect("scope retains the full source owner");
        assert_eq!(
            scope_receipt
                .for_body_token(source, &input, token)
                .unwrap()
                .0,
            namespace
        );
        assert!(
            scope_receipt
                .logical
                .as_ref()
                .unwrap()
                .source_words()
                .matches_source(&SourceImage::document(source), input.lexer_config(),)
        );
        let mut changed = input.clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        assert!(receipt.for_body_token(source, &changed, token).is_none());
        assert!(
            receipt
                .for_body_token(&source.replace("local 1", "local 2"), &input, token)
                .is_none()
        );
        let mut foreign = token;
        foreign.span = Span::new(0, 0);
        assert!(receipt.for_body_token(source, &input, foreign).is_none());
    }
    let source = "namespace eval caller {apply {{} {set local 1}}}";
    let result = Analyser::new().analyse(source, "tcl");
    let receipt =
        retained(&result.global_scope).expect("namespace caller cannot donate lambda namespace");
    assert!(receipt.namespace.is_root());
    assert!(receipt.logical.is_some());
    for source in [
        "apply $lambda",
        "apply {{} {return} sub extra}",
        r"apply {{} {return} sub\ name}",
        "proc apply args {}; apply {{} {return}}",
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        let input = analyser.resolved_analysis_input();
        let command = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        assert!(
            analyser
                .original_interp_apply_body(command.argv[0], command.arg_tokens(), &[])
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_child_visibility_keeps_default_and_supplied_whole_parent_config() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // Conditional source visibility; no child interpreter/worker admission.
    let body = "interp create -safe s; interp eval s {source a.tcl}";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let source = format!("\u{feff}{body}");
    let result = Analyser::new().analyse(&source, "tcl9.1");
    let observed = subjects(&result);
    assert_eq!(observed.len(), 1, "{:?}", result.diagnostics);
    assert!(observed[0].matches_source(&SourceImage::document(&source), &input));
    assert!(
        observed[0]
            .original_words()
            .iter()
            .all(|word| word.config() == input.lexer_config())
    );
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut supplied = input.clone();
        supplied.config.leading_bom = bom;
        supplied.config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        let result = Analyser::new()
            .with_resolved_input(supplied.clone())
            .analyse(body, "display-only");
        let observed = subjects(&result);
        assert_eq!(observed.len(), 1, "{bom:?}: {:?}", result.diagnostics);
        let original = observed[0];
        let image = SourceImage::document(body);
        assert!(original.matches_source(&image, &supplied));
        assert!(
            original
                .original_words()
                .iter()
                .all(|word| word.image() == &image && word.config() == supplied.lexer_config())
        );
        assert!(
            !original.matches_source(&SourceImage::document(&(body.to_owned() + "\n")), &supplied)
        );
        assert!(!original.matches_source(&SourceImage::native(body.as_bytes()), &supplied));
        let mut foreign = supplied.clone();
        foreign.context = std::sync::Arc::new(
            supplied
                .context
                .with_command_store(std::sync::Arc::clone(supplied.context.commands())),
        );
        assert!(!original.matches_source(&image, &foreign));
        let mut stale = supplied.clone();
        stale.config.leading_bom = if bom == tcl_lexer::LeadingBom::Skip {
            tcl_lexer::LeadingBom::Content
        } else {
            tcl_lexer::LeadingBom::Skip
        };
        assert!(!original.matches_source(&image, &stale));
    }
}

#[test]
fn original_child_script_positions_retain_list_values_and_reference_timing() {
    // naming.source.original-structured-script-timing
    // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for body in [
            "uplevel [list source b.tcl]",
            "uplevel [list source {a b.tcl}]",
            "uplevel [list source [list b.tcl]]",
        ] {
            let source = format!("interp create -safe s; interp eval s {{{body}}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            let observed = subjects(&analysis);
            assert_eq!(
                observed.len(),
                1,
                "{dialect} {body}: {:?}",
                analysis.diagnostics
            );
            assert_eq!(observed[0].name_input().bytes(), b"source");
            assert_eq!(observed[0].original_words().len(), 3);
            assert!(observed[0].matches_source(
                &SourceImage::document(&source),
                analysis.resolved_input.as_ref().unwrap()
            ));
            assert!(
                observed[0].original_words()[1].span().start()
                    > u32::try_from(source.find("uplevel").unwrap()).unwrap()
            );
        }
        for body in [
            "uplevel [list $head b.tcl]",
            "uplevel [lindex {source b.tcl} 0]",
            "proc list args {return data}; uplevel [list source b.tcl]",
            "set data [list source b.tcl]",
            "trace remove variable x write [list exec ls]",
            "trace add $kind x write [list exec ls]",
        ] {
            let source = format!("interp create -safe s; interp eval s {{{body}}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            assert!(
                subjects(&analysis).is_empty(),
                "{dialect} {body}: {:?}",
                analysis.diagnostics
            );
        }
    }
}

#[test]
fn original_child_receiver_visibility_keeps_unknown_namespace_and_body_owner_explicit() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // naming.tcloo.original-source-receiver-body-declaration
    // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
    let source =
        "interp create -safe s; interp eval s {oo::class create C {method m {} {source a.tcl}}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let observed = subjects(&analysis);
    assert_eq!(observed.len(), 1, "{:?}", analysis.diagnostics);
    assert!(observed[0].namespace().is_none());
    assert!(observed[0].source_receiver_body_declaration().is_some());
    assert!(observed[0].declared_receiver_body().is_none());
    assert!(
        observed[0]
            .obligations()
            .contains(&InterpreterVisibilityObligation::UnknownExecutingReceiverNamespace)
    );
    assert!(
        observed[0]
            .obligations()
            .contains(&InterpreterVisibilityObligation::NoEarlierReceiverNamespaceBinding)
    );
    assert!(observed[0].matches_source(
        &SourceImage::document(source),
        analysis.resolved_input.as_ref().unwrap()
    ));
    assert!(observed[0].slot().namespace.is_root());
    assert_eq!(observed[0].slot().simple.as_bytes(), b"source");
    assert!(
        analysis.source_targets[0]
            .original_interpreter_source_load
            .as_ref()
            .unwrap()
            .source_receiver_body_declaration()
            .is_some()
    );
    let inert = "interp create -safe s; interp eval s {set data {method m {} {source a.tcl}}}";
    assert_eq!(
        subjects(&Analyser::new().analyse(inert, "tcl8.6")),
        [] as [&crate::analyser::interp_visibility::ConditionalInterpreterVisibilitySubject; 0]
    );
}

#[test]
fn original_created_handle_visibility_uses_the_same_child_source_snapshot() {
    // naming.interpreter.original-created-handle-source-body
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
    let handle = "interp create -safe s; s eval {source a.tcl}";
    let path = "interp create -safe s; interp eval s {source a.tcl}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let by_handle = Analyser::new().analyse(handle, dialect);
        let by_path = Analyser::new().analyse(path, dialect);
        assert_eq!(
            subjects(&by_handle).len(),
            1,
            "{dialect}: {:?}",
            by_handle.diagnostics
        );
        assert_eq!(
            subjects(&by_path).len(),
            1,
            "{dialect}: {:?}",
            by_path.diagnostics
        );
        assert_eq!(subjects(&by_handle)[0].slot(), subjects(&by_path)[0].slot());
        assert_eq!(
            subjects(&by_handle)[0].obligations(),
            subjects(&by_path)[0].obligations()
        );
        assert!(subjects(&by_handle)[0].matches_source(
            &SourceImage::document(handle),
            by_handle.resolved_input.as_ref().unwrap()
        ));
        assert_eq!(by_handle.source_targets.len(), 1);
        assert!(
            by_handle.source_targets[0]
                .original_interpreter_source_load
                .is_some()
        );
    }
    for source in [
        "interp create -safe s; rename s moved; s eval {source a.tcl}",
        "interp create -safe s; rename s moved; interp delete s; moved eval {source a.tcl}",
        "interp create -safe s; rename s moved; rename moved {}; moved eval {source a.tcl}",
        "interp create -safe s; mystery; s eval {source a.tcl}",
        "interp create -safe s; proc s args {}; s eval {source a.tcl}",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            subjects(&result).is_empty(),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn original_moved_handle_visibility_uses_the_original_child_path() {
    // naming.interpreter.original-created-handle-source-body
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
    // Independent CLI observations: naming.interpreter.original-created-parent-command-move
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md
    for source in [
        "interp create -safe s; rename s moved; moved eval {source a.tcl}",
        "interp create -safe s; rename s first; rename first second; second eval {source a.tcl}",
        "interp create -safe s; namespace eval N {}; rename s ::N::held; ::N::held eval {source a.tcl}",
    ] {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = Analyser::new().analyse(source, dialect);
            assert_eq!(subjects(&result).len(), 1, "{dialect}: {source}");
            assert_eq!(result.source_targets.len(), 1);
            let subject = subjects(&result)[0];
            assert!(subject.matches_source(
                &SourceImage::document(source),
                result.resolved_input.as_ref().unwrap()
            ));
            assert_eq!(subject.name_input().bytes(), b"source");
        }
    }
}

#[test]
fn original_child_path_visibility_survives_parent_command_moves() {
    // naming.interpreter.original-child-path-source-binding
    // docs/design/analysis/name-resolution-proofs/interpreter-original-child-path-source-binding.md
    // naming.interpreter.original-created-parent-command-move
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md
    // Native paths are measured independently; diagnostics and source edges
    // below remain conditional on successful original source operations.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            "interp create -safe s; rename s moved; interp eval s {source a.tcl}",
            "interp create -safe s; rename s first; rename first second; interp eval s {source a.tcl}",
            "interp create -safe s; namespace eval N {}; rename s ::N::held; interp eval s {source a.tcl}",
        ] {
            let result = Analyser::new().analyse(source, dialect);
            assert_eq!(subjects(&result).len(), 1, "{dialect}: {source}");
            assert_eq!(result.source_targets.len(), 1);
            assert!(
                !result
                    .diagnostics
                    .iter()
                    .any(|diag| diag.code == tcl_core_types::DiagCode::W140)
            );
        }
        let exposed = "interp create -safe s; rename s moved; interp expose s source; interp eval s {source a.tcl}";
        assert_eq!(
            subjects(&Analyser::new().analyse(exposed, dialect)),
            [] as [&crate::analyser::interp_visibility::ConditionalInterpreterVisibilitySubject; 0]
        );
        let deleted =
            "interp create -safe s; rename s moved; interp delete s; moved eval {source a.tcl}";
        assert_eq!(
            subjects(&Analyser::new().analyse(deleted, dialect)),
            [] as [&crate::analyser::interp_visibility::ConditionalInterpreterVisibilitySubject; 0]
        );
    }
    let unknown = "interp create -safe s; rename s moved; mystery; interp eval s {source a.tcl}";
    assert_eq!(
        subjects(&Analyser::new().analyse(unknown, "tcl8.6")),
        [] as [&crate::analyser::interp_visibility::ConditionalInterpreterVisibilitySubject; 0]
    );
}

#[test]
fn original_child_visibility_is_scoped_conditional_and_retains_possible_source_edges() {
    // Implementation contract: naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // Empirical C source controls: naming.interpreter.safe-visibility-source-controls
    // docs/design/analysis/name-resolution-proofs/interpreter-safe-visibility-source-controls.md
    // These assertions test source projections. CLI observations do not
    // supply a hidden allocation, entered interpreter or selected call.
    let source =
        "interp create -safe s\ninterp eval s {source a.tcl; ::source b.tcl; ::::source c.tcl}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let subjects = subjects(&result);
        assert_eq!(subjects.len(), 3, "{dialect}: {:?}", result.diagnostics);
        assert_eq!(result.source_targets.len(), 3, "{dialect}");
        let input = result.resolved_input.as_ref().unwrap();
        for target in &result.source_targets {
            let original = target.original_interpreter_source_load.as_ref().unwrap();
            assert!(original.matches_source(&SourceImage::document(source), input));
            assert_eq!(original.original_words().len(), 2);
            assert_eq!(original.path_word(), &original.original_words()[1]);
            assert!(!original.matches_source(&SourceImage::document("source foreign.tcl"), input));
        }
        for subject in subjects {
            assert!(subject.matches_source(&SourceImage::document(source), input));
            assert!(subject.slot().namespace.is_root());
            assert_eq!(subject.slot().simple.as_bytes(), b"source");
            assert_eq!(subject.obligations().len(), 6);
            assert_eq!(
                subject.original_words().first().unwrap().span(),
                subject.name_input().original_word_key().unwrap().span()
            );
        }
    }
    let jim = Analyser::new().analyse(source, "jimtcl");
    assert!(
        subjects(&jim).is_empty(),
        "Jim does not inherit the C child surface"
    );
}

#[test]
fn original_child_visibility_preserves_literal_colon_and_independent_allocations() {
    // Implementation contract: naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // Exact colon and hidden/visible allocation CLI controls are recorded
    // in interpreter-safe-visibility-source-controls.md. This source ledger
    // remains conditional and never replaces an actual hidden-slot gate.
    let source = "interp create -safe s\ninterp eval s {proc :source {} {return literal}; namespace eval :ns {proc source {} {return scoped}}; :source; :ns::source; source a.tcl}\ninterp expose s source :visible\ninterp eval s {:visible a.tcl; source b.tcl}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        assert_eq!(
            subjects(&result).len(),
            2,
            "{dialect}: {:?}",
            result.diagnostics
        );
        assert!(
            subjects(&result)
                .iter()
                .all(|subject| subject.name_input().bytes() == b"source")
        );
    }
    let source = "interp create -safe s\ninterp eval s {proc source {} {return visible}; source}\ninterp expose s source :original\ninterp eval s {source; :original a.tcl; rename source {}; source b.tcl}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(subjects(&result).len(), 1, "{:?}", result.diagnostics);
    assert_eq!(subjects(&result)[0].name_input().bytes(), b"source");
}

#[test]
fn original_child_lambda_visibility_uses_its_own_literal_namespace() {
    // Implementation contract: naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // Independent original source controls: naming.procedure.original-root-and-colon-holder-publication
    // docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md
    // This checks a retained original literal lambda and source lookup
    // premises, never an entered lambda or runtime namespace allocation.
    let source = "interp create -safe s\ninterp eval s {namespace eval :ns {proc source {} {return local}; apply {{} {source a.tcl} :ns}; apply {{} {source b.tcl}}}}";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let visibility = subjects(&result);
        // C85 publishes the procedure in reparsed ns, matching Apply's
        // independently ::-prefixed :ns. C86+ publishes it in :ns while
        // Apply still selects ns, so both original source calls may miss.
        let expected = if dialect == "tcl8.5" {
            vec!["source b.tcl"]
        } else {
            vec!["source a.tcl", "source b.tcl"]
        };
        let actual = visibility
            .iter()
            .map(|subject| {
                subject
                    .name_input()
                    .original_word_key()
                    .unwrap()
                    .span()
                    .start()
            })
            .collect::<Vec<_>>();
        let expected = expected
            .iter()
            .map(|text| u32::try_from(source.find(text).unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{dialect}");
    }
    let shadow = "interp create -safe s\ninterp eval s {proc apply {args} {return}; apply {{} {source data.tcl}}}";
    let result = Analyser::new().analyse(shadow, "tcl8.6");
    assert!(
        subjects(&result).is_empty(),
        "a source-defined callable supplies no Apply schema"
    );
}

#[test]
fn original_child_visibility_rejects_foreign_source_configuration_and_generation() {
    // Implementation contract: naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let source = "interp create -safe s\ninterp eval s {source a.tcl}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let original = subjects(&result)[0];
    let input = result.resolved_input.as_ref().unwrap();
    let image = SourceImage::document(source);
    assert!(original.matches_source(&image, &input.clone()));
    assert!(!original.matches_source(&SourceImage::document(&(source.to_owned() + "\n")), input));
    let mut foreign = input.clone();
    foreign.context = Arc::new(
        input
            .context
            .with_command_store(Arc::clone(input.context.commands())),
    );
    assert!(!original.matches_source(&image, &foreign));
    let mut config = input.clone();
    config.config.leading_bom = if config.config.leading_bom == tcl_lexer::LeadingBom::Skip {
        tcl_lexer::LeadingBom::Content
    } else {
        tcl_lexer::LeadingBom::Skip
    };
    assert!(!original.matches_source(&image, &config));
    let native_image = SourceImage::native(source.as_bytes());
    assert!(!original.matches_source(&native_image, input));
}
