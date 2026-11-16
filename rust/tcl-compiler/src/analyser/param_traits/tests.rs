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
use tcl_registry::CommandRegistry;

#[test]
fn selected_lambda_source_refuses_a_foreign_document_image() {
    let source = "proc f {body} {apply {x {eval $x}} $body}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let offset = u32::try_from(source.find("apply").expect("apply head")).unwrap();
    let changed = source.replace("$body", "$other");
    assert!(
        selected_lambda_source(
            &changed,
            &analysis,
            Some(offset),
            0,
            analysis.body_lexer_config.expect("analysed lexer config"),
        )
        .is_none()
    );
}

fn assert_trait(
    traits: &HashMap<String, HashSet<ProcArgTrait>>,
    param: &str,
    expected: ProcArgTrait,
) {
    let set = traits
        .get(param)
        .unwrap_or_else(|| panic!("no traits for {param}"));
    assert!(
        set.contains(&expected),
        "{param}: expected {expected:?}, got {set:?}",
    );
}

/// Retain an uncalled declaration with its exact formals and body source.
/// Successful own-body advice requires this receipt; no caller is invented.
fn infer(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_owned(params, body, false)
}

fn infer_owned(params: &[&str], body: &str, deep: bool) -> HashMap<String, HashSet<ProcArgTrait>> {
    with_receipt(params, body, |env| {
        if deep {
            infer_param_traits_deep(params, body, env)
        } else {
            infer_param_traits(params, body, env)
        }
    })
}

/// Run `scan` over the receipt [`infer`] retains for `body` declared with
/// exactly `params` as its formals.
fn with_receipt<R>(params: &[&str], body: &str, scan: impl FnOnce(TraitScanEnv<'_>) -> R) -> R {
    let registry = CommandRegistry::build_default();
    let formals = tcl_syntax::list::join_list(params);
    let source = tcl_syntax::list::join_list(["proc", "__trait_probe", &formals, body]);
    let identities = crate::realm::document_realm_bindings(
        &source,
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        &registry,
    );
    let command = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default())
        .pop()
        .unwrap();
    let script = identities
        .executed_script_for_word(command.argv.last().unwrap().span)
        .unwrap();
    assert_eq!(script.text.try_text().unwrap(), body);
    scan(TraitScanEnv {
        surface: DocumentCommandSurface::new(&registry, None),
        config: LexerConfig::default(),
        identities: &identities,
        executed_source: Some(script),
        original_logical_body: None,
    })
}

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

/// A scan environment over `registry` with no document declarations and
/// no document binding facts — what a bare-body unit test scans under.
fn env_for(registry: &CommandRegistry, config: LexerConfig) -> TraitScanEnv<'_> {
    TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, None),
        config,
        executed_source: None,
        original_logical_body: None,
        identities: if registry.profile().is_some() {
            crate::realm::CommandBindingRealm::none()
        } else {
            static ENTRY: std::sync::OnceLock<crate::realm::CommandBindingRealm> =
                std::sync::OnceLock::new();
            ENTRY.get_or_init(|| {
                crate::realm::document_realm_bindings(
                    "",
                    tcl_dialect::DialectProfile::find("tcl9.0").expect("test native core"),
                    tcl_registry::default_registry(),
                )
            })
        },
    }
}

/// [`env_for`] with the document's own declarations attached.
fn env_with_declarations<'a>(
    registry: &'a CommandRegistry,
    declared: &'a tcl_registry::model::DeclaredSurface,
) -> TraitScanEnv<'a> {
    TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, Some(declared)),
        ..env_for(registry, LexerConfig::default())
    }
}

fn lexical_reference_values<'p>(
    source_word: &str,
    parameters: &[&'p str],
    config: LexerConfig,
) -> ArgumentValues<'p> {
    let registry = CommandRegistry::build_default();
    let source = format!("command {source_word}");
    let commands = segment_commands_with_offset_and_config(&source, 0, config);
    assert_eq!(commands.len(), 1);
    let parameters = parameters.iter().copied().collect();
    ArgumentValues::lexical(
        &source,
        &commands[0],
        &parameters,
        &Aliases::default(),
        config,
        &registry,
    )
}

#[test]
fn extract_var_name_simple() {
    let parameters = ["foo", "1abc"];
    let whole =
        |source| lexical_reference_values(source, &parameters, LexerConfig::default()).argument(0);
    assert_eq!(whole("$foo"), Some("foo"));
    assert_eq!(whole("${foo}"), Some("foo"));
    assert_eq!(whole("foo"), None);
    assert_eq!(whole("$"), None);
    assert_eq!(whole("$1abc"), Some("1abc"));
}

#[test]
fn eval_param_records_eval_trait() {
    let traits = infer(&["body"], "eval $body");
    assert_trait(&traits, "body", ProcArgTrait::Eval);
}

/// The *deep* pass
/// (`infer_param_traits_deep`, which alone recurses into braced body
/// arguments) must reach real commands *inside* an `apply` lambda body,
/// not misread the whole `{argList} {body}` blob as one script (which
/// would treat the parameter word as a command name and never find the
/// `eval` at all) — mirrors `declarations_deep_recurse_through_declared_body_args`,
/// swapping the registry-known `apply`/`LambdaLiteral` shape in for a
/// stub-declared `Body` shape. The lambda's own param is `x`, bound to
/// the literal `1` — an enclosing `body` param is neither the lambda's
/// param nor forwarded into the call, so it must record nothing: a lambda
/// body runs in a fresh frame, and a same-named enclosing param is not
/// implicitly in scope there.
#[test]
fn eval_inside_apply_lambda_body_does_not_leak_to_unrelated_enclosing_param() {
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits_deep(
        &["body"],
        "apply {x {eval $body}} 1",
        env_for(&registry, LexerConfig::default()),
    );
    assert!(
        !traits
            .get("body")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "an apply lambda's fresh frame must not leak a same-named \
             enclosing param's trait when that param is never forwarded \
             into the call; got {traits:?}"
    );
}

/// The real forwarding case: the lambda's own param `x` is `eval`'d
/// inside its body, and the enclosing `body` param is passed as the
/// actual argument that binds to `x` — so `body`'s value genuinely does
/// flow into an `eval`, and the trait must propagate back to it.
#[test]
fn eval_param_forwarded_into_apply_lambda_records_eval_trait() {
    let traits = infer_owned(&["body"], "apply {x {eval $x}} $body", true);
    assert_trait(&traits, "body", ProcArgTrait::Eval);
}

#[test]
fn trait_inference_is_dialect_aware_via_expand_syntax() {
    // `extract_commands` / `scan_deep` re-segment under the document
    // dialect, so the body's `{*}` is the expansion operator on 8.5+
    // but a literal brace word on 8.4.  `eval {*}$p` is therefore
    // `eval $p` (→ `p` is Eval) on 9.0 but `eval` of the single
    // composite word `{*}$p` (no clean `$p`, no trait) on 8.4.
    let registry = CommandRegistry::build_default();
    let on = infer(&["p"], "eval {*}$p");
    assert!(
        on.get("p").is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "9.0 expands `{{*}}` → eval $p → Eval: {on:?}",
    );
    let off = infer_param_traits(
        &["p"],
        "eval {*}$p",
        env_for(&registry, LexerConfig::for_dialect("tcl8.4")),
    );
    assert!(
        off.get("p")
            .is_none_or(|s| !s.contains(&ProcArgTrait::Eval)),
        "8.4 treats `{{*}}` as literal → no clean $p → no Eval: {off:?}",
    );
}

#[test]
fn uplevel_records_eval_on_last_arg_only() {
    let traits = infer(&["lvl", "script"], "uplevel $lvl $script");
    assert_trait(&traits, "script", ProcArgTrait::Eval);
    assert!(
        !traits
            .get("lvl")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval))
    );
}

/// Eval follows the registry's evaluation traits: every word of a script
/// that concatenates its arguments (`uplevel 1 $a $b` runs `$a $b`), a
/// script word of any evaluating command, and a `::`-rooted spelling.
#[test]
fn eval_reads_the_registry_evaluation_traits() {
    let traits = infer(&["a", "b"], "uplevel 1 $a $b");
    assert_trait(&traits, "a", ProcArgTrait::Eval);
    assert_trait(&traits, "b", ProcArgTrait::Eval);
    assert_trait(
        &infer(&["s"], "namespace eval ::ns $s"),
        "s",
        ProcArgTrait::Eval,
    );
    assert_trait(&infer(&["s"], "::eval $s"), "s", ProcArgTrait::Eval);
}

/// Both trackers read the `VarWrite` role: any write of an `upvar` alias
/// writes the caller's variable, a read through it does not, and any write
/// of a local ends the parameter value it carried.
#[test]
fn the_copy_trackers_read_the_var_write_role() {
    assert_trait(
        &infer(&["v"], "upvar 1 $v arr\narray set arr {k 1}"),
        "v",
        ProcArgTrait::VarWrite,
    );
    let traits = infer(&["v"], "upvar 1 $v local\nreturn [set local]");
    assert!(!traits["v"].contains(&ProcArgTrait::VarWrite), "{traits:?}");
    assert!(!infer(&["v"], "set n $v\nlassign {x} n\nset $n 1").contains_key("v"));
}

#[test]
fn foreach_records_loop_list_and_body() {
    let traits = infer(&["items", "body"], "foreach x $items $body");
    assert_trait(&traits, "items", ProcArgTrait::LoopList);
    assert_trait(&traits, "body", ProcArgTrait::Body);
}

#[test]
fn while_records_expr_and_body() {
    let traits = infer(&["cond", "body"], "while $cond $body");
    assert_trait(&traits, "cond", ProcArgTrait::Expr);
    assert_trait(&traits, "body", ProcArgTrait::Body);
}

#[test]
fn for_records_init_cond_next_body() {
    let traits = infer(&["i", "c", "n", "b"], "for $i $c $n $b");
    assert_trait(&traits, "i", ProcArgTrait::Body);
    assert_trait(&traits, "c", ProcArgTrait::Expr);
    assert_trait(&traits, "n", ProcArgTrait::Body);
    assert_trait(&traits, "b", ProcArgTrait::Body);
}

#[test]
fn upvar_records_var_read_and_aliases_writes() {
    let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
    assert_trait(&traits, "var", ProcArgTrait::VarRead);
    // Write through the alias upgrades to VarWrite.
    assert_trait(&traits, "var", ProcArgTrait::VarWrite);
}

/// The frame level the trait map cannot record.  Pinned against tclsh
/// 9.0.4 and 8.6.14 (byte-identical): with `proc q {n} {upvar L $n a; set
/// a 1}` called as `q y` from a proc, the caller's `y` exists afterwards
/// only for `L` = `1` (or an omitted level).  `0` aliases the callee's
/// own frame, `#0` the global one, `2` the caller's caller.
#[test]
fn caller_frame_upvar_params_accepts_only_the_caller_frame_level() {
    let params = ["n"];
    for (body, expected) in [
        ("upvar 1 $n a; set a 1", true),
        ("upvar $n a; set a 1", true),
        ("upvar +1 $n a", true),
        ("upvar 0x1 $n a", true),
        ("upvar 0 $n a; set a 1", false),
        ("upvar #0 $n a; set a 1", false),
        ("upvar 2 $n a; set a 1", false),
        ("upvar -1 $n a", false),
        ("upvar $lvl $n a", false),
        ("upvar bogus $n a", false),
        // `namespace upvar` aliases a namespace variable, never the
        // calling frame.
        ("namespace upvar ::cfg $n a", false),
        // The `myVar` slot names a callee-local alias, not a caller
        // variable.
        ("upvar 1 caller $n", false),
    ] {
        let got = with_receipt(&params, body, |env| {
            caller_frame_upvar_params(&params, body, env)
        });
        assert_eq!(
            got.contains("n"),
            expected,
            "{body:?} -> {got:?} (expected caller-frame param: {expected})"
        );
    }
}

/// The literal-target fact — `upvar 1 name name` binds the
/// caller's `name`, written through the alias when the body assigns it.
/// tclsh 9.0.4: `proc np {} {upvar name name; set name W1}` then
/// `np; puts $name` prints `W1` in the caller.
#[test]
fn caller_frame_literal_targets_records_written_and_read_only() {
    let targets =
        |body: &str| with_receipt(&[], body, |env| caller_frame_literal_targets(body, env));
    let got = targets("upvar name name\nset name W1");
    assert_eq!(got.get("name"), Some(&true), "written through the alias");
    // A body that only reads through the alias creates nothing.
    let got = targets("upvar name name\nreturn [string length $name]");
    assert_eq!(got.get("name"), Some(&false), "read-only alias");
    // Distinct local alias spelling still records the CALLER name.
    let got = targets("upvar 1 other mine\nset mine 1");
    assert_eq!(got.get("other"), Some(&true));
    assert!(!got.contains_key("mine"));
}

/// The exclusions: non-caller levels, `::`-qualified cells, array
/// elements, substituted sources, dynamic locals, `namespace upvar`.
#[test]
fn caller_frame_literal_targets_excludes_other_frames_and_cells() {
    let registry = CommandRegistry::build_default();
    for body in [
        "upvar 0 name name; set name 1",
        "upvar #0 name name; set name 1",
        "upvar 2 name name; set name 1",
        "upvar $lvl name name; set name 1",
        "upvar 1 ::tk::FocusGrab($i) data; set data 1",
        "upvar 1 ::ns::cell local; set local 1",
        "upvar 1 arr(k) local; set local 1",
        "upvar 1 $src local; set local 1",
        "upvar 1 name $dst",
        "namespace upvar ::cfg name local; set local 1",
    ] {
        let got = caller_frame_literal_targets(body, env_for(&registry, LexerConfig::default()));
        assert!(got.is_empty(), "{body:?} must record nothing, got {got:?}");
    }
}

/// The fact is strictly narrower than the trait: `upvar 0 $n a; set a 1`
/// still records `VarWrite` (the parameter's value *is* used as a
/// variable name through an `upvar`) while contributing no caller-frame
/// parameter — which is exactly why a call-site consumer must intersect
/// the two.
#[test]
fn caller_frame_params_is_narrower_than_the_var_traits() {
    let registry = CommandRegistry::build_default();
    let body = "upvar 0 $n a; set a 1";
    let traits = infer(&["n"], body);
    assert_trait(&traits, "n", ProcArgTrait::VarWrite);
    assert!(
        caller_frame_upvar_params(&["n"], body, env_for(&registry, LexerConfig::default()))
            .is_empty(),
        "the level fact must reject what the trait alone accepts"
    );
}

/// `upvar $lvl a b` has three words, so parity puts `$lvl` in the level
/// slot and `(a, b)` in the pair slot — the level is unplaceable, so no
/// caller-frame parameter is claimed, while the *pair* handling (and hence
/// the trait map) still applies.
#[test]
fn dynamic_level_word_keeps_parity_pairing_but_claims_no_frame() {
    let registry = CommandRegistry::build_default();
    let traits = infer(&["lvl", "v"], "upvar $lvl $v local\nset local 1");
    assert_trait(&traits, "v", ProcArgTrait::VarWrite);
    assert!(
        caller_frame_upvar_params(
            &["lvl", "v"],
            "upvar $lvl $v local\nset local 1",
            env_for(&registry, LexerConfig::default())
        )
        .is_empty(),
        "a computed level names no statically-known frame"
    );
}

#[test]
fn upvar_my_var_slot_param_is_callee_local_not_var_write() {
    // `upvar 1 caller $local` — the param in the `myVar` slot names the
    // *callee-local* alias, not a caller variable, so it is a
    // `DynamicNameLocal` use, never a caller-frame `VarWrite`.
    let traits = infer(&["local"], "upvar 1 caller $local");
    assert_trait(&traits, "local", ProcArgTrait::DynamicNameLocal);
    assert!(
        !traits
            .get("local")
            .unwrap()
            .contains(&ProcArgTrait::VarWrite),
        "myVar-slot param must not be a caller VarWrite; got {:?}",
        traits.get("local"),
    );
}

#[test]
fn lassign_records_dynamic_name_local() {
    // `lassign {1 2} $a $b` names CALLEE-LOCAL output vars via the
    // params' values — DynamicNameLocal (+ VarRead), NOT VarWrite
    // (which would imply a caller-frame alias).
    let traits = infer(&["a", "b"], "lassign {1 2} $a $b");
    for p in ["a", "b"] {
        assert_trait(&traits, p, ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, p, ProcArgTrait::VarRead);
        assert!(
            !traits.get(p).unwrap().contains(&ProcArgTrait::VarWrite),
            "{p}: callee-local lassign target must not be VarWrite, got {:?}",
            traits.get(p),
        );
    }
}

#[test]
fn command_substitution_is_scanned() {
    // A param used inside a `[...]` substitution is a first-order use — the
    // shallow scan must descend into it.  `return [set $v 1]` names a
    // callee-local variable via `$v` → DynamicNameLocal (+ VarRead).
    let traits = infer(&["v"], "return [set $v 1]");
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "v", ProcArgTrait::VarRead);
    // The read form (single-arg `set`) is scanned identically.
    let traits = infer(&["v"], "return [set $v]");
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
}

#[test]
fn command_substitution_plain_value_is_not_a_var_name() {
    // `[string length $x]` reads `$x` as a value, not a variable name, so
    // no role is recorded — the descent must not over-mark ordinary reads.
    let traits = infer(&["x"], "return [string length $x]");
    assert!(
        traits.get("x").is_none_or(HashSet::is_empty),
        "plain value read must record no trait, got {:?}",
        traits.get("x"),
    );
}

#[test]
fn command_substitution_geometry_keeps_quoted_compound_and_index_bodies() {
    for body in [
        "return \"[set $v]\"",
        "return before[set $v]after",
        "return $array([set $v])",
        "return [list [set $v]]",
    ] {
        let traits = infer(&["v"], body);
        assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
        assert_trait(&traits, "v", ProcArgTrait::VarRead);
    }
    for body in ["return {[set $v]}", "return \\[set\\ $v\\]"] {
        let traits = infer(&["v"], body);
        assert!(traits.get("v").is_none_or(HashSet::is_empty), "{body}");
    }
}

#[test]
fn command_substitution_geometry_preserves_index_and_root_alias_order() {
    let traits = infer(&["v"], "return \"$array([set alias $v])[set $alias]\"");
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "v", ProcArgTrait::VarRead);
    let reversed = infer(&["v"], "return \"[set $alias]$array([set alias $v])\"");
    assert!(reversed.get("v").is_none_or(HashSet::is_empty));
}

#[test]
fn braced_var_name_has_no_substitution() {
    // `set {arr($k)} 1` — the braces make `arr($k)` a *literal* variable
    // name, so `$k` is not a substitution and must not mark `k` (the
    // segmenter drops the braces, so the guard is by token kind).
    assert!(!infer(&["k"], "set {arr($k)} 1").contains_key("k"));
    assert!(!infer(&["p"], "set {$p} 1").contains_key("p"));
    // The unbraced form *does* substitute, so it still marks the component.
    assert_trait(
        &infer(&["k"], "set arr($k) 1"),
        "k",
        ProcArgTrait::DynamicNameLocal,
    );
}

#[test]
fn compound_dynamic_array_name_marks_components() {
    // `set ${v}($k) 1` — both the array-name param `v` and the key param
    // `k` flow into a dynamic variable name, so both are DynamicNameLocal
    // (+ VarRead).  This is the shape behind `return [set ${v}($k)]`.
    for body in ["set ${v}($k) 1", "set $v($k) 1", "return [set ${v}($k)]"] {
        let traits = infer(&["v", "k"], body);
        for p in ["v", "k"] {
            assert_trait(&traits, p, ProcArgTrait::DynamicNameLocal);
            assert_trait(&traits, p, ProcArgTrait::VarRead);
        }
    }
    // A literal array name with a `$k` key marks only the key.
    let traits = infer(&["k"], "set arr($k) 1");
    assert_trait(&traits, "k", ProcArgTrait::DynamicNameLocal);
}

#[test]
fn var_substitutions_finds_compound_components() {
    let parameters = ["v", "k", "escaped", "1backref"];
    let components = |source| {
        lexical_reference_values(source, &parameters, LexerConfig::default())
            .components
            .remove(1)
    };
    assert_eq!(components("$v"), vec!["v"]);
    assert_eq!(components("${v}($k)"), vec!["v", "k"]);
    assert_eq!(components("$v($k)"), vec!["v", "k"]);
    assert_eq!(components("arr($k)"), vec!["k"]);
    assert_eq!(components("literal"), Vec::<&str>::new());
    assert_eq!(components("\\$escaped"), Vec::<&str>::new());
    assert_eq!(components("$1backref"), vec!["1backref"]);
}

#[test]
fn original_parameter_reference_parts_keep_configured_names_and_literal_indices() {
    let parameters = [
        "1abc", "", "odd name", "caf", "café", "a{b", "a{b}c", "v", "k", "arr",
    ];
    for (dialect, unicode, nested) in [
        ("tcl8.6", false, false),
        ("tcl9.1", false, true),
        ("jim", true, false),
    ] {
        let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
            .expect("selected original reference grammar environment")
            .unit_profile();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let values = |word| lexical_reference_values(word, &parameters, config);
        for (word, expected) in [
            ("$1abc", Some("1abc")),
            ("${}", Some("")),
            ("${odd name}", Some("odd name")),
            ("\"$1abc\"", Some("1abc")),
            ("{$1abc}", None),
            ("\\$1abc", None),
        ] {
            assert_eq!(values(word).argument(0), expected, "{dialect}/{word}");
        }
        assert_eq!(values("$café").argument(0), unicode.then_some("café"));
        assert_eq!(
            values("$café").argument_components(0),
            if unicode {
                &["café"][..]
            } else {
                &["caf"][..]
            }
        );
        assert_eq!(values("${a{b}c}").argument(0), nested.then_some("a{b}c"));
        assert_eq!(
            values("${a{b}c}").argument_components(0),
            if nested { &["a{b}c"][..] } else { &["a{b"][..] }
        );
        assert_eq!(values("$arr($k)").argument_components(0), &["arr", "k"]);
        assert_eq!(values("${arr($k)}").argument_components(0), [] as [&str; 0]);
        assert_eq!(
            values("[set $café]").argument_components(0),
            if unicode {
                &["café"][..]
            } else {
                &["caf"][..]
            },
        );
        assert_eq!(
            values("[set ${a{b}c}]").argument_components(0),
            if nested { &["a{b}c"][..] } else { &["a{b"][..] },
        );
        assert!(values("${arr($k)}").argument(0).is_none());
        assert_eq!(values("{$arr($k)}").argument_components(0), [] as [&str; 0]);
    }
}

#[test]
fn original_parameter_reference_components_keep_lexical_scalar_names() {
    let parameters = ["p", "p(key)", "p($k)", "k", "after"];
    for dialect in ["tcl8.6", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
            .unwrap()
            .unit_profile();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        for (source, expected) in [
            ("${p(key)}", vec!["p(key)"]),
            ("${p($k)}", vec!["p($k)"]),
            ("$p(key)", vec!["p"]),
            ("$p($k)", vec!["p", "k"]),
            ("prefix-${p(key)}", vec!["p(key)"]),
            ("[set ${p(key)}]", vec!["p(key)"]),
            ("[set ${p($k)}]", vec!["p($k)"]),
            ("[set $p($k)]", vec!["p", "k"]),
            ("$p([set ${p(key)}])", vec!["p", "p(key)"]),
            ("[set ${p(key)}]$after", vec!["p(key)", "after"]),
            ("[set [set [set ${p(key)}]]]", vec!["p(key)"]),
            ("[set {${p(key)}}]", vec![]),
            ("{${p(key)}}", vec![]),
            ("\\${p(key)}", vec![]),
        ] {
            let values = lexical_reference_values(source, &parameters, config);
            assert_eq!(
                values.argument_components(0),
                expected,
                "{dialect}/{source}"
            );
        }
        assert!(
            lexical_reference_values("${p(key)}", &parameters, config)
                .argument(0)
                .is_none()
        );
    }
}

#[test]
fn original_parameter_scalar_components_do_not_borrow_an_array_root_trait() {
    for deep in [false, true] {
        for body in [
            "set ${p(key)} 1",
            "set target [set ${p(key)}]",
            "set target [set $p([set ${p(key)}])]",
        ] {
            let traits = infer_owned(&["p", "p(key)"], body, deep);
            assert_trait(&traits, "p(key)", ProcArgTrait::DynamicNameLocal);
            if !body.contains("$p(") {
                assert!(!traits.contains_key("p"), "{deep}/{body}: {traits:?}");
            }
        }
        let traits = infer_owned(&["p", "k"], "set $p($k) 1", deep);
        for parameter in ["p", "k"] {
            assert_trait(&traits, parameter, ProcArgTrait::DynamicNameLocal);
        }
    }
}

#[test]
fn original_parameter_reference_parts_refuse_changed_segment_presentations() {
    let config = LexerConfig::default();
    let registry = CommandRegistry::build_default();
    let original = "command $old";
    let commands = segment_commands_with_offset_and_config(original, 0, config);
    let parameters = HashSet::from(["old", "new"]);
    let aliases = Aliases::default();
    let valid = ArgumentValues::lexical(
        original,
        &commands[0],
        &parameters,
        &aliases,
        config,
        &registry,
    );
    assert_eq!(valid.argument(0), Some("old"));
    let changed = ArgumentValues::lexical(
        "command $new",
        &commands[0],
        &parameters,
        &aliases,
        config,
        &registry,
    );
    assert!(changed.argument(0).is_none());
    assert_eq!(changed.argument_components(0), [] as [&str; 0]);
    let literal = lexical_reference_values("{$old}", &["old"], config);
    assert!(literal.argument(0).is_none());
    assert_eq!(literal.argument_components(0), [] as [&str; 0]);
}

#[test]
fn value_copy_carries_param_into_name_and_command_positions() {
    // `set n $p` copies the param's value into local `n`; a later `$n` in a
    // name / command position then resolves back to the param.
    assert_trait(
        &infer(&["v"], "set n $v\nset $n 1"),
        "v",
        ProcArgTrait::DynamicNameLocal,
    );
    assert_trait(
        &infer(&["cmd"], "set c $cmd\n$c arg"),
        "cmd",
        ProcArgTrait::Command,
    );
    // In a command substitution too.
    assert_trait(
        &infer(&["v"], "set n $v\nreturn [set $n]"),
        "v",
        ProcArgTrait::DynamicNameLocal,
    );
    // Transitive copies chain.
    assert_trait(
        &infer(&["cmd"], "set c $cmd\nset d $c\n$d arg"),
        "cmd",
        ProcArgTrait::Command,
    );
}

#[test]
fn value_copy_is_invalidated_and_not_over_eager() {
    // Reassigning the local to a non-param drops the copy, so a later `$n`
    // no longer resolves to the original param.
    assert!(!infer(&["v"], "set n $v\nset n other\nset $n 1").contains_key("v"));
    // Merely passing the value through (never used as a name) is not a role.
    assert!(!infer(&["v"], "set n $v\nreturn $n").contains_key("v"));
}

#[test]
fn accepted_copies_feed_eval_without_surviving_unknown_or_mutated_values() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    assert_trait(
        &infer(&["body"], "set local $body; eval $local"),
        "body",
        ProcArgTrait::Eval,
    );
    for body in [
        "set local $body; set local ordinary; eval $local",
        "set local $body; set ::auto_index(unknown_callback) {source lib.tcl}; unknown_callback; eval $local",
        "set local $body; eval {$local}",
    ] {
        assert!(
            !infer(&["body"], body)
                .get("body")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval)),
            "{body}"
        );
    }
}

#[test]
fn referenced_upvar_contents_are_not_the_parameter_value_that_named_them() {
    let traits = infer(
        &["name"],
        "upvar 1 $name contents; set contents SCRIPT; set copy $contents; eval $copy",
    );
    assert_trait(&traits, "name", ProcArgTrait::VarWrite);
    assert!(
        !traits["name"].contains(&ProcArgTrait::Eval),
        "the caller cell's contents are distinct from its parameter-supplied name"
    );
}

#[test]
fn param_used_as_command_head_is_command() {
    // `$cmd arg1 arg2` — the param's value is the command word, so it names
    // a command.  Works at the top level and inside a `[...]` substitution.
    assert_trait(&infer(&["cmd"], "$cmd a b"), "cmd", ProcArgTrait::Command);
    assert_trait(
        &infer(&["cmd"], "return [$cmd x]"),
        "cmd",
        ProcArgTrait::Command,
    );
    // A braced `{$cmd}` head is a *literal* command name — no substitution.
    assert!(!infer(&["cmd"], "{$cmd} arg").contains_key("cmd"));
    // A param merely read as a value is not a command.
    assert!(!infer(&["x"], "puts $x").contains_key("x"));
}

#[test]
fn stub_command_prefix_role_is_command() {
    // A stub-declared `command_prefix` argument names a command, so a
    // `$param` there is inferred `Command`.
    let declared = make_declarations(vec![stub_sig(
        "on_event",
        &[
            ("event", ArgRole::Value),
            ("handler", ArgRole::CommandPrefix),
        ],
    )]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["h"],
        "on_event click $h",
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    assert_trait(&traits, "h", ProcArgTrait::Command);
}

#[test]
fn namespace_upvar_dynamic_name_reads_other_var() {
    // `namespace upvar ns $token arr` — the token/state-array idiom, with a
    // dynamic `$token` naming the namespace variable aliased into `arr`.
    // The pairs start after the `namespace` word (index 2 of the
    // sub-command args), so `$token` is the *other* var → VarRead, and a
    // later write through the `arr` alias upgrades it to VarWrite.
    let traits = infer(&["token"], "namespace upvar ns $token arr");
    assert_trait(&traits, "token", ProcArgTrait::VarRead);
    let traits = infer(&["token"], "namespace upvar ns $token arr\nset arr 1");
    assert_trait(&traits, "token", ProcArgTrait::VarWrite);
}

#[test]
fn real_world_by_reference_patterns_are_inferred() {
    // The dominant variable-name-by-reference shapes mined from the tcllib /
    // Tcl-library corpus (`http`, `smtp`, `ftp`, … token idiom).  Each names
    // a variable via a parameter, so each must record a read/write trait.
    let has_var = |body: &str, p: &str| {
        let t = infer(&[p], body);
        t.get(p).is_some_and(|s| {
            s.contains(&ProcArgTrait::VarRead) || s.contains(&ProcArgTrait::VarWrite)
        })
    };
    for (body, p) in [
        ("upvar 1 $varname data", "varname"),
        ("upvar 0 $token state", "token"),
        ("upvar #0 $token state", "token"),
        ("upvar $v local", "v"),
        ("variable $name", "name"),
        ("array set $a {x 1}", "a"),
        ("array get $a", "a"),
        ("array names $a", "a"),
        ("unset $v", "v"),
        ("dict set $d k 1", "d"),
        ("incr $counter", "counter"),
        ("append $resultVar x", "resultVar"),
        ("lappend $pages_var p", "pages_var"),
        ("set ${tok}(state) connected", "tok"),
        ("set $gas(idx) 1", "gas"),
        ("namespace upvar ns $v local", "v"),
        ("foreach k [array names $a] { puts $k }", "a"),
        ("return [set ${token}($field)]", "token"),
    ] {
        assert!(
            has_var(body, p),
            "expected a by-reference var trait for `{p}` in `{body}`; got {:?}",
            infer(&[p], body).get(p),
        );
    }
}

#[test]
fn after_records_eval_skipping_cancel_info() {
    let traits = infer(&["body"], "after 100 $body");
    assert_trait(&traits, "body", ProcArgTrait::Eval);
    let traits = infer(&["x"], "after cancel $x");
    // ``after cancel`` doesn't take a script, so $x is not eval.
    assert!(
        traits
            .get("x")
            .is_none_or(|s| !s.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn regsub_records_dynamic_name_local() {
    // `regsub`'s output var is CALLEE-LOCAL — DynamicNameLocal
    // (+ VarRead), not VarWrite.
    let traits = infer(&["line", "out"], "regsub -all foo $line bar $out");
    assert_trait(&traits, "out", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "out", ProcArgTrait::VarRead);
    assert!(
        !traits.get("out").unwrap().contains(&ProcArgTrait::VarWrite),
        "callee-local regsub target must not be VarWrite, got {:?}",
        traits.get("out"),
    );
}

#[test]
fn regexp_capture_vars_after_switches_record_dynamic_name_local() {
    // `regexp -nocase -- $pat $s $m` — the registry `regexp` spec's
    // arg-role resolver performs the switch skip (`--` terminator
    // included), so the capture var `$m` names a callee-local output
    // variable.  The pattern / subject args are plain value reads and
    // must record no trait.
    let traits = infer(&["pat", "s", "m"], "regexp -nocase -- $pat $s $m");
    assert_trait(&traits, "m", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "m", ProcArgTrait::VarRead);
    assert!(!traits.contains_key("pat"), "{traits:?}");
    assert!(!traits.contains_key("s"), "{traits:?}");
}

#[test]
fn regexp_start_switch_value_is_skipped() {
    // `-start` consumes its value word (spec-declared), so the capture
    // var still resolves after the pattern + string positionals.
    let traits = infer(&["s", "m"], "regexp -start 2 {x+} $s $m");
    assert_trait(&traits, "m", ProcArgTrait::DynamicNameLocal);
}

#[test]
fn regsub_command_switch_is_skipped_to_output_var() {
    // Tcl 9 `regsub -command exp string cmdPrefix varName` — `-command`
    // is a spec-declared flag (one a hardcoded switch list misses), so
    // the output var must still resolve at exp + 3.
    let traits = infer(&["s", "out"], "regsub -command {x+} $s myCb $out");
    assert_trait(&traits, "out", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "out", ProcArgTrait::VarRead);
}

#[test]
fn set_dynamic_name_records_dynamic_name_local_not_var_write() {
    // `set $p 1` — the param's VALUE names a callee-local variable
    // (registry `VarWrite` role on the `$p` substitution).  The
    // refined trait is DynamicNameLocal (+ VarRead), NOT VarWrite:
    // a caller passing a literal name (`f x`) does not have its `x`
    // consumed by this callee.
    let traits = infer(&["p"], "set $p 1");
    assert_trait(&traits, "p", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "p", ProcArgTrait::VarRead);
    assert!(
        !traits.get("p").unwrap().contains(&ProcArgTrait::VarWrite),
        "callee-local `set $p` must not be VarWrite, got {:?}",
        traits.get("p"),
    );
}

#[test]
fn genuine_upvar_write_stays_var_write_not_dynamic_name_local() {
    // A real caller-frame write-back through an `upvar` alias is a
    // genuine `VarWrite` — the DynamicNameLocal refinement must NOT
    // leak onto it (otherwise caller-side suppression of a genuine
    // call-by-name write would regress).
    let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
    assert_trait(&traits, "var", ProcArgTrait::VarWrite);
    assert!(
        !traits
            .get("var")
            .unwrap()
            .contains(&ProcArgTrait::DynamicNameLocal),
        "upvar write-back must not be DynamicNameLocal, got {:?}",
        traits.get("var"),
    );
}

#[test]
fn empty_body_returns_empty_map() {
    let traits = infer(&["a"], "");
    assert!(traits.is_empty());
}

/// Deep-pass helper that mirrors [`infer`] for ergonomics.
fn infer_deep(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_owned(params, body, true)
}

#[test]
fn deep_pass_surfaces_eval_trait_inside_braced_body() {
    // `$body` is used inside a nested `foreach` body — the
    // shallow pass walks only top-level commands, so it
    // misses the trait.  The deep pass descends into the
    // braced `foreach` body and surfaces `Eval`.
    let body = "foreach item $items {\n  uplevel 1 $body\n}";
    let shallow = infer(&["items", "body"], body);
    let deep = infer_deep(&["items", "body"], body);
    // Shallow catches `items` (LoopList) but misses `body`.
    assert_trait(&shallow, "items", ProcArgTrait::LoopList);
    assert!(
        !shallow
            .get("body")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "shallow pass should not surface nested Eval, got {shallow:?}",
    );
    // Deep catches both.
    assert_trait(&deep, "items", ProcArgTrait::LoopList);
    assert_trait(&deep, "body", ProcArgTrait::Eval);
}

#[test]
fn deep_pass_descends_through_multiple_levels() {
    // `$inner` is buried two levels deep: `if` → `while` →
    // `eval $inner`.  Shallow misses it; deep finds it.
    let body = "if {1} {\n  while {1} {\n    eval $inner\n  }\n}";
    let deep = infer_deep(&["inner"], body);
    assert_trait(&deep, "inner", ProcArgTrait::Eval);
}

#[test]
fn deep_pass_respects_max_depth() {
    // Build a body nested past `MAX_DEPTH` (8 levels of
    // `if {1} { ... }`) with `eval $deep_var` at the
    // innermost level.  The recursion should stop before
    // reaching the innermost level and the trait should not
    // be surfaced.  Using `MAX_DEPTH + 2` (10) levels of
    // nesting puts the eval below the recursion bound.
    let depth_to_nest = usize::from(MAX_DEPTH) + 2;
    let mut body = String::from("eval $deep_var");
    for _ in 0..depth_to_nest {
        body = format!("if {{1}} {{ {body} }}");
    }
    let deep = infer_deep(&["deep_var"], &body);
    assert!(
        !deep
            .get("deep_var")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "MAX_DEPTH bound should keep deeply-nested eval from being surfaced, got {deep:?}",
    );
}

/// Depth coverage for the `apply` seam: `scan_deep`'s `apply`
/// (`ArgRole::LambdaLiteral`) handling must thread its own `depth + 1`
/// rather than re-enter `infer_param_traits_deep_with_config` — the
/// *public*, depth-0 entry point. Re-entering resets the *logical*
/// [`MAX_DEPTH`] counter to 0 on every `apply` boundary while the
/// *native* Rust call stack (`scan_deep` ↔
/// `infer_param_traits_deep_at_depth`) keeps growing one frame group per
/// level regardless — unboundedly, for however deep the input
/// alternates, so `MAX_DEPTH` (8) never bites.
///
/// 2000 alternating pairs (4000 real nesting levels) is far beyond
/// anything such a bypass would survive on a small-stack
/// thread; this must terminate cleanly, not hang or overflow the
/// stack — the same rationale as the big-stack helpers in
/// `analyser::commands::tests` / `lowering::tests` (`cargo test`'s
/// per-test thread has the same undersized default stack production hits).
#[test]
fn deep_pass_bounds_alternating_if_apply_nesting() {
    const PAIRS: usize = 2000;
    let mut body = String::from("puts leaf");
    for _ in 0..PAIRS {
        body = format!("if {{1}} {{ apply {{x {{ {body} }}}} 1 }}");
    }
    // With `depth` correctly threaded, an `if`/`apply` pair costs 2
    // logical levels, so real recursion stops at MAX_DEPTH (8) — a
    // handful of native frames regardless of PAIRS. This deliberately
    // runs on the *default* test-thread stack (no big-stack wrapper,
    // unlike the sibling tests in this file and in
    // `commands::tests`/`lowering::tests`): if the reset bug ever comes
    // back, real recursion runs all the way down through every one of
    // PAIRS × 2 levels again — verified locally, reverting just the
    // `depth + 1` fix below to `0` reliably overflows the stack and
    // aborts the test process outright on this same (small,
    // default-sized) thread, before the wall-clock assertion even gets
    // a chance to run. The elapsed-time check below is a second,
    // softer signal for the rare platform/build where the crash
    // threshold happens to sit a little higher.
    let registry = CommandRegistry::build_default();
    let start = std::time::Instant::now();
    let _ = infer_param_traits_deep(&["p"], &body, env_for(&registry, LexerConfig::default()));
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "took {elapsed:?} — alternating if/apply nesting is no longer bounded by \
             MAX_DEPTH (reset-to-0 bypass regression?)"
    );
}

#[test]
fn deep_pass_skips_dynamic_body_args() {
    // `if {1} $body` — the body is a `$var` reference,
    // not a braced literal.  The deep pass shouldn't try
    // to descend into it (we have no body text to scan);
    // the shallow pass already surfaces the `Body` trait
    // via the registry's role scan.  This pins that
    // contract: dynamic body args don't double-count.
    let body = "if {1} $body";
    let deep = infer_deep(&["body"], body);
    assert_trait(&deep, "body", ProcArgTrait::Body);
}

#[test]
fn merge_traits_unions_shallow_and_deep() {
    let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    shallow
        .entry("p1".into())
        .or_default()
        .insert(ProcArgTrait::VarRead);
    let mut deep: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    deep.entry("p1".into())
        .or_default()
        .insert(ProcArgTrait::Eval);
    deep.entry("p2".into())
        .or_default()
        .insert(ProcArgTrait::Body);

    let merged = merge_traits(shallow, deep);
    // p1 gains Eval from deep without losing VarRead from shallow.
    assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::VarRead));
    assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::Eval));
    // p2 (deep-only) lands in the merged map.
    assert!(merged.get("p2").unwrap().contains(&ProcArgTrait::Body));
}

#[test]
fn merge_traits_with_empty_deep_returns_shallow_unchanged() {
    let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    shallow
        .entry("p1".into())
        .or_default()
        .insert(ProcArgTrait::VarWrite);
    let merged = merge_traits(shallow.clone(), HashMap::new());
    assert_eq!(merged.get("p1"), shallow.get("p1"));
}

#[test]
fn deep_pass_matches_shallow_for_top_level_only_bodies() {
    // When there are no nested bodies, the deep pass
    // should return exactly what the shallow pass does.
    let body = "set $x 1\nupvar 1 $var local";
    let shallow = infer(&["x", "var"], body);
    let deep = infer_deep(&["x", "var"], body);
    assert_eq!(shallow, deep);
}

#[test]
fn deep_pass_empty_params_returns_empty_map() {
    let deep = infer_deep(&[], "foreach item $items { uplevel 1 $body }");
    assert!(deep.is_empty());
}

// document-declaration integration
//
// These tests pin the contract that a non-empty
// [`tcl_registry::model::DeclaredSurface`] threaded through
// `infer_param_traits` / `infer_param_traits_deep` lets user-declared
// `# tcl-lsp: stub` commands participate in role-driven trait
// inference alongside the catalogue.

use tcl_dialect::model::Provenance;
use tcl_registry::model::{DeclaredArgument, DeclaredCommand, DeclaredSurface};

fn make_declarations(commands: Vec<DeclaredCommand>) -> DeclaredSurface {
    let mut surface = DeclaredSurface::new();
    for command in commands {
        surface.declare(command);
    }
    surface
}

fn stub_sig(name: &str, args: &[(&str, ArgRole)]) -> DeclaredCommand {
    DeclaredCommand::new(
        name.to_string(),
        args.iter()
            .map(|(n, r)| DeclaredArgument {
                name: (*n).to_string(),
                role: *r,
                optional: false,
            })
            .collect(),
        Provenance::Document,
    )
}

#[test]
fn declarations_shallow_surface_the_declared_body_role() {
    // `my_eval` isn't in the catalogue, but the document
    // declares its arg-0 as `body`.  An invocation
    // `my_eval $script` should therefore surface `Body` on
    // the `script` param.
    let declared = make_declarations(vec![stub_sig("my_eval", &[("script", ArgRole::Body)])]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["script"],
        "my_eval $script",
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    assert_trait(&traits, "script", ProcArgTrait::Body);
}

#[test]
fn declarations_shallow_surface_the_declared_var_write_role() {
    // A declared `VarWrite` on a bare `$param` substitution
    // surfaces the callee-local DynamicNameLocal refinement.
    let declared = make_declarations(vec![stub_sig(
        "with_var",
        &[("varName", ArgRole::VarWrite), ("value", ArgRole::Value)],
    )]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["v"],
        "with_var $v 42",
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    // A declared `VarWrite` role on a bare `$v` substitution is the same
    // callee-local dynamic-name shape as `set $v …` — refined to
    // DynamicNameLocal (+ VarRead), not a caller-frame VarWrite.
    assert_trait(&traits, "v", ProcArgTrait::DynamicNameLocal);
    assert_trait(&traits, "v", ProcArgTrait::VarRead);
}

#[test]
fn declarations_deep_recurse_through_declared_body_args() {
    // The document declares `my_loop`'s arg-1 as a body.
    // Without the declaration the deep pass can't see that
    // `my_loop { uplevel 1 $body }` carries an Eval inside.
    // With it the pass descends into the brace and surfaces
    // the Eval.
    let declared = make_declarations(vec![stub_sig(
        "my_loop",
        &[("count", ArgRole::Value), ("body", ArgRole::Body)],
    )]);
    let registry = CommandRegistry::build_default();
    let body = "my_loop 5 { uplevel 1 $script }";
    // Sanity: without the declaration, the deep pass misses
    // the nested Eval because `my_loop` isn't recognised.
    let undeclared = infer_param_traits_deep(
        &["script"],
        body,
        env_for(&registry, LexerConfig::default()),
    );
    assert!(
        !undeclared
            .get("script")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "undeclared, my_loop's body shouldn't be recognised, got {undeclared:?}",
    );
    // With the declaration, the recursion fires.
    let with_declaration = infer_param_traits_deep(
        &["script"],
        body,
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, Some(&declared)),
            config: LexerConfig::default(),
            identities: crate::realm::CommandBindingRealm::none(),
            executed_source: None,
            original_logical_body: None,
        },
    );
    assert_trait(&with_declaration, "script", ProcArgTrait::Eval);
}

#[test]
fn declarations_do_not_disturb_catalogue_resolution() {
    // A declaration covering `my_thing` mustn't shadow any
    // catalogue command.  A shipped `foreach` invocation
    // still records its `LoopList` / `Body` traits through the
    // catalogue path even when an unrelated document
    // declaration is active.
    let declared = make_declarations(vec![stub_sig("my_thing", &[("a", ArgRole::Body)])]);
    let registry = CommandRegistry::build_default();
    let traits = infer_param_traits(
        &["items", "body"],
        "foreach x $items $body",
        env_with_declarations(&registry, &declared),
    );
    assert_trait(&traits, "items", ProcArgTrait::LoopList);
    assert_trait(&traits, "body", ProcArgTrait::Body);
}

#[test]
fn no_declarations_matches_an_empty_declaration_set() {
    // An empty declaration set should produce the same result as
    // `None` — a document that declares nothing is exactly the
    // catalogue.
    let registry = CommandRegistry::build_default();
    let body = "foreach x {1 2 3} $body";
    let none = infer_param_traits(&["body"], body, env_for(&registry, LexerConfig::default()));
    let declared = DeclaredSurface::new();
    let empty = infer_param_traits(&["body"], body, env_with_declarations(&registry, &declared));
    assert_eq!(none, empty);
}

/// Trait inference must resolve a command head's *effective identity*, not
/// its written spelling.
///
/// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} run
/// {} eval` makes `run $body` evaluate `$body`; `rename eval run` moves it
/// and leaves `eval` gone; a top-level `proc eval …` takes the name over,
/// so `eval $body` no longer evaluates anything.
///
/// The environment carries the *document's* facts while the scan reads a
/// *proc body*, which is exactly the shape the analyser threads.
fn realm_for_body(
    prelude: &str,
    body: &str,
) -> (crate::realm::CommandBindingRealm, tcl_lexer::Span) {
    let source = format!("{prelude}proc __trait_probe {{body v}} {{{body}}}");
    let registry = CommandRegistry::build_default();
    let identities = crate::realm::document_realm_bindings(
        &source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    let segments = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default());
    let body_span = segments.last().unwrap().argv.last().unwrap().span;
    (identities, body_span)
}

fn traits_under(prelude: &str, body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
    let registry = CommandRegistry::build_default();
    let (identities, body_span) = realm_for_body(prelude, body);
    infer_param_traits(
        &["body", "v"],
        body,
        TraitScanEnv {
            surface: DocumentCommandSurface::new(&registry, None),
            config: LexerConfig::default(),
            identities: &identities,
            executed_source: identities.executed_script_for_word(body_span),
            original_logical_body: None,
        },
    )
}

fn is_eval(traits: &HashMap<String, HashSet<ProcArgTrait>>) -> bool {
    traits
        .get("body")
        .is_some_and(|s| s.contains(&ProcArgTrait::Eval))
}

#[test]
fn param_traits_follow_an_aliased_head() {
    assert!(is_eval(&traits_under(
        "interp alias {} run {} eval\n",
        "run $body"
    )));
    // The `::`-qualified spelling of the alias classifies alike.
    assert!(is_eval(&traits_under(
        "interp alias {} run {} eval\n",
        "::run $body"
    )));
    // Guard: an unbound `run` evaluates nothing.
    assert!(!is_eval(&traits_under("set y 1\n", "run $body")));
}

#[test]
fn param_traits_follow_a_renamed_head() {
    assert!(is_eval(&traits_under("rename eval run\n", "run $body")));
    assert!(
        !is_eval(&traits_under("rename eval run\n", "eval $body")),
        "a renamed-away `eval` must not keep the built-in's grammar"
    );
}

#[test]
fn param_traits_abstain_for_a_builtin_shadowed_by_a_user_proc() {
    assert!(
        !is_eval(&traits_under("proc eval {s} { return $s }\n", "eval $body")),
        "a user `proc eval` takes the name over; its argument is not a script"
    );
    // Guard: the unshadowed built-in still records the trait.
    assert!(is_eval(&traits_under("set y 1\n", "eval $body")));
}

#[test]
fn param_traits_abstain_for_a_dynamic_binding() {
    assert!(
        !is_eval(&traits_under("rename $old run\n", "run $body")),
        "a dynamic rename must not make `run` an evaluator"
    );
    assert!(
        !is_eval(&traits_under("rename $old run\n", "eval $body")),
        "the unknown source may be eval, so its later evaluator identity is unresolved"
    );
    assert!(
        is_eval(&traits_under("rename puts output\n", "eval $body")),
        "a proved unrelated rename preserves the evaluator binding"
    );
}

/// The caller-frame scans read the resolved head too: `upvar`'s
/// `FrameArgLayout::AliasPairs` grammar belongs to the command a head is.
#[test]
fn caller_frame_scans_follow_the_resolved_head() {
    let registry = CommandRegistry::build_default();
    let params_under = |prelude: &str, body: &str| {
        let (identities, span) = realm_for_body(prelude, body);
        caller_frame_upvar_params(
            &["body", "v"],
            body,
            TraitScanEnv {
                surface: DocumentCommandSurface::new(&registry, None),
                config: LexerConfig::default(),
                executed_source: identities.executed_script_for_word(span),
                original_logical_body: None,
                identities: &identities,
            },
        )
    };
    let literals_under = |prelude: &str, body: &str| {
        let (identities, span) = realm_for_body(prelude, body);
        caller_frame_literal_targets(
            body,
            TraitScanEnv {
                surface: DocumentCommandSurface::new(&registry, None),
                config: LexerConfig::default(),
                executed_source: identities.executed_script_for_word(span),
                original_logical_body: None,
                identities: &identities,
            },
        )
    };
    let alias = "interp alias {} peek {} upvar\n";
    assert!(
        params_under(alias, "peek 1 $v alias").contains("v"),
        "an alias of upvar must select the caller frame"
    );
    assert!(literals_under(alias, "peek 1 name name\nset name W1").contains_key("name"));
    let shadow = "proc upvar {a b c} { return 1 }\n";
    assert!(params_under(shadow, "upvar 1 $v alias").is_empty());
    assert!(literals_under(shadow, "upvar 1 name name\nset name W1").is_empty());
    assert!(params_under("", "upvar 1 $v alias").contains("v"));
    assert!(literals_under("", "upvar 1 name name\nset name W1").contains_key("name"));
}

#[test]
fn positioned_own_body_tracks_local_symbolic_copy_without_a_caller() {
    let source = "proc p {body} {set local $body; eval $local}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let identities = crate::realm::document_realm_bindings(
        source,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        registry,
    );
    let command = segment_commands_with_offset_and_config(source, 0, LexerConfig::default())
        .pop()
        .unwrap();
    let script = identities
        .executed_script_for_word(command.argv.last().unwrap().span)
        .unwrap();
    let env = TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, None),
        config: LexerConfig::default(),
        identities: &identities,
        executed_source: Some(script),
        original_logical_body: None,
    };
    let traits = infer_param_traits(&["body"], script.try_text().unwrap(), env);
    assert_trait(&traits, "body", ProcArgTrait::Eval);
    assert!(caller_frame_upvar_params(&["body"], script.try_text().unwrap(), env).is_empty());
    assert!(caller_frame_literal_targets(script.try_text().unwrap(), env).is_empty());
    let bare = infer_param_traits(
        &["body"],
        script.try_text().unwrap(),
        env_for(registry, LexerConfig::default()),
    );
    assert!(
        !bare
            .get("body")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Eval))
    );
}

#[test]
fn declaration_retains_symbolic_caller_name_without_caller_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let declaration = "proc p {v} {upvar 1 $v alias; set alias 1}";
    for (called, source) in [
        (false, declaration.to_owned()),
        (true, format!("{declaration}; p target")),
    ] {
        let identities = crate::realm::document_realm_bindings(
            &source,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            registry,
        );
        let commands = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default());
        let script = identities
            .executed_script_for_word(commands[0].argv.last().unwrap().span)
            .unwrap();
        let env = TraitScanEnv {
            surface: DocumentCommandSurface::new(registry, None),
            config: LexerConfig::default(),
            identities: &identities,
            executed_source: Some(script),
            original_logical_body: None,
        };
        assert!(caller_frame_upvar_params(&["v"], script.try_text().unwrap(), env).contains("v"));
        assert_eq!(env.has_actual_procedure_entry(), called);
        assert!(caller_frame_literal_targets(script.try_text().unwrap(), env).is_empty());
    }
}

#[test]
fn conditional_own_body_traits_reach_whole_and_per_item_proc_consumers() {
    let source = "proc p {body} {set local $body; eval $local}";
    let mut analyser = crate::analyser::Analyser::new();
    let whole = analyser.analyse(source, "tcl8.6");
    let mut analyser = crate::analyser::Analyser::new();
    let per_item = analyser.analyse_per_item(source, "tcl8.6");
    for result in [whole, per_item] {
        let procedure = result.all_procs.get("::p").expect("original declaration");
        assert_trait(&procedure.param_traits, "body", ProcArgTrait::Eval);
        assert!(procedure.caller_frame_params.is_empty());
        assert!(procedure.caller_frame_literals.is_empty());
    }
}

#[test]
fn caller_frame_parameter_templates_reach_navigation_without_entered_caller() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "proc setter {d} {upvar 1 $d local; set local VALUE}\nproc caller {} {setter target; puts $target}";
        let mut whole = crate::analyser::Analyser::new();
        let mut per_item = crate::analyser::Analyser::new();
        for analysis in [
            whole.analyse(source, dialect),
            per_item.analyse_per_item(source, dialect),
        ] {
            let setter = analysis.all_procs.get("::setter").expect("original setter");
            assert!(setter.caller_frame_params.contains("d"), "{dialect}");
            assert_trait(&setter.param_traits, "d", ProcArgTrait::VarWrite);
        }
        let replaced = "proc setter {d} {set d OTHER; upvar 1 $d local; set local VALUE}\nproc caller {} {setter target; puts $target}";
        let mut analyser = crate::analyser::Analyser::new();
        let analysis = analyser.analyse(replaced, dialect);
        let setter = analysis.all_procs.get("::setter").expect("original setter");
        assert!(!setter.caller_frame_params.contains("d"), "{dialect}");
        assert!(
            !setter
                .param_traits
                .get("d")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite)),
            "{dialect}"
        );
    }
}

#[test]
fn declared_roles_cannot_restore_a_replaced_or_opaque_slot() {
    let declared = make_declarations(vec![stub_sig("my_eval", &[("script", ArgRole::Body)])]);
    let registry = CommandRegistry::build_default();
    for prelude in [
        "proc my_eval args {return ordinary};",
        "rename $old output;",
    ] {
        let body = "my_eval $body";
        let (identities, span) = realm_for_body(prelude, body);
        let traits = infer_param_traits(
            &["body"],
            body,
            TraitScanEnv {
                surface: DocumentCommandSurface::new(&registry, Some(&declared)),
                config: LexerConfig::default(),
                identities: &identities,
                executed_source: identities.executed_script_for_word(span),
                original_logical_body: None,
            },
        );
        assert!(
            !traits
                .get("body")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::Body)),
            "{prelude}"
        );
    }
}
