// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Logical procedure allocation and argument correspondence.

use crate::ir::{CommandTokens, Module, Procedure, WordExpr};
use tcl_registry::CommandRegistry;
use tcl_syntax::formal_params::FormalArgumentCountShape;

/// A conditional source call joined to its genuine original declaration.
/// Logical and authored-simulation issuers retain separate applicability;
/// this argument/count model grants no Native invocation, frame or completion.
pub(crate) struct OriginalSourceProcedureCall<'a> {
    id: crate::ir::ProcedureId,
    procedure: &'a Procedure,
    arguments: super::EffectiveCommandWords,
    count: Option<FormalArgumentCountShape>,
}

impl<'a> OriginalSourceProcedureCall<'a> {
    pub(crate) const fn id(&self) -> crate::ir::ProcedureId {
        self.id
    }

    pub(crate) const fn procedure(&self) -> &'a Procedure {
        self.procedure
    }

    /// Read the same retained effective argv, including original captured operands.
    pub(crate) const fn effective_arguments(&self) -> &super::EffectiveCommandWords {
        &self.arguments
    }

    pub(crate) fn argument_count(&self) -> Option<usize> {
        if self
            .arguments
            .words
            .iter()
            .any(|word| matches!(word, WordExpr::Expand { .. }))
        {
            return None;
        }
        self.arguments.words.len().checked_sub(1)
    }

    pub(crate) fn accepts_arguments(&self) -> bool {
        self.argument_count()
            .is_some_and(|count| self.count.is_some_and(|shape| shape.accepts(count)))
    }
}

/// Match all conditional target allocations against the retained Module's
/// exact original header/body inventory. Neither reported names nor a later
/// equal source offset select a declaration. Captured operands use the shared
/// post-binding argv owner; their values are not re-evaluated at this call.
pub(crate) fn original_logical_procedure_calls_for_module<'a>(
    tokens: &CommandTokens,
    module: &'a Module,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalSourceProcedureCall<'a>>> {
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    if binding.original_invocation_source_purpose(tokens, metadata, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::Logical
    {
        return None;
    }
    let targets = binding
        .original_logical_procedure_call_targets(tokens, metadata.source_analysis_input()?)?;
    if targets.is_empty() {
        return None;
    }
    original_procedure_calls_for_targets(tokens, module, &targets, |procedure| {
        original_procedure_formal_count_shape(module, procedure, registry)
    })
}

/// A source calling template under an explicitly authored naming simulation.
/// It never selects Native dispatch, compiler entry, a frame or completion.
pub(crate) fn original_authored_procedure_calls_for_module<'a>(
    tokens: &CommandTokens,
    module: &'a Module,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalSourceProcedureCall<'a>>> {
    use tcl_syntax::naming::NamePolicyAuthority;
    if module.source_entry.native_entry.is_some()
        || module.source_entry.hosted_execution_context.is_some()
        || module
            .source_entry
            .options()
            .execution_name_policy()?
            .native_recipe()?
            .authority()
            != NamePolicyAuthority::AuthoredSimulation
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let advice = binding.declaration_call_layout_advice(tokens)?;
    if !advice.source_targets_are_closed()
        || advice.targets().is_empty()
        || advice.targets().iter().any(|target| {
            target.registry_backed
                || target.kind != crate::command_binding::BindingKind::Proc
                || target.implementation_allocation.is_none()
        })
    {
        return None;
    }
    original_procedure_calls_for_targets(tokens, module, advice.targets(), |procedure| {
        let arguments =
            crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots::from_module(
                module, procedure,
            )?;
        Some(tcl_syntax::formal_params::formal_argument_count_shape(
            arguments.arguments().names().iter().map(|_| (false, false)),
            module.parameter_grammar()?,
        ))
    })
}

/// Possible body identities from the original invocation's checked source lookup.
/// This supplies source reachability only, never dispatch, argument or Native grants.
pub(crate) fn original_source_procedure_ids_for_word_for_module(
    tokens: &CommandTokens,
    word: &str,
    module: &Module,
    registry: &CommandRegistry,
) -> Option<Vec<crate::ir::ProcedureId>> {
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let lookup = binding.lookup_command_word(word);
    if lookup.unknown || lookup.may_be_absent || lookup.targets.is_empty() {
        return None;
    }
    let mut ids = Vec::new();
    for target in &lookup.targets {
        let (id, procedure) = module.original_source_procedure_for_target(target)?;
        if !module
            .retained_source_bindings
            .as_ref()?
            .matches_original_procedure(procedure)
        {
            return None;
        }
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Some(ids)
}

/// Source-selected body calls with the same original captured argv; no Native grant.
pub(crate) fn original_source_procedure_calls_for_word_for_module<'a>(
    tokens: &CommandTokens,
    word: &str,
    module: &'a Module,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalSourceProcedureCall<'a>>> {
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let lookup = binding.lookup_command_word(word);
    if lookup.unknown || lookup.may_be_absent || lookup.targets.is_empty() {
        return None;
    }
    original_procedure_calls_for_targets(tokens, module, &lookup.targets, |procedure| {
        original_procedure_formal_count_shape(module, procedure, registry)
    })
}

fn original_procedure_calls_for_targets<'a>(
    tokens: &CommandTokens,
    module: &'a Module,
    targets: &[crate::command_binding::SourceCommandTarget],
    count_for_procedure: impl Fn(&Procedure) -> Option<FormalArgumentCountShape>,
) -> Option<Vec<OriginalSourceProcedureCall<'a>>> {
    targets
        .iter()
        .map(|target| {
            let (id, procedure) = module.original_source_procedure_for_target(target)?;
            if !module
                .retained_source_bindings
                .as_ref()?
                .matches_original_procedure(procedure)
            {
                return None;
            }
            let count = count_for_procedure(procedure);
            let arguments = super::effective_words_for_target(tokens, target)?;
            Some(OriginalSourceProcedureCall {
                id,
                procedure,
                arguments,
                count,
            })
        })
        .collect()
}

/// Descriptive formal count shape from the same original declaration and
/// independently selected parameter/list grammar. This supplies no argument
/// values, local cells, activation or successful procedure completion.
pub(crate) fn original_procedure_formal_count_shape(
    module: &Module,
    procedure: &Procedure,
    registry: &CommandRegistry,
) -> Option<FormalArgumentCountShape> {
    let (parameters, grammar) = original_procedure_parameters(module, procedure, registry)?;
    Some(tcl_syntax::formal_params::formal_argument_count_shape(
        parameters
            .iter()
            .map(|parameter| (parameter.name == "args", parameter.default.is_some())),
        grammar,
    ))
}

/// Names bound as direct local scalars under every accepted formal branch of
/// the conditional Logical model. The selected binding planner retains Jim
/// defaults/rest names and caller links; links and nonlocal/element destinations
/// cannot borrow this scalar no-error model. Native activation stays separate.
pub(crate) fn original_procedure_scalar_bindings(
    module: &Module,
    procedure: &Procedure,
    registry: &CommandRegistry,
) -> Option<std::collections::HashSet<String>> {
    use tcl_syntax::formal_params::FormalArgumentBinding as Binding;
    let (parameters, grammar) = original_procedure_parameters(module, procedure, registry)?;
    let shape = tcl_syntax::formal_params::formal_argument_count_shape(
        parameters
            .iter()
            .map(|parameter| (parameter.name == "args", parameter.default.is_some())),
        grammar,
    );
    let mut common: Option<std::collections::HashSet<String>> = None;
    // The planner's branches depend on the supplied fixed positions. One extra
    // position covers the variadic branch; further surplus changes only its list
    // contents, which this source name-purpose query never reads.
    for count in 0..=parameters.len().checked_add(1)? {
        if !shape.accepts(count) {
            continue;
        }
        let mut names = std::collections::HashSet::new();
        for binding in
            tcl_syntax::formal_params::bind_formal_arguments(&parameters, count, grammar).ok()?
        {
            let name = match binding {
                Binding::Value { parameter, .. } | Binding::Default { parameter } => {
                    parameters.get(parameter)?.name.clone()
                }
                Binding::Rest { name, .. } => name,
                Binding::CallerLink { .. } => return None,
            };
            if name.is_empty()
                || tcl_syntax::naming::is_qualified(name.as_bytes())
                || tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_some()
            {
                return None;
            }
            names.insert(name);
        }
        if let Some(common) = &mut common {
            common.retain(|name| names.contains(name));
        } else {
            common = Some(names);
        }
    }
    common
}

pub(crate) fn original_procedure_parameters(
    module: &Module,
    procedure: &Procedure,
    registry: &CommandRegistry,
) -> Option<(
    Vec<tcl_syntax::formal_params::FormalParameter>,
    tcl_dialect::ParameterGrammar,
)> {
    let retained = module.retained_source_bindings.as_ref()?;
    if !retained.matches_module(module, registry)
        || !retained.matches_original_procedure(procedure)
        || module.original_invocation_source_purpose(
            registry,
            super::InvocationMetadataContext::for_module(registry, module)?,
        ) != Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
    {
        return None;
    }
    let grammar = module.parameter_grammar()?;
    let parameters = crate::signature_scan::params::parse_param_list_strict_in(
        &procedure.params_raw,
        module.word_values(),
        grammar,
    )
    .ok()?;
    if !parameters
        .iter()
        .map(|parameter| &parameter.name)
        .eq(procedure.params.iter())
    {
        return None;
    }
    Some((parameters, grammar))
}

/// Possible original body-summary dependencies, separate from a selected call.
/// Open alternatives cannot supply a return value, argument binder or call edge.
pub(crate) struct OriginalProcedureMayDependencies {
    pub(crate) bodies: Vec<crate::ir::ProcedureId>,
    pub(crate) unknown_alternatives: bool,
}

/// Join original call geometry to genuine declaration cards for MAY consumers.
/// Implementation-backed selection keeps its stronger existing owner; Native
/// source cards retain other handler/alias possibilities independently.
pub(crate) fn original_procedure_may_dependencies_for_word_for_module(
    tokens: &CommandTokens,
    word: &str,
    module: &Module,
    registry: &CommandRegistry,
) -> Option<OriginalProcedureMayDependencies> {
    if let Some(bodies) =
        original_source_procedure_ids_for_word_for_module(tokens, word, module, registry)
    {
        return Some(OriginalProcedureMayDependencies {
            bodies,
            unknown_alternatives: false,
        });
    }
    let binding = tokens.source_binding.as_ref()?;
    if binding.original_native_source_may_owner(tokens, module, registry) {
        return Some(OriginalProcedureMayDependencies {
            bodies: module
                .original_source_procedure_bodies()
                .map(|(id, _)| *id)
                .collect(),
            unknown_alternatives: true,
        });
    }
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let input = metadata.source_analysis_input()?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || module.source_metadata_input.as_ref() != Some(input)
    {
        return None;
    }
    let lookup = binding.original_static_head_lookup_for_tokens(tokens)?;
    if lookup.name_input().bytes() != word.as_bytes()
        || lookup.site().source.source_image() != &module.source
    {
        return None;
    }
    let retained = module.retained_source_bindings.as_ref()?;
    retained.positioned_for_module(module, registry)?;
    let candidates = module
        .original_native_declaration_body_units
        .iter()
        .filter_map(|(id, card)| {
            let procedure = module.body_units.get(id)?;
            (module.procedure_ids.owns(*id)
                && card.matches_procedure(module, procedure)
                && retained.matches_original_procedure(procedure))
            .then_some((card.source_slot(), card.original_name_input().policy(), *id))
        });
    let mut bodies = lookup.matching_slot_publications(candidates)?;
    bodies.sort_unstable();
    bodies.dedup();
    Some(OriginalProcedureMayDependencies {
        bodies,
        unknown_alternatives: true,
    })
}

#[cfg(test)]
mod native_may_dependency_tests {
    use super::*;

    fn fixture(source: &str) -> (Module, std::sync::Arc<tcl_registry::model::ContextRegistry>) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_resolved_analysis_input(input),
            source,
        );
        (module, context)
    }

    #[test]
    fn native_declaration_dependencies_keep_quoted_braced_heads_and_open_alternatives() {
        for head in ["f", "{f}", "\"f\""] {
            let source = format!("proc f {{x}} {{return $x}}\nproc caller {{x}} {{{head} $x}}\n");
            let (module, context) = fixture(&source);
            let retained = module.retained_source_bindings.as_ref().unwrap();
            let bindings = retained
                .positioned_for_module(&module, context.commands())
                .unwrap();
            let offset = u32::try_from(source.rfind(head).unwrap()).unwrap();
            let binding = bindings.invocation_at_source("", offset);
            let (_, mut tokens) = binding.original_recorded_command().unwrap();
            tokens.source_binding = Some(binding);
            let selected = original_procedure_may_dependencies_for_word_for_module(
                &tokens,
                "f",
                &module,
                context.commands(),
            )
            .unwrap();
            assert!(selected.unknown_alternatives);
            let (id, card) = module
                .original_native_declaration_body_units
                .iter()
                .find(|(_, card)| card.name() == Some("f"))
                .unwrap();
            assert_eq!(selected.bodies, vec![*id]);
            assert!(card.matches_procedure(&module, module.body_units.get(id).unwrap()));
            assert!(
                original_source_procedure_ids_for_word_for_module(
                    &tokens,
                    "f",
                    &module,
                    context.commands()
                )
                .is_none()
            );
            assert!(
                original_source_procedure_calls_for_word_for_module(
                    &tokens,
                    "f",
                    &module,
                    context.commands()
                )
                .is_none()
            );
            let mut changed = tokens.clone();
            changed.source_binding = None;
            assert!(
                original_procedure_may_dependencies_for_word_for_module(
                    &changed,
                    "f",
                    &module,
                    context.commands()
                )
                .is_none()
            );
            assert!(
                original_procedure_may_dependencies_for_word_for_module(
                    &tokens,
                    "other",
                    &module,
                    context.commands()
                )
                .is_none()
            );
            let mut foreign = module.clone();
            foreign.source_metadata_input = None;
            assert!(
                original_procedure_may_dependencies_for_word_for_module(
                    &tokens,
                    "f",
                    &foreign,
                    context.commands()
                )
                .is_none()
            );
        }
    }

    #[test]
    fn native_declaration_dependencies_preserve_repeated_headers_and_unrepresented_aliases() {
        let source = "proc f {} {return FIRST}\nproc f {} {return SECOND}\nproc caller {} {f}\nproc unknown {} {alias}\n";
        let (module, context) = fixture(source);
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let bindings = retained
            .positioned_for_module(&module, context.commands())
            .unwrap();
        let queried = |word: &str| {
            let offset = u32::try_from(source.rfind(word).unwrap()).unwrap();
            let binding = bindings.invocation_at_source("", offset);
            let (_, mut tokens) = binding.original_recorded_command().unwrap();
            tokens.source_binding = Some(binding);
            original_procedure_may_dependencies_for_word_for_module(
                &tokens,
                word,
                &module,
                context.commands(),
            )
            .unwrap()
        };
        let possible = queried("f");
        assert!(possible.unknown_alternatives);
        let mut expected = module
            .original_native_declaration_body_units
            .iter()
            .filter_map(|(id, card)| (card.name() == Some("f")).then_some(*id))
            .collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(possible.bodies, expected);
        assert_eq!(possible.bodies.len(), 2);
        let unavailable = queried("alias");
        assert!(unavailable.bodies.is_empty());
        assert!(unavailable.unknown_alternatives);
    }
}
