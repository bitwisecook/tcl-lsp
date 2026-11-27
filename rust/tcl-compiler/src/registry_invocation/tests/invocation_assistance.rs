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

//! Registry shapes and advisory assistance: frozen values, unanimous words and declared roles.

use super::*;

#[test]
fn argument_values_are_frozen_before_later_substitutions() {
    let tokens = last_source_tokens("set v OLD; list $v [set v NEW]");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let invocation = resolved_tokens_invocation(registry, Some(test_context()), &tokens).unwrap();
    assert_eq!(invocation.argument_literal(0).as_deref(), Some("OLD"));
    assert_eq!(invocation.arguments[0].as_deref(), Some("${v}"));
    assert!(matches!(
        invocation.effective.words[1],
        WordExpr::Variable { .. }
    ));
    assert_eq!(
        invocation.effective.origins[1],
        InvocationWordOrigin::Written(1)
    );
    assert_eq!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .variable_context
            .substitution_literal_value("$v", registry),
        Some("NEW")
    );
}

#[test]
fn possible_registry_shapes_do_not_establish_execution_facts() {
    let tokens = last_source_tokens("if {$flag} {proc set args {return CUSTOM}}; set x 1");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    assert!(resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none());
    let assistance =
        registry_invocation_assistance(registry, Some(test_context()), &tokens).unwrap();
    assert!(assistance.unknown_residual);
    assert!(
        assistance
            .candidates
            .iter()
            .any(|candidate| candidate.command == "set")
    );
    assert_eq!(
        assistance
            .candidates
            .iter()
            .find(|candidate| candidate.command == "set")
            .unwrap()
            .effective
            .written_argument(0),
        Some(0)
    );
}

#[test]
fn unanimous_advisory_words_keep_all_lookup_residuals() {
    let tokens = last_source_tokens("set target VALUE");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let original = registry_invocation_assistance(registry, Some(test_context()), &tokens)
        .expect("stock command has advisory candidates");
    let first = original
        .candidates
        .first()
        .expect("stock candidate")
        .clone();
    let mut assistance = RegistryInvocationAssistance {
        candidates: vec![first.clone(), first.clone()],
        unknown_residual: false,
        may_be_absent: false,
    };
    let view = assistance
        .unanimous_command_words()
        .expect("unanimous words");
    assert_eq!(view.command(), first.command);
    assert_eq!(view.effective(), &first.effective);
    // Role agreement is a separate purpose and cannot be borrowed here.
    assistance.candidates[1].roles.clear();
    assistance.candidates[1].roles_complete = false;
    assert!(assistance.unanimous_command_words().is_some());
    for (unknown_residual, may_be_absent) in [(true, false), (false, true), (true, true)] {
        assistance.unknown_residual = unknown_residual;
        assistance.may_be_absent = may_be_absent;
        assert!(assistance.unanimous_command_words().is_none());
    }
    assistance.unknown_residual = false;
    assistance.may_be_absent = false;
    assistance.candidates.clear();
    assert!(assistance.unanimous_command_words().is_none());
    for malformed in [
        EffectiveCommandWords {
            words: Vec::new(),
            ..first.effective.clone()
        },
        EffectiveCommandWords {
            origins: Vec::new(),
            ..first.effective.clone()
        },
        EffectiveCommandWords {
            origins: vec![InvocationWordOrigin::BindingPrefix(0); first.effective.words.len()],
            binding_prefix: Vec::new(),
            ..first.effective.clone()
        },
    ] {
        assistance.candidates.push(RegistryInvocationShape {
            effective: malformed,
            ..first.clone()
        });
        assert!(assistance.unanimous_command_words().is_none());
        assistance.candidates.clear();
    }
}

#[test]
fn unanimous_advisory_words_require_identity_values_and_origins() {
    let tokens = last_source_tokens("set target VALUE");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let original = registry_invocation_assistance(registry, Some(test_context()), &tokens)
        .expect("stock command has advisory candidates");
    let first = original
        .candidates
        .first()
        .expect("stock candidate")
        .clone();
    let mut alternatives = Vec::new();
    let mut renamed = first.clone();
    renamed.command.push_str("_other");
    alternatives.push(renamed);
    let mut changed_word = first.clone();
    changed_word.effective.words[1] = literal("other_target");
    alternatives.push(changed_word);
    let mut changed_origin = first.clone();
    changed_origin.effective.origins[1] = InvocationWordOrigin::BindingPrefix(0);
    alternatives.push(changed_origin);
    let mut changed_prefix = first.clone();
    changed_prefix
        .effective
        .binding_prefix
        .push(EffectiveInvocationWord::Dynamic);
    alternatives.push(changed_prefix);
    for alternative in alternatives {
        let assistance = RegistryInvocationAssistance {
            candidates: vec![first.clone(), alternative],
            unknown_residual: false,
            may_be_absent: false,
        };
        assert!(assistance.unanimous_command_words().is_none());
    }
}

#[test]
fn role_assistance_maps_alias_prefixes_to_only_original_operands() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    for (source, expected) in [
        (
            "proc p {} {}; info body p",
            vec![(1, tcl_registry::ArgRole::CommandName)],
        ),
        (
            "proc p {} {}; interp alias {} inspect {} info; inspect body p",
            vec![(1, tcl_registry::ArgRole::CommandName)],
        ),
        (
            "proc p {} {}; interp alias {} inspect {} info body; inspect p",
            vec![(0, tcl_registry::ArgRole::CommandName)],
        ),
        (
            "proc p {} {}; interp alias {} inspect {} info body p; inspect",
            vec![],
        ),
        (
            "interp alias {} declare {} proc p; declare {x} {}",
            vec![
                (0, tcl_registry::ArgRole::ParamList),
                (1, tcl_registry::ArgRole::Body),
            ],
        ),
        (
            "interp alias {} declare {} proc p {x}; declare {}",
            vec![(0, tcl_registry::ArgRole::Body)],
        ),
        (
            "interp alias {} inspect {} info body; rename inspect {}; proc inspect args {}; inspect p",
            vec![],
        ),
    ] {
        let tokens = last_source_tokens(source);
        let roles =
            invocation_argument_role_assistance(registry, test_context(), &surface, &tokens);
        assert_eq!(roles, expected, "{source}");
    }
}

#[test]
fn definite_roles_require_closed_actual_alternatives_and_original_positions() {
    use tcl_registry::ArgRole::{Body, Name, ParamList};
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("proc p {x} {}", vec![(0, Name), (1, ParamList), (2, Body)]),
        (
            "interp alias {} declare {} proc p; declare {x} {}",
            vec![(0, ParamList), (1, Body)],
        ),
        (
            "interp alias {} declare {} proc p {x}; declare {}",
            vec![(0, Body)],
        ),
        (
            "rename proc original; original proc args {return ordinary}; proc p {{a b c}} {}",
            vec![],
        ),
        (
            "if {$unknown} {rename proc original; original proc args {return ordinary}}; proc p {{a b c}} {}",
            vec![],
        ),
        ("external p {{a b c}} {}", vec![]),
        ("proc p {*}$unknown", vec![]),
    ] {
        let tokens = last_source_tokens(source);
        assert_eq!(
            invocation_argument_role_consensus(registry, test_context(), &tokens),
            expected,
            "{source}"
        );
    }
    let opaque = last_source_tokens("proc p {{a b c}} {}")
        .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    assert_eq!(
        invocation_argument_role_consensus(registry, test_context(), &opaque),
        vec![]
    );
}

#[test]
fn unloaded_catalogue_assistance_never_restores_a_live_implementation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let tokens = last_source_tokens("::struct::tree tree");
    assert!(resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none());
    let assistance = catalogue_invocation_assistance(registry, test_context(), &tokens)
        .expect("catalogue metadata at an unchanged qualified slot");
    assert_eq!(assistance.shape.command, "struct::tree");
    let shadowed = last_source_tokens(
        "namespace eval ::struct {}; proc ::struct::tree args {return ordinary}; ::struct::tree tree",
    );
    assert!(catalogue_invocation_assistance(registry, test_context(), &shadowed).is_none());
    let unregistered = last_source_tokens("::external::factory tree");
    assert!(catalogue_invocation_assistance(registry, test_context(), &unregistered).is_none());
}

#[test]
fn explicit_point_uncertainty_never_recovers_written_builtin_facts() {
    let registry = CommandRegistry::build_default();
    let command = segment_commands("set x 1").remove(0);
    let tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new("set x 1"),
        tcl_lexer::LexerConfig::default(),
        &command,
    )
    .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    assert!(effective_command_words(&tokens).is_none());
    assert!(matches!(
        resolve_command_tokens(&registry, Some(test_context()), &tokens),
        Ok(RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedBinding { unknown: true, .. }
        ))
    ));
}

#[test]
fn declared_roles_remain_separate_from_runtime_semantics() {
    use tcl_registry::model::DocumentCommandSurface;
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::default();
    let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub custom {destination:var}\n# tcl-lsp: stubs-end\ncustom result";
    let declared = crate::analyser::utils::document_declared_surface(source, None, "tcl8.6");
    let bindings = crate::command_binding::SourceCommandBindings::analyse_in_namespace_with_options(
        source,
        "::",
        config,
        &registry,
        crate::command_binding::SourceAnalysisOptions {
            declared_commands: Some(&declared),
            ..Default::default()
        },
    );
    let segment = segment_commands(source).remove(0);
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("custom", segment.span.start()));
    let surface = DocumentCommandSurface::new(&registry, Some(&declared));
    let assistance =
        resolved_declared_assistance(&surface, &tokens).expect("untouched declared lookup slot");
    assert_eq!(assistance.roles, [(0, tcl_registry::ArgRole::VarWrite)]);
    assert!(matches!(
        resolve_command_tokens(&registry, None, &tokens),
        Ok(RegistryInvocationResolution::Unresolved(_))
    ));
    assert!(resolved_tokens_invocation(&registry, None, &tokens).is_none());
}

#[test]
fn nested_dispatch_inheritance_never_recovers_a_missing_point() {
    let source = "set r [expr {1 + 2}]";
    let config = tcl_lexer::LexerConfig::default();
    let segment = segment_commands(source).remove(0);
    let parent =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    let word = &parent.words()[2];
    let mut nested =
        crate::word_subst::nested_command_words(&word.legacy_text(), word.source(), config)
            .unwrap();
    nested.inherit_nested_bindings(&parent);
    assert!(nested.has_unproved_source_binding());
    assert!(
        resolved_tokens_invocation(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &nested
        )
        .is_none()
    );
}

fn advisory_metadata_tokens(source: &str, registry: &CommandRegistry) -> CommandTokens {
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let command = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command)
        .with_source_binding(
            bindings.invocation_at_source("unused-reporting-head", command.span.start()),
        )
}

#[test]
fn advisory_value_metadata_shares_supplied_availability_and_store_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Registry May candidates only; no actual assignment or returned object.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    for name in ["set", "return"] {
        let mut descriptor = registry.get(name).unwrap().clone();
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(descriptor);
    }
    let generation = baseline.with_command_store(std::sync::Arc::new(registry));
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(generation.commands()));
    let registry = generation.commands();
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let mut assignment = advisory_metadata_tokens("set target VALUE", registry);
    let mut returning = advisory_metadata_tokens("return VALUE", registry);
    let current = advisory_value_assignments_in_context(&generation, &assignment);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].name, "target");
    assert_eq!(current[0].value, assignment.words()[2]);
    assert_eq!(
        advisory_return_values_in_context(&generation, &returning),
        vec![returning.words()[1].clone()]
    );
    assert_eq!(
        advisory_value_assignments_in_context(&older, &assignment),
        [] as [crate::registry_invocation::AdvisoryValueAssignment; 0]
    );
    assert_eq!(
        advisory_return_values_in_context(&older, &returning),
        [] as [tcl_source::words::WordExpr; 0]
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert_eq!(
        advisory_value_assignments_with_metadata_context(
            registry,
            Some(foreign.into()),
            &assignment
        ),
        [] as [crate::registry_invocation::AdvisoryValueAssignment; 0]
    );
    assert_eq!(
        advisory_return_values_with_metadata_context(registry, Some(foreign.into()), &returning),
        [] as [tcl_source::words::WordExpr; 0]
    );
    assignment.source_binding = None;
    returning.source_binding = None;
    assert_eq!(
        advisory_value_assignments_in_context(&generation, &assignment),
        [] as [crate::registry_invocation::AdvisoryValueAssignment; 0]
    );
    assert_eq!(
        advisory_return_values_in_context(&generation, &returning),
        [] as [tcl_source::words::WordExpr; 0]
    );
}
