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

//! Invocation metadata contexts: availability, generation and retained script ordinals.

use super::*;

#[test]
fn original_metadata_context_keeps_actual_availability_and_generation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = generation.commands();
    let old = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let tokens = last_source_tokens("dict for {key value} {} {puts nested}");
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    assert_ne!(
        invocation_argument_role_assistance(registry, generation, &surface, &tokens),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        invocation_argument_role_assistance(registry, &old, &surface, &tokens),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        invocation_argument_role_consensus(registry, &old, &tokens),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    let body = invocation_body_assistance(registry, &old, &surface, &tokens);
    assert!(body.possible_roles.is_empty() && body.definite_roles.is_empty());
    assert!(body.unknown_residual && body.definite_invocation.is_none());
    let set = last_source_tokens("set target value");
    assert!(original_variable_write_advice(registry, generation, &set).is_some());
    let foreign =
        generation.with_command_store(std::sync::Arc::new(CommandRegistry::build_default()));
    assert_ne!(
        foreign.commands().snapshot().semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert_eq!(
        invocation_argument_role_assistance(registry, &foreign, &surface, &set),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        invocation_argument_role_consensus(registry, &foreign, &set),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(original_variable_write_advice(registry, &foreign, &set).is_none());
    assert!(
        original_symbol_declaration_advice(registry, &foreign, &last_source_tokens("proc p {} {}"))
            .is_none()
    );
}

#[test]
fn retained_script_metadata_keeps_actual_context_and_effective_ordinals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for (source, argument, expected) in [
        (
            "trace add variable v write changed",
            4,
            tcl_registry::ScriptTiming::Deferred,
        ),
        (
            "trace remove variable v write changed",
            4,
            tcl_registry::ScriptTiming::ReferenceOnly,
        ),
        (
            "lsort -command compare {b a}",
            1,
            tcl_registry::ScriptTiming::SameInvocation,
        ),
        (
            "interp alias {} install {} trace add variable v write; install changed",
            4,
            tcl_registry::ScriptTiming::Deferred,
        ),
    ] {
        let tokens = last_source_tokens(source);
        let invocation = resolved_tokens_invocation_in_context(generation, &tokens)
            .expect("authentic selected invocation");
        let realm = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_realm()
            .unwrap();
        assert_eq!(
            invocation.with_metadata_schema(
                generation.commands(),
                generation.into(),
                realm,
                |selected| selected.authored_source_script_timing_at(argument)
            ),
            Some(expected),
            "{source}"
        );
        assert!(
            invocation
                .with_metadata_schema(generation.commands(), foreign.into(), realm, |selected| {
                    selected.authored_source_script_timing_at(argument)
                })
                .is_none()
        );
    }
}

#[test]
fn original_body_metadata_keeps_supplied_availability_and_generation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(generation.commands()));
    let source = "dict for {key value} {one two} {set found $value}";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let bindings = crate::command_binding::SourceCommandBindings::analyse(
        source,
        config,
        generation.commands(),
    );
    let surface = tcl_registry::model::DocumentCommandSurface::new(generation.commands(), None);
    let command =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let current = segmented_body_assistance(
        &surface,
        &bindings,
        source,
        config,
        &command,
        0,
        Some(generation),
    )
    .unwrap();
    assert!(
        current
            .possible_roles
            .contains(&(3, tcl_registry::ArgRole::Body))
    );
    let unavailable = segmented_body_assistance(
        &surface,
        &bindings,
        source,
        config,
        &command,
        0,
        Some(&older),
    )
    .unwrap();
    assert_eq!(
        unavailable.possible_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(unavailable.unknown_residual);
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert!(
        segmented_body_assistance(
            &surface,
            &bindings,
            source,
            config,
            &command,
            0,
            Some(foreign)
        )
        .is_none()
    );
    assert!(
        unpositioned_body_assistance(&surface, &bindings, source, config, &command, Some(foreign))
            .is_none()
    );
}

#[test]
fn retained_invocation_context_keeps_availability_without_donating_handler_proof() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(generation.commands()));
    let tokens = last_source_tokens("dict create key value");
    assert!(resolved_tokens_invocation_in_context(generation, &tokens).is_some());
    assert!(resolved_tokens_invocation_in_context(&older, &tokens).is_none());
    assert!(normal_representation_invocation_in_context(generation, &tokens).is_some());
    assert!(normal_representation_invocation_in_context(&older, &tokens).is_none());
    assert!(normal_transfer_invocation_in_context(generation, &tokens).is_some());
    assert!(normal_transfer_invocation_in_context(&older, &tokens).is_none());
    let names = possible_variable_name_operands_in_context(generation, &tokens).unwrap();
    assert!(!names.candidates.is_empty());
    let unavailable_names = possible_variable_name_operands_in_context(&older, &tokens).unwrap();
    assert!(unavailable_names.candidates.is_empty() && unavailable_names.unknown_residual());
    let assistance = registry_invocation_assistance_in_context(generation, &tokens).unwrap();
    assert_ne!(
        assistance.candidates,
        [] as [crate::registry_invocation::RegistryInvocationShape; 0]
    );
    let unavailable = registry_invocation_assistance_in_context(&older, &tokens).unwrap();
    assert!(unavailable.candidates.is_empty() && unavailable.unknown_residual);
    let replaced = last_source_tokens("proc dict args {}; dict create key value");
    assert!(resolved_tokens_invocation_in_context(generation, &replaced).is_none());
    assert!(normal_representation_invocation_in_context(generation, &replaced).is_none());
    assert!(normal_transfer_invocation_in_context(generation, &replaced).is_none());
    let replaced_names = possible_variable_name_operands_in_context(generation, &replaced).unwrap();
    assert!(replaced_names.candidates.is_empty() && replaced_names.unknown_residual());
    let replaced_assistance =
        registry_invocation_assistance_in_context(generation, &replaced).unwrap();
    assert!(replaced_assistance.candidates.is_empty() && replaced_assistance.unknown_residual);
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            generation.into(),
            &tokens,
            None
        )
        .is_some()
    );
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            (&older).into(),
            &tokens,
            None
        )
        .is_none()
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            foreign.into(),
            &tokens,
            None
        )
        .is_none()
    );
    let mut unproved = tokens;
    unproved.source_binding = None;
    assert!(resolved_tokens_invocation_in_context(generation, &unproved).is_none());
    assert!(normal_representation_invocation_in_context(generation, &unproved).is_none());
    assert!(normal_transfer_invocation_in_context(generation, &unproved).is_none());
    assert!(possible_variable_name_operands_in_context(generation, &unproved).is_none());
    assert!(registry_invocation_assistance_in_context(generation, &unproved).is_none());
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            generation.into(),
            &unproved,
            None
        )
        .is_none()
    );
    assert!(
        original_callback_invocation_with_metadata_context(
            generation.commands(),
            generation.into(),
            &unproved
        )
        .is_none()
    );
}
