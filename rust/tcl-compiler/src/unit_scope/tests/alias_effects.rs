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

//! Alias creation covered by an original effect: publication, prefix owners and refusals.

use super::*;

fn alias_creation_tokens(module: &IrModule) -> crate::ir::CommandTokens {
    module
        .top_level
        .statements
        .iter()
        .find_map(|statement| {
            statement.tokens().filter(|tokens| {
                tokens
                    .argv_texts
                    .first()
                    .is_some_and(|head| head == "interp")
            })
        })
        .expect("actual original alias declaration")
        .clone()
}

fn alias_binding_transition(
    tokens: &crate::ir::CommandTokens,
    module: &IrModule,
    registry: &CommandRegistry,
) -> tcl_registry::CommandBindingTransition {
    let binding = tokens.source_binding.as_ref().unwrap();
    let metadata = binding
        .original_invocation_metadata_for_module(tokens, module, registry)
        .unwrap();
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )
    .unwrap();
    invocation
        .facts
        .state_transitions
        .declared()
        .unwrap()
        .command_bindings()
        .find(|transition| {
            matches!(
                transition,
                tcl_registry::CommandBindingTransition::Alias { .. }
            )
        })
        .unwrap()
        .clone()
}

#[test]
fn original_covered_alias_effect_keeps_actual_publication_and_prefix_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The held Vm supplies actual Native source entry, not successful alias execution.
    let (context, unit) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
    );
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let tokens = alias_creation_tokens(module);
    let transition = alias_binding_transition(&tokens, module, context.commands());
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                input,
                context.commands(),
                &transition,
            )
            .is_some()
    );
    let evidence = unit.caller_scope.call_sites.get("::selected").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(evidence.uniform_literal_at(0), Some("PREFIX"));
    assert_eq!(evidence.uniform_literal_at(1), Some("FIXED"));
    let tcl_registry::CommandBindingTransition::Alias { arguments, .. } = &transition else {
        unreachable!()
    };
    let mut changed = transition.clone();
    let tcl_registry::CommandBindingTransition::Alias {
        arguments: changed_arguments,
        ..
    } = &mut changed
    else {
        unreachable!()
    };
    assert_ne!(arguments.len(), 0);
    changed_arguments.clear();
    assert!(
            binding
                .original_covered_alias_transition(
                    &tokens,
                    module,
                    input,
                    context.commands(),
                    &changed,
                )
                .is_none()
        );
}

#[test]
fn original_covered_alias_effect_refuses_missing_foreign_source_and_vector_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (context, unit) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
    );
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let tokens = alias_creation_tokens(module);
    let transition = alias_binding_transition(&tokens, module, context.commands());
    let binding = tokens.source_binding.as_ref().unwrap();
    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    let mut absent = module.clone();
    absent.source_metadata_input = None;
    let mut foreign_source = module.clone();
    foreign_source.source = tcl_lexer::SourceImage::document("proc foreign {} {}");
    let mut config = input.lexer_config();
    config.leading_bom = tcl_lexer::LeadingBom::Skip;
    let changed_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    for candidate in [&missing, &absent, &foreign_source] {
        assert!(
            binding
                .original_covered_alias_transition(
                    &tokens,
                    candidate,
                    input,
                    context.commands(),
                    &transition,
                )
                .is_none()
        );
    }
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                &changed_input,
                context.commands(),
                &transition,
            )
            .is_none()
    );
    let foreign = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                input,
                foreign.commands(),
                &transition,
            )
            .is_none()
    );
    let mut altered = tokens.clone();
    altered.argv_texts.last_mut().unwrap().push_str("foreign");
    assert!(
        binding
            .original_covered_alias_transition(
                &altered,
                module,
                input,
                context.commands(),
                &transition,
            )
            .is_none()
    );
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                input,
                context.commands(),
                &transition,
            )
            .is_some()
    );
}

#[test]
fn original_covered_alias_effect_keeps_rebinding_unknown_and_callback_opacity() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    for source in [
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; rename forwarded moved; moved FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; unknown; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; after 0 forwarded; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded FOREIGN selected PREFIX; forwarded FIXED",
    ] {
        let (_, unit) = native_argument_caller_unit(source);
        assert!(
            unit.caller_scope
                .call_sites
                .get("::selected")
                .unwrap()
                .opaque_caller,
            "{source}"
        );
    }
}

#[test]
fn original_covered_alias_effect_requires_original_entered_continuation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (context, current) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
    );
    let tokens = alias_creation_tokens(&current.ir_module);
    let transition = alias_binding_transition(&tokens, &current.ir_module, context.commands());
    for source in [
        "proc selected {prefix value} {}; return DONE; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
        "proc selected {prefix value} {}; unknown; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded FOREIGN selected PREFIX; forwarded FIXED",
    ] {
        let (context, unit) = native_argument_caller_unit(source);
        let module = &unit.ir_module;
        let tokens = alias_creation_tokens(module);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_covered_alias_transition(
                    &tokens,
                    module,
                    module.source_metadata_input.as_ref().unwrap(),
                    context.commands(),
                    &transition,
                )
                .is_none(),
            "{source}"
        );
    }
}
