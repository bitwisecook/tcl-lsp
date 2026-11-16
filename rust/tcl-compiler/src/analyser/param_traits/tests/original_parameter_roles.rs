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

//! Which formal input a body occurrence still denotes: original and Logical parameter roles, nested frames, replaced formals and callback mutations.

use super::*;

fn assert_logical_parameter_shape_refusals(
    text: &str,
    env: TraitScanEnv<'_>,
    input: &crate::analyser::ResolvedAnalysisInput,
) {
    assert!(infer_param_traits(&["other"], text, env).is_empty());
    assert!(infer_param_traits(&["name", "other"], text, env).is_empty());
    assert!(infer_param_traits_deep(&["name", "other"], text, env).is_empty());
    assert!(caller_frame_upvar_params(&["name", "other"], text, env).is_empty());
    assert!(infer_param_traits(&["name"], &(text.to_owned() + " "), env).is_empty());
    let prefix = &text[..text.len() - 1];
    assert!(infer_param_traits(&["name"], prefix, env).is_empty());
    assert!(infer_param_traits_deep(&["name"], prefix, env).is_empty());
    assert!(caller_frame_upvar_params(&["name"], prefix, env).is_empty());
    assert!(caller_frame_literal_targets(prefix, env).is_empty());
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    assert!(infer_param_traits(&["name"], text, TraitScanEnv { config, ..env }).is_empty());
    assert!(
        infer_param_traits(
            &["name"],
            text,
            TraitScanEnv {
                original_logical_body: None,
                ..env
            }
        )
        .is_empty()
    );
}

fn assert_no_closed_logical_parameter_roles(
    view: &crate::registry_invocation::InvocationBodyAssistance,
) {
    // This current formal read retains real observer/lookup debt. Its
    // source recipe must never become a closed execution candidate.
    assert!(!view.parameter_role_advice().is_closed());
    assert_eq!(view.parameter_role_advice().candidates(), []);
    assert!(view.parameter_invocation(&["name"]).is_none());
    assert!(view.parameter_invocation(&["other"]).is_none());
    assert!(view.caller_name_template_invocation().is_none());
}

fn assert_no_definite_parameter_invocation(
    view: &crate::registry_invocation::InvocationBodyAssistance,
) {
    assert!(view.definite_invocation.is_none());
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
}

#[test]
fn original_logical_parameter_roles_keep_authored_body_and_formals_without_entry() {
    let source = "proc reader {name} {upvar 1 $name cell; set cell 1}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let realm = analysis.retained_command_realm().unwrap();
    let cards = realm.original_logical_procedure_body_cards(input).unwrap();
    let [body] = cards.as_slice() else {
        panic!("one authentic original procedure body");
    };
    let original = body.scope().scope_body();
    let content = original.content_span();
    let text = &source[content.as_range()];
    assert!(
        realm
            .executed_script_for_word(original.original_container().span())
            .is_none()
    );
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(analysis.resolved_registry().unwrap(), None),
        config: input.lexer_config(),
        identities: realm,
        executed_source: None,
        original_logical_body: Some(body),
    };
    let traits = infer_param_traits(&["name"], text, env);
    assert!(traits["name"].contains(&ProcArgTrait::VarWrite));
    assert!(traits["name"].contains(&ProcArgTrait::VarRead));
    assert_eq!(
        caller_frame_upvar_params(&["name"], text, env),
        HashSet::from(["name".to_owned()])
    );
    assert_eq!(
        analysis
            .original_procedure_declarations()
            .next()
            .unwrap()
            .metadata()
            .param_traits,
        traits
    );
    assert_logical_parameter_shape_refusals(text, env, input);
    let command =
        segment_commands_with_offset_and_config(text, 0, input.lexer_config().nested()).remove(0);
    let view = crate::registry_invocation::logical_parameter_body_assistance(
        &env.surface,
        realm,
        body,
        text,
        &command,
        content.start(),
    )
    .unwrap();
    assert_no_closed_logical_parameter_roles(&view);
    let shape = view.conditional_parameter_source_shape(&["name"]).unwrap();
    let selected = view
        .conditional_parameter_source_invocation(&["name"])
        .unwrap();
    assert_eq!(shape.command, selected.facts.canonical_command);
    assert_eq!(
        shape.written_argument_roles(),
        selected.written_argument_roles()
    );
    assert!(
        view.conditional_parameter_source_shape(&["other"])
            .is_none()
    );
    assert!(
        view.conditional_parameter_source_invocation(&["other"])
            .is_none()
    );
    let source_words = crate::registry_invocation::source_structure::source_registry_words_at(
        source,
        &analysis,
        content.start(),
    )
    .unwrap();
    let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
        advice,
    ) = source_words.source()
    else {
        panic!("actual source advice");
    };
    let prefix = body
        .parameter_source_prefix(advice.original_words())
        .unwrap();
    assert!(prefix.matches_body(body, input));
    assert_eq!(prefix.operations().len(), 1);
    assert_eq!(prefix.formal_read_count(), 1);
    assert_eq!(
        prefix.current().unwrap().original_words(),
        advice.original_words()
    );
    assert_ne!(prefix.current().unwrap().uncertain_operations(), []);
    assert!(prefix.current().unwrap().obligations().contains(
        &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
    ));
    assert_no_definite_parameter_invocation(&view);
}

#[test]
fn original_logical_parameter_source_roles_keep_ordinary_frame_policy_unavailable() {
    let source = "proc reader {name} {upvar 1 $name cell; set cell 1}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let realm = analysis.retained_command_realm().unwrap();
    let cards = realm.original_logical_procedure_body_cards(input).unwrap();
    let [body] = cards.as_slice() else {
        panic!("one authentic original procedure body");
    };
    let content = body.scope().scope_body().content_span();
    let text = &source[content.as_range()];
    let segment =
        segment_commands_with_offset_and_config(text, 0, input.lexer_config().nested()).remove(0);
    let view = crate::registry_invocation::logical_parameter_body_assistance(
        &DocumentCommandSurface::new(analysis.resolved_registry().unwrap(), None),
        realm,
        body,
        text,
        &segment,
        content.start(),
    )
    .expect("actual own-body Logical upvar source prefix");
    let selected = view
        .conditional_parameter_source_invocation(&["name"])
        .expect("dedicated conditional source invocation");
    let shape = view.conditional_parameter_source_shape(&["name"]).unwrap();
    assert!(!selected.facts.arg_roles_complete);
    assert_eq!(
        selected.retained_invocation().written_argument_roles(),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(std::ptr::eq(
        selected.facts,
        selected.retained_invocation().facts.as_ref()
    ));
    assert_eq!(
        selected.written_argument_roles(),
        shape.written_argument_roles()
    );
    assert!(shape.roles_complete);
    assert_ne!(
        shape.written_argument_roles(),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(view.parameter_role_advice().candidates(), []);
    assert!(!view.parameter_role_advice().is_closed());
    assert!(view.parameter_invocation(&["name"]).is_none());
    assert!(view.caller_name_template_invocation().is_none());
    assert!(view.definite_invocation.is_none());
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(
        view.conditional_parameter_source_shape(&["other"])
            .is_none()
    );

    for (fragment, count) in [("upvar 1", 1), ("set cell", 2)] {
        let offset = u32::try_from(source.find(fragment).unwrap()).unwrap();
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source, &analysis, offset,
        )
        .unwrap();
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            panic!("genuine actual source advice");
        };
        let prefix = body
            .parameter_source_prefix(advice.original_words())
            .unwrap();
        assert!(prefix.matches_body(body, input));
        assert_eq!(prefix.operations().len(), count);
        assert_eq!(prefix.formal_read_count(), 1);
        assert_ne!(prefix.current().unwrap().uncertain_operations(), []);
        assert!(prefix.current().unwrap().obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        ));
    }
}

#[test]
fn original_logical_parameter_roles_require_original_formal_order_before_role_application() {
    let source = "proc reader {name spare} {set $name 1}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let realm = analysis.retained_command_realm().unwrap();
    let cards = realm.original_logical_procedure_body_cards(input).unwrap();
    let [body] = cards.as_slice() else {
        panic!("one original procedure");
    };
    let text = &source[body.scope().scope_body().content_span().as_range()];
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(analysis.resolved_registry().unwrap(), None),
        config: input.lexer_config(),
        identities: realm,
        executed_source: None,
        original_logical_body: Some(body),
    };
    assert!(
        infer_param_traits(&["name", "spare"], text, env)["name"]
            .contains(&ProcArgTrait::DynamicNameLocal)
    );
    for parameters in [
        vec!["spare", "name"],
        vec!["name", "spare", "extra"],
        vec!["name"],
    ] {
        assert!(infer_param_traits(&parameters, text, env).is_empty());
        assert!(infer_param_traits_deep(&parameters, text, env).is_empty());
        assert!(caller_frame_upvar_params(&parameters, text, env).is_empty());
    }
}

#[test]
fn original_logical_parameter_roles_keep_nested_frames_separate_from_same_frame_bodies() {
    for (source, expected) in [
        ("proc outer {name} {proc inner {name} {$name}}", false),
        ("proc outer {name} {if 1 {$name}}", true),
        ("proc outer {name} {if 1 {[$name]}}", true),
        ("proc outer {name} {if 1 $name}", false),
        ("proc outer {name} {if 1 [list $name]}", false),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let cards = realm.original_logical_procedure_body_cards(input).unwrap();
        let body = cards
            .iter()
            .find(|body| body.name() == Some("outer"))
            .unwrap();
        let text = &source[body.scope().scope_body().content_span().as_range()];
        let env = TraitScanEnv {
            surface: DocumentCommandSurface::new(analysis.resolved_registry().unwrap(), None),
            config: input.lexer_config(),
            identities: realm,
            executed_source: None,
            original_logical_body: Some(body),
        };
        let traits = infer_param_traits_deep(&["name"], text, env);
        assert_eq!(
            traits
                .get("name")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Command)),
            expected,
            "{source}"
        );
    }
}

#[test]
fn original_logical_parameter_roles_preserve_actual_body_lookup_barriers() {
    for source in [
        "proc reader {name} {unknown_effect; set $name 1}",
        "proc set {args} {}; proc reader {name} {set $name 1}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let record = analysis
            .original_procedure_declarations()
            .find(|record| record.metadata().name == "reader")
            .unwrap();
        assert!(record.metadata().param_traits.is_empty(), "{source}");
    }
}

#[test]
fn conditional_parameter_source_prefix_keeps_unknown_callbacks_and_scope_mutations_terminal() {
    for source in [
        "proc reader {name} {unknown_effect; set $name 1}",
        "proc reader {name} {eval $name; set $name 1}",
        "proc reader {name} {set tmp [unknown_effect]; set $name 1}",
        "proc reader {name} {trace add variable name read callback; set $name 1}",
        "proc reader {name} {proc set {args} {}; set $name 1}",
        "proc reader {name} {rename set old; set $name 1}",
        "proc reader {name} {if 1 {unknown_effect}; set $name 1}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let cards = realm.original_logical_procedure_body_cards(input).unwrap();
        let body = cards
            .iter()
            .find(|body| body.name() == Some("reader"))
            .unwrap();
        let offset = u32::try_from(source.rfind("set $name 1").unwrap()).unwrap();
        let current = crate::registry_invocation::source_structure::source_registry_words_at(
            source, &analysis, offset,
        );
        if let Some(current) = current {
            let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = current.source() else { continue; };
            assert!(
                body.parameter_source_prefix(advice.original_words())
                    .is_none(),
                "{source}"
            );
        }
        // An earlier eval can still report Eval at its own source point;
        // it cannot donate later dynamic-name or alias-write observations.
        let traits = &analysis
            .original_procedure_declarations()
            .find(|row| row.metadata().name == "reader")
            .unwrap()
            .metadata()
            .param_traits;
        assert!(
            !traits
                .get("name")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::DynamicNameLocal)),
            "{source}"
        );
    }
}

#[test]
fn original_parameter_scans_keep_parent_input_and_nested_body_bom() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Conditional own-body/caller-name advice only; no entered caller or alias link.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut supplied = input.clone();
        supplied.config.leading_bom = bom;
        supplied.config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        for per_item in [false, true] {
            let mut analyser =
                crate::analyser::Analyser::new().with_resolved_input(supplied.clone());
            let source = "proc p {body name} {upvar 1 $name local; set local VALUE; eval $body}";
            let result = if per_item {
                analyser.analyse_per_item(source, "display-only")
            } else {
                analyser.analyse(source, "display-only")
            };
            assert_eq!(result.resolved_input.as_ref(), Some(&supplied));
            let procedure = result
                .all_procs
                .get("::p")
                .expect("genuine original procedure");
            assert_trait(&procedure.param_traits, "body", ProcArgTrait::Eval);
            assert!(procedure.caller_frame_params.contains("name"));

            // The original order first evaluates an unknown script. Its
            // later upvar cannot acquire the preceding positive's selection.
            let mut analyser =
                crate::analyser::Analyser::new().with_resolved_input(supplied.clone());
            let source = "proc p {body name} {eval $body; upvar 1 $name local; set local VALUE}";
            let result = if per_item {
                analyser.analyse_per_item(source, "display-only")
            } else {
                analyser.analyse(source, "display-only")
            };
            assert_eq!(result.resolved_input.as_ref(), Some(&supplied));
            let procedure = result
                .all_procs
                .get("::p")
                .expect("original unknown-eval procedure");
            assert_trait(&procedure.param_traits, "body", ProcArgTrait::Eval);
            assert!(procedure.caller_frame_params.is_empty());

            let mut analyser =
                crate::analyser::Analyser::new().with_resolved_input(supplied.clone());
            let source = "proc p {body name} {\u{feff}eval $body; \u{feff}upvar 1 $name local; set local VALUE}";
            let result = if per_item {
                analyser.analyse_per_item(source, "display-only")
            } else {
                analyser.analyse(source, "display-only")
            };
            let procedure = result
                .all_procs
                .get("::p")
                .expect("genuine original procedure");
            assert!(
                !procedure
                    .param_traits
                    .get("body")
                    .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
            );
            assert!(procedure.caller_frame_params.is_empty());
        }
    }
}

#[test]
fn replaced_formal_does_not_represent_its_incoming_value() {
    let replaced = infer(&["body"], "set body {}; eval $body");
    assert!(
        !replaced
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
    let copied = infer(&["body"], "set local $body; set body {}; eval $local");
    assert!(
        copied
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn callback_mutations_follow_the_actual_selected_cell() {
    let helper = "proc replace {name} {upvar 1 $name local; set local {}};";
    let replaced = infer(
        &["body"],
        &format!("{helper} set local $body; replace local; eval $local"),
    );
    assert!(
        !replaced
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
    let untouched = infer(
        &["body"],
        &format!("{helper} set local $body; replace body; eval $local"),
    );
    assert!(
        untouched
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn iteration_binding_retires_the_original_formal_input() {
    let replaced = infer_owned(&["body"], "foreach body {{}} {eval $body}", true);
    assert!(
        !replaced
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
    let untouched = infer_owned(&["body"], "foreach other {{}} {eval $body}", true);
    assert!(
        untouched
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn original_formal_reads_preserve_literal_variable_names() {
    for (name, reference) in [("a$b", "${a$b}"), ("1", "$1"), ("é", "${é}")] {
        let body = format!("eval {reference}");
        let result = infer(&[name], &body);
        assert!(
            result
                .get(name)
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval)),
            "{name:?}: {result:?}"
        );
    }
}

#[test]
fn nested_command_reads_do_not_name_the_outer_argument() {
    let result = infer(&["body"], "set [string length $body] 1");
    assert!(
        !result
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::DynamicNameLocal))
    );
}
