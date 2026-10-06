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

//! Original procedure-body layouts independent of command installation.

use std::sync::Arc;

use super::declaration_layout::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
    original_declaration_layouts,
};
use super::{
    CommandAllocationSite, ExecutedScriptSource, SourceCommandBindings, SourceLookupSnapshot,
};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target, resolve_registry_words_in_realm,
};

/// A positioned conditional declaration recipe, never an implementation allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredProcedureBody {
    declaration: CommandAllocationSite,
    candidate: super::SourceCommandTarget,
    slot: super::SourceCommandKey,
    pub(super) source: ExecutedScriptSource,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    pub(super) frame: crate::var_resolve::VariableExecutionFrame,
}

impl SourceCommandBindings {
    /// Preserve only original lexical layouts when the declaration's runtime
    /// installation is unknown. No dispatch, body entry or effect inventory is
    /// populated by this separate collector.
    pub(super) fn retain_uninstalled_declaration_body_layouts(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let roots: Vec<_> = self
            .declaration_layouts
            .iter()
            .filter_map(|(site, observations)| {
                let observation = original_declaration_layouts(observations)?.next()?;
                matches!(
                    observation.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::RootScript { .. }
                )
                .then(|| (site.clone(), observation.clone()))
            })
            .collect();
        for (site, observation) in roots {
            let Some(tokens) = declaration_tokens(&site, &observation) else {
                continue;
            };
            let Some(body) = declared_body(&site, &tokens, &observation, registry) else {
                continue;
            };
            // An independently retained installed declaration has its own
            // collector and ownership; never join it with this lexical recipe.
            if self
                .conditional_body_entry_at(&body.source.origin, body.source.base())
                .is_some()
            {
                continue;
            }
            self.retain_declared_body_layouts(body, &observation, registry);
        }
        self.retain_nested_declared_layouts(registry);
    }

    fn retain_nested_declared_layouts(&mut self, registry: &tcl_registry::CommandRegistry) {
        let roots: Vec<_> = self
            .declaration_layouts
            .iter()
            .filter_map(|(site, observations)| {
                let originals = original_declaration_layouts(observations)?;
                Some((site.clone(), originals.cloned().collect::<Vec<_>>()))
            })
            .collect();
        let mut pending = Vec::new();
        for (site, observations) in roots {
            let Some(tokens) = declaration_tokens(&site, &observations[0]) else {
                continue;
            };
            pending.extend(unanimous_nested_layouts(
                &site,
                &tokens,
                &observations,
                registry,
            ));
        }
        while let Some((source, parents)) = pending.pop() {
            let config = parents[0].config;
            let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
                &source.text,
                source.base(),
                config,
            ) else {
                continue;
            };
            for segment in segments {
                if segment.is_partial {
                    break;
                }
                let tokens = super::source_command_tokens_boxed(
                    &source.text,
                    source.base(),
                    config,
                    &segment,
                );
                let site = CommandAllocationSite {
                    source: Arc::clone(&source.origin),
                    offset: segment.span.start(),
                };
                let observations: Vec<_> = parents
                    .iter()
                    .map(|parent| DeclarationLayoutObservation {
                        words: Arc::from(tokens.words()),
                        ..parent.clone()
                    })
                    .collect();
                let retained = self.declaration_layouts.entry(site.clone()).or_default();
                for observation in &observations {
                    if !retained.contains(observation) {
                        retained.push(observation.clone());
                    }
                }
                pending.extend(unanimous_nested_layouts(
                    &site,
                    &tokens,
                    &observations,
                    registry,
                ));
                if parents.iter().any(|parent| {
                    !retains_successor_layout(
                        &site,
                        &tokens,
                        &parent.snapshot,
                        &parent.namespace,
                        registry,
                    )
                }) {
                    break;
                }
            }
        }
    }

    fn retain_declared_body_layouts(
        &mut self,
        body: DeclaredProcedureBody,
        declaration: &DeclarationLayoutObservation,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &body.source.text,
            body.source.base(),
            declaration.config,
        ) else {
            return;
        };
        let mut state = declaration.snapshot.state.clone();
        state.variable_frame = body.frame.clone();
        state.source_variables = Arc::new(
            state
                .source_variables
                .in_frame(&body.frame)
                .with_namespace_identity(body.namespace.clone()),
        );
        for parameter in &body.parameters {
            Arc::make_mut(&mut state.source_variables)
                .bind_unknown_incoming(&parameter.name, registry);
        }
        state.current_source_origin = Some(Arc::clone(&body.source.origin));
        let namespace = body.namespace.clone();
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            declaration.snapshot.realm,
        ));
        let entry = Arc::new(OriginalDiagnosticFrameEntry::DeclaredProcedure(Arc::new(
            body,
        )));
        for segment in segments {
            if segment.is_partial {
                break;
            }
            let tokens = super::source_command_tokens_boxed(
                &entry.source().text,
                entry.source().base(),
                declaration.config,
                &segment,
            );
            let site = CommandAllocationSite {
                source: Arc::clone(&entry.source().origin),
                offset: segment.span.start(),
            };
            let observations = self.declaration_layouts.entry(site.clone()).or_default();
            let observation = DeclarationLayoutObservation {
                issuer: DeclarationLayoutIssuer::OriginalDeclaration,
                entry: Arc::clone(&entry),
                snapshot: Arc::clone(&snapshot),
                namespace: namespace.clone(),
                config: declaration.config,
                words: Arc::from(tokens.words()),
            };
            if !observations.contains(&observation) {
                observations.push(observation);
            }
            if !retains_successor_layout(&site, &tokens, &snapshot, &namespace, registry) {
                break;
            }
        }
    }
}

fn unanimous_nested_layouts(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parents: &[DeclarationLayoutObservation],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let mut agreed: Option<Vec<ExecutedScriptSource>> = None;
    for parent in parents {
        let sources = nested_conditional_layouts(site, tokens, parent, registry)
            .into_iter()
            .map(|(source, _)| source)
            .collect::<Vec<_>>();
        if agreed.as_ref().is_some_and(|previous| previous != &sources) {
            return Vec::new();
        }
        agreed = Some(sources);
    }
    agreed
        .unwrap_or_default()
        .into_iter()
        .map(|source| (source, parents.to_vec()))
        .collect()
}

fn nested_conditional_layouts(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parent: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<(ExecutedScriptSource, DeclarationLayoutObservation)> {
    let Some(advice) = super::original_site_operand_layout_advice(
        site,
        tokens,
        &parent.snapshot,
        &parent.namespace,
    ) else {
        return Vec::new();
    };
    let Some(selected) =
        crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)
    else {
        return Vec::new();
    };
    let tcl_registry::script_body_flow::ScriptBodyFlow::Conditional(branches) = &selected.flow
    else {
        return Vec::new();
    };
    // Substitutions in argv precede every condition and may change its lookup.
    if selected.effective.words.iter().any(|word| {
        effective_invocation_word(word, parent.config.escapes, advice.dialect().word_values)
            .literal_bytes()
            .is_none()
    }) {
        return Vec::new();
    }
    let mut children = Vec::new();
    for &(condition, body) in branches {
        if condition.is_some_and(|argument| {
            !condition_retains_original_lookup(
                site,
                argument,
                &selected.effective,
                parent,
                &advice,
                registry,
            )
        }) {
            break;
        }
        if let Some(source) = original_nested_body(site, body, &selected.effective, parent, &advice)
        {
            children.push((source, parent.clone()));
        }
    }
    children
}

fn original_nested_body(
    site: &CommandAllocationSite,
    argument: usize,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    parent: &DeclarationLayoutObservation,
    advice: &super::OriginalCompilationLookupAdvice,
) -> Option<ExecutedScriptSource> {
    let written = effective.written_argument(argument)?;
    let word = effective.words.get(argument + 1)?;
    let EffectiveInvocationWord::Literal(value) =
        effective_invocation_word(word, parent.config.escapes, advice.dialect().word_values)
    else {
        return None;
    };
    let source =
        ExecutedScriptSource::from_word(site.clone(), written, word, &value, parent.config);
    let super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
        return None;
    };
    let end = base.checked_add(u32::try_from(source.text.len()).ok()?)?;
    (source.origin == site.source
        && base > site.offset
        && parent.entry.owns_source(&source.origin, base)
        && end
            .checked_sub(1)
            .is_some_and(|last| parent.entry.owns_source(&source.origin, last)))
    .then_some(source)
}

fn condition_retains_original_lookup(
    site: &CommandAllocationSite,
    argument: usize,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    parent: &DeclarationLayoutObservation,
    advice: &super::OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    use tcl_syntax::expr::ast::ExprNode;
    let Some(source) = original_nested_body(site, argument, effective, parent, advice) else {
        return false;
    };
    let Ok(text) = source.text.try_text() else {
        return false;
    };
    let tcl_syntax::expr::parser::CheckedExprParse::Parsed(expression) =
        tcl_syntax::expr::parser::parse_expr_checked_with_context(
            text,
            &advice.dialect().expression_parse_context(None),
        )
    else {
        return false;
    };
    let context = advice.original_variable_context();
    if context.dynamic_traces {
        return false;
    }
    let mut pending = vec![&expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::Var { text, name, .. } => {
                let reference =
                    tcl_lexer::word_parts::whole_var_ref(text.as_bytes(), parent.config);
                if !matches!(reference, Ok(Some(reference)) if reference.index.is_none())
                    || tcl_syntax::naming::is_qualified(name.as_bytes())
                    || super::declaration_layout::local_read_scope_is_excluded(context, name)
                    || crate::script_binds::script_image_binds_name(
                        &parent.entry.source().text,
                        name,
                        crate::script_binds::Ownership::ScopeAliases,
                        registry,
                        parent.config,
                    )
                {
                    return false;
                }
            }
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => return false,
        }
    }
    true
}

fn declaration_tokens(
    site: &CommandAllocationSite,
    observation: &DeclarationLayoutObservation,
) -> Option<crate::ir::CommandTokens> {
    let text = site.source.try_text().ok()?;
    let first = tcl_lexer::word_span_at(text, observation.words.first()?.source().span);
    let last = tcl_lexer::word_span_at(text, observation.words.last()?.source().span);
    if first.start() != site.offset || first.start() > last.end() {
        return None;
    }
    let image = tcl_lexer::SourceImage::from_bytes(
        text.as_bytes()
            .get(first.start() as usize..last.end() as usize)?,
        site.source.source_image().channel(),
    );
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &image,
        first.start(),
        observation.config,
    )?;
    let [segment] = segments.as_slice() else {
        return None;
    };
    let tokens =
        super::source_command_tokens_boxed(&image, first.start(), observation.config, segment);
    (tokens.words() == observation.words.as_ref()).then_some(*tokens)
}

fn declared_body(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
) -> Option<DeclaredProcedureBody> {
    let advice = super::original_site_operand_layout_advice(
        site,
        tokens,
        &observation.snapshot,
        &observation.namespace,
    )?;
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        let recipe = procedure_body_recipe(
            site,
            tokens,
            observation,
            &facts,
            target,
            &ProcedureOperands {
                effective: &effective,
                values: &values,
                dialect,
            },
        )?;
        if agreed.as_ref().is_some_and(|old| old != &recipe) {
            return None;
        }
        agreed = Some(recipe);
    }
    agreed
}

struct ProcedureOperands<'a> {
    effective: &'a crate::registry_invocation::EffectiveCommandWords,
    values: &'a [EffectiveInvocationWord],
    dialect: tcl_registry::InvocationDialect,
}

fn procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    candidate: &super::SourceCommandTarget,
    operands: &ProcedureOperands<'_>,
) -> Option<DeclaredProcedureBody> {
    use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition};
    let ProcedureOperands {
        effective,
        values,
        dialect,
    } = *operands;
    if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let name = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| {
            if let CommandBindingTransition::Define {
                name,
                kind: CommandBindingDefinitionKind::Procedure,
            } = transition
            {
                name.literal()
            } else {
                None
            }
        })?;
    let key = observation.snapshot.state.publication_key_at(
        &observation.namespace,
        name,
        super::namespace_slots::PublicationPurpose::Procedure,
    )?;
    let namespace = key.holder().into_owned();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments =
        tcl_registry::InvocationArguments::structured(words.get(1..)?).with_dialect(dialect);
    let parameters = super::native_procedure_parameters(facts, arguments)?;
    let body_index = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Body)
            .then_some(facts.argument_offset + usize::from(*index) + 1)
    })?;
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        effective.origins.get(body_index)?
    else {
        return None;
    };
    let EffectiveInvocationWord::Literal(value) = values.get(body_index)? else {
        return None;
    };
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        written.checked_sub(1)?,
        tokens.words().get(*written)?,
        value,
        observation.config,
    );
    if source.origin != site.source {
        return None;
    }
    let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: namespace.display().unwrap_or_default(),
        identity: super::source_activation_name(
            Some(&site.source),
            "declared-procedure-layout",
            site.offset,
        ),
    }
    .with_namespace_identity(namespace.clone());
    Some(DeclaredProcedureBody {
        declaration: site.clone(),
        candidate: candidate.clone(),
        slot: key,
        source,
        namespace,
        parameters,
        frame,
    })
}

fn retains_successor_layout(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &super::SourceNamespaceKey,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some(advice) =
        super::original_site_operand_layout_advice(site, tokens, snapshot, namespace)
    else {
        return false;
    };
    let Some(selected) =
        crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)
    else {
        return false;
    };
    selected.effects.lookup_stable
        && !selected.effects.unknown_writes
        && matches!(
            selected.flow,
            tcl_registry::script_body_flow::ScriptBodyFlow::None
        )
        && selected.effective.words.iter().all(|word| {
            effective_invocation_word(
                word,
                advice.dialect().lexer_grammar.escapes,
                advice.dialect().word_values,
            )
            .literal_bytes()
            .is_some()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(source: &str, command: &str) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        inventory_in_world(source, command, false)
    }

    fn inventory_in_world(
        source: &str,
        command: &str,
        unknown_entry: bool,
    ) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                unknown_entry,
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(command, offset, config)
                .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens)
    }

    #[test]
    fn nested_declared_layout_keeps_original_scope_without_entered_authority() {
        let source = "proc wrap {condition} {if {$condition} {info exists condition}}";
        let (bindings, tokens) = inventory_in_world(source, "info exists condition", true);
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("unchanged scalar condition retains original nested grammar");
        assert!(!layout.closed_lookup());
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let site = binding.invocation_site().unwrap();
        let lexical_body = bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .expect("independent unchanged declaration body recipe");
        assert!(lexical_body.owns_source(&site.source, site.offset));
        assert!(lexical_body.matches_parameters(&["condition"]));
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
        let originals = original_declaration_layouts(
            binding.declaration_layout_observations.as_deref().unwrap(),
        )
        .unwrap();
        assert!(originals.clone().all(|observation| {
            observation.entry.owns_source(&site.source, site.offset)
                && observation
                    .entry
                    .owns_original_context(&observation.snapshot.state.source_variables)
        }));
    }

    #[test]
    fn nested_declared_layout_stops_before_condition_callbacks_or_aliases() {
        for body in [
            "if {[rename info saved; expr 1]} {info exists condition}",
            "if {$array(index)} {info exists condition}",
            "if {[expr {1}]} {info exists condition}",
            "upvar 1 outer condition; if {$condition} {info exists condition}",
            "trace add variable condition read replaceInfo; if {$condition} {info exists condition}",
        ] {
            let source = format!("proc wrap {{condition}} {{{body}}}");
            let (_, tokens) = inventory_in_world(&source, "info exists condition", true);
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&tokens)
                    .is_none(),
                "condition lookup is unavailable after: {body}"
            );
        }
    }

    #[test]
    fn uninstalled_body_layout_retains_original_candidate_without_runtime_authority() {
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let (bindings, tokens) = inventory_in_world(source, "upvar 1 other local", true);
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("conditional original grammar");
        assert!(!layout.closed_lookup());
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let initial = super::super::ModuleCommandBindings::initial(registry);
        let original = super::super::source_binding(
            &initial,
            "upvar",
            &super::super::SourceNamespaceKey::authored("::"),
        );
        assert!(layout.targets().contains(original.proved_target().unwrap()));
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let (_, original_words, original_offset) = binding
            .original_compiler_source(&tokens)
            .expect("unchanged lexical vector is geometry, not compilation admission");
        assert_eq!(original_words, tokens.words());
        assert_eq!(
            original_offset,
            u32::try_from(source.find("upvar").unwrap()).unwrap()
        );
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        let site = binding.invocation_site().unwrap();
        let lexical_body = bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .expect("independent unchanged declaration body recipe");
        assert!(lexical_body.owns_source(&site.source, site.offset));
        assert!(lexical_body.matches_parameters(&["arg"]));
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
        let observations = binding.declaration_layout_observations.as_deref().unwrap();
        let entry = &original_declaration_layouts(observations)
            .unwrap()
            .next()
            .unwrap()
            .entry;
        let OriginalDiagnosticFrameEntry::DeclaredProcedure(body) = entry.as_ref() else {
            panic!("independent lexical owner")
        };
        assert_eq!(body.parameters[0].name, "arg");
        assert_eq!(
            body.declaration.offset,
            u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        );
        assert_eq!(
            body.source.try_text().unwrap(),
            "upvar 1 other local; return $local"
        );
        assert_eq!(
            layout.snapshot.state.source_variables.namespace_identity,
            Some(body.namespace.clone())
        );
        assert!(entry.owns_original_context(&layout.snapshot.state.source_variables));
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .declaration_operand_layout_advice(&changed)
                .is_none()
        );
    }

    #[test]
    fn abrupt_prefix_retains_declaration_body_geometry_without_installation() {
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let (bindings, tokens) = inventory(source, "upvar 1 other local");
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("unchanged abrupt table retains original grammar");
        assert!(
            layout.closed_lookup(),
            "the skipped rename did not change the borrowed table"
        );
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let site = binding.invocation_site().unwrap();
        assert!(
            bindings
                .conditional_body_entry_at(&site.source, site.offset)
                .is_none()
        );
        assert!(bindings.entered_scripts.keys().all(
            |parent| parent.offset != u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        ));
    }

    #[test]
    fn public_declaration_consumer_retains_original_authored_body_namespace() {
        let source = "rename $old decl\nproc wrap {} {\nupvar 1 other local\nreturn $local\n}\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let realm = crate::realm::document_realm_bindings(source, profile, &registry);
        let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "upvar 1 other local",
            offset,
            config,
        )
        .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        realm.stamp_original_tokens(&mut tokens);
        let assistance = realm
            .original_declaration_assistance(&tokens, &registry)
            .expect("same original authored namespace and lexical declaration owner");
        assert_eq!(
            assistance.declarations,
            vec![crate::registry_invocation::DeclarationArgument {
                argument: 2,
                name: "local".to_owned(),
            },]
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.proved_execution_target().is_none());
        let (_, original_words, original_offset) = binding
            .original_compiler_source(&tokens)
            .expect("conditional declaration retains only its original vector");
        assert_eq!(original_words, tokens.words());
        assert_eq!(original_offset, offset);
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn uninstalled_body_layout_stops_at_original_lookup_changes() {
        for prefix in [
            "rename upvar replacement",
            "proc upvar args {}",
            "unknown_call",
        ] {
            let source =
                format!("rename $old decl\nproc wrap {{}} {{{prefix}; upvar 1 other local}}");
            let (_, tokens) = inventory(&source, "upvar 1 other local");
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&tokens)
                    .is_none(),
                "{prefix}"
            );
        }
    }
}
