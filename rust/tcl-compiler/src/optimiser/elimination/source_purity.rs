// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original substitution purity, separate from evaluation and edit permission.

use super::{EffectCtx, PurityCtx};
use crate::ir::CommandTokens;

pub(super) fn substitutions_are_pure(effect: EffectCtx<'_>) -> bool {
    let purity = effect.purity;
    let (Some(registry), Some(metadata), Some(tokens)) =
        (purity.registry, purity.metadata, effect.source_tokens)
    else {
        return false;
    };
    if !metadata.matches_registry(registry) {
        return false;
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
        return false;
    };
    if config.normalized() != purity.config.normalized() || tokens.nested_bindings.is_empty() {
        return false;
    }
    tokens.nested_bindings.iter().all(|(_, original)| {
        let Some((_, mut nested)) = original.original_recorded_command() else {
            return false;
        };
        nested.inherit_nested_bindings(tokens);
        match crate::registry_invocation::original_substitution_purity_with_metadata_context(
            registry, metadata, &nested,
        ) {
            Some(pure) => pure,
            None => logical_procedure_is_pure(&nested, purity),
        }
    })
}

fn logical_procedure_is_pure(tokens: &CommandTokens, purity: PurityCtx<'_>) -> bool {
    let (Some(metadata), Some(module), Some(registry)) =
        (purity.metadata, purity.module, purity.registry)
    else {
        return false;
    };
    if !metadata.permits_logical_source_names() {
        return false;
    }
    let Some(calls) = crate::registry_invocation::original_logical_procedure_calls_for_module(
        tokens, module, registry,
    ) else {
        return false;
    };
    !calls.is_empty()
        && calls.iter().all(|call| {
            purity
                .interproc_pure
                .contains(&call.procedure().qualified_name)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::optimiser::elimination::assignment_safe_to_delete_at;
    use std::{collections::HashSet, sync::Arc};
    use tcl_registry::model::ContextRegistry;

    fn unit(
        source: &str,
        context: &Arc<ContextRegistry>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> CompilationUnit {
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    fn check(unit: &CompilationUnit, registry: &tcl_registry::CommandRegistry) -> bool {
        check_with_procedure_summary(unit, registry, &HashSet::new())
    }

    fn check_with_procedure_summary(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        pure: &HashSet<String>,
    ) -> bool {
        let function = &unit.top_level;
        let purity = PurityCtx {
            registry: Some(registry),
            interproc_pure: pure,
            config: function.source_lexer_config(),
            module: Some(&unit.ir_module),
            metadata: function.invocation_metadata_context_for_module(registry, &unit.ir_module),
        };
        let (block, index) = function
            .cfg
            .blocks
            .iter()
            .find_map(|(block, contents)| {
                contents
                    .statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        matches!(statement,
                            crate::ir::Statement::AssignConst { name, .. }
                            | crate::ir::Statement::AssignValue { name, .. }
                            | crate::ir::Statement::AssignExpr { name, .. } if name == "unused"
                        )
                        .then_some((*block, index))
                    })
            })
            .expect("actual source assignment");
        assignment_safe_to_delete_at(function, block, index, purity)
    }

    #[test]
    fn original_substitution_purity_keeps_alias_prefixes_and_target_barriers() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Source-model purity only. Other erasure/evaluation premises remain open.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        for (source, expected) in [
            ("set unused [string length VALUE]", true),
            (
                "interp alias {} len {} string length; set unused [len VALUE]",
                true,
            ),
            (
                "rename string moved; interp alias {} len {} moved length; set unused [len café]",
                true,
            ),
            (
                "proc string args {puts CHANGED}; set unused [string length VALUE]",
                false,
            ),
            (
                "interp alias {} len {} string length; rename string moved; set unused [len VALUE]",
                false,
            ),
            (
                "interp alias {} add {} dict set data; set unused [add key VALUE]",
                false,
            ),
            ("set unused [puts VALUE]", false),
            ("set unused [expr {[puts VALUE]}]", false),
            ("set unused [expr {[string length VALUE]}]", true),
        ] {
            let selected = unit(source, &context, profile);
            assert_eq!(check(&selected, context.commands()), expected, "{source}");
        }
    }

    #[test]
    fn original_substitution_purity_requires_actual_availability_and_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "metadata_pure",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("string").unwrap().clone()
        });
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        for (context, expected) in [(&current, true), (&older, false)] {
            let selected = unit("set unused [metadata_pure length VALUE]", context, profile);
            assert_eq!(check(&selected, &registry), expected);
        }
        let mut selected = unit("set unused [string length VALUE]", &current, profile);
        assert!(check(&selected, &registry));
        selected.ir_module.source_metadata_input = None;
        assert!(!check(&selected, &registry));
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        selected = unit("set unused [string length VALUE]", &current, profile);
        assert!(!check(&selected, foreign.commands()));
        selected.ir_module.lexer_config.strict_quoting =
            !selected.ir_module.lexer_config.strict_quoting;
        assert!(!check(&selected, &registry));
    }

    #[test]
    fn native_substitution_metadata_does_not_borrow_logical_purity() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry();
            let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
            let selected = unit("set unused [string length VALUE]", &context, profile);
            let script = &selected.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let empty = HashSet::new();
            let purity = PurityCtx {
                registry: Some(context.commands()),
                interproc_pure: &empty,
                config: selected.top_level.source_lexer_config(),
                module: Some(&selected.ir_module),
                metadata: selected.top_level.invocation_metadata_context_for_module(
                    context.commands(),
                    &selected.ir_module,
                ),
            };
            assert!(
                !substitutions_are_pure(EffectCtx {
                    purity,
                    source_tokens: Some(tokens)
                }),
                "no independent Native entry: {environment}"
            );
        }
    }

    #[test]
    fn literal_assignment_purity_keeps_no_evaluation_without_metadata() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let mut selected = unit(
            "set unused {[puts VALUE]}",
            &context,
            tcl_dialect::DialectProfile::plain_tcl(),
        );
        selected.ir_module.source_metadata_input = None;
        // The original braced literal is AssignConst; no substitution dispatch
        // query or effect conclusion about puts is needed for this value gate.
        assert!(check(&selected, context.commands()));
    }

    #[test]
    fn logical_procedure_purity_joins_the_original_declaration_not_its_label() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // A supplied source summary still needs the exact original allocation.
        // This checks that join, not the summary's independent effect analysis.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let summary = ["::pure".to_owned()].into_iter().collect();
        let original = unit(
            "proc pure {x} {return $x}; interp alias {} called {} pure; set unused [called VALUE]",
            &context,
            profile,
        );
        assert!(check_with_procedure_summary(
            &original,
            context.commands(),
            &summary
        ));
        let mut changed = original.clone();
        changed
            .ir_module
            .procedures
            .get_mut("::pure")
            .unwrap()
            .params_raw = "{x DEFAULT}".into();
        assert!(!check_with_procedure_summary(
            &changed,
            context.commands(),
            &summary
        ));
        let replaced = unit(
            "proc pure {x} {return $x}; set unused [pure VALUE]; proc pure {x} {puts $x}",
            &context,
            profile,
        );
        assert!(!check_with_procedure_summary(
            &replaced,
            context.commands(),
            &summary
        ));
    }
}
