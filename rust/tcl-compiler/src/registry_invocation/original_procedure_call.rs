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
    procedure: &'a Procedure,
    arguments: super::EffectiveCommandWords,
    count: Option<FormalArgumentCountShape>,
}

impl<'a> OriginalSourceProcedureCall<'a> {
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
    if !metadata.permits_logical_source_names() {
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

fn original_procedure_calls_for_targets<'a>(
    tokens: &CommandTokens,
    module: &'a Module,
    targets: &[crate::command_binding::SourceCommandTarget],
    count_for_procedure: impl Fn(&Procedure) -> Option<FormalArgumentCountShape>,
) -> Option<Vec<OriginalSourceProcedureCall<'a>>> {
    targets
        .iter()
        .map(|target| {
            let mut matching = module.procedures.values().filter(|procedure| {
                target.matches_authored_implementation_image(&module.source, procedure.span.start())
            });
            let procedure = matching.next()?;
            if matching.next().is_some() {
                return None;
            }
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
        || !super::InvocationMetadataContext::for_module(registry, module)?
            .permits_logical_source_names()
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
