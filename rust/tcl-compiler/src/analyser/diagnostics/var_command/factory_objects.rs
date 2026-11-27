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

//! The object types constructors and factory procedures return.

use super::FactoryObjectAdvice;
use crate::analyser::state::Analyser;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;
use tcl_core_types::BodyFrameId;

impl Analyser {
    /// Nominal object results are assistance. Actual lookup identity and
    /// catalogue applicability come from the shared point owner.
    fn nominal_object_result(
        &self,
        word: &tcl_source::words::WordExpr,
        parent: Option<&crate::ir::CommandTokens>,
        registry: &tcl_registry::CommandRegistry,
        module: &crate::ir::Module,
    ) -> (bool, Option<BodyFrameId>) {
        let Some(commands) =
            crate::value_shapes::command_substitution_tokens(word, parent, self.lexer_config())
        else {
            return (false, None);
        };
        let Some(tokens) = commands.last() else {
            return (false, None);
        };
        self.nominal_tokens_object_result(tokens, registry, module)
    }

    pub(super) fn nominal_tokens_object_result(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        module: &crate::ir::Module,
    ) -> (bool, Option<BodyFrameId>) {
        use crate::registry_invocation::registry_invocation_assistance_in_context;
        let context = self.analysis_context();
        let object_metadata = registry_invocation_assistance_in_context(&context, tokens)
            .is_some_and(|assistance| {
                assistance
                    .candidates
                    .iter()
                    .any(|shape| shape.nominal_return_type == Some(tcl_registry::TclType::Object))
            });
        let Some(binding) = &tokens.source_binding else {
            return (object_metadata, None);
        };
        let dispatcher_name = binding.nominal_definition_name_result(registry).is_some();
        let object_class = !binding.class_factory_candidates(registry).is_empty();
        let callee = {
            let candidates = binding.declared_procedure_result_candidates(tokens);
            candidates
                .first()
                .filter(|first| {
                    candidates.iter().all(|candidate| {
                        candidate.implementation_allocation == first.implementation_allocation
                            && candidate.command == first.command
                    })
                })
                .and_then(|target| module.source_procedure_for_target(target))
                .map(|(id, _)| BodyFrameId::Body(id))
        }
        .or_else(|| {
            let candidates = binding.declared_self_method_candidates(registry);
            let first = candidates.first()?;
            candidates
                .iter()
                .all(|entry| entry == first)
                .then_some(())?;
            module
                .methods
                .for_original_entry(first)
                .map(|(id, _)| BodyFrameId::Body(id))
        });
        (object_metadata || object_class || dispatcher_name, callee)
    }

    pub(super) fn compute_factory_object_ranges(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> FactoryObjectAdvice {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Possible result advice uses the same source and actual availability
        // generation as the supplied unit, without a completed result grant.
        let Some(input) = self.result.resolved_input.as_ref() else {
            return FactoryObjectAdvice::default();
        };
        if crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module)
            .is_none()
        {
            return FactoryObjectAdvice::default();
        }
        if cu.ir_module.source_metadata_input.as_ref() != Some(input)
            || *cu.source != *self.source
            || !self.result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                input.lexer_config(),
            )
        {
            return FactoryObjectAdvice::default();
        }
        let top_scope = BodyFrameId::Top(cu.ir_module.procedure_ids.module_id());
        let units: Vec<(BodyFrameId, &crate::compilation_unit::FunctionUnit)> =
            std::iter::once((top_scope, &cu.top_level))
                .chain(
                    cu.procedures
                        .iter()
                        .map(|(id, function)| (BodyFrameId::Body(*id), function)),
                )
                .chain(
                    cu.methods
                        .iter()
                        .map(|(id, function)| (BodyFrameId::Body(*id), function)),
                )
                .chain(
                    cu.body_lowering_functions()
                        .map(|(id, function)| (BodyFrameId::Body(*id), function)),
                )
                .collect();

        // Per-proc: factory-local vars (non-user-proc factory heads), the last
        // returned var, and the `{var -> rhs command head}` assignment map.
        let mut maps = FactoryMaps::default();
        for (qname, fu) in &units {
            seed_factory_maps(
                *qname,
                fu,
                &cu.ir_module,
                &|word, parent| self.nominal_object_result(word, parent, registry, &cu.ir_module),
                &mut maps,
                self.lexer_config(),
            );
        }
        let FactoryMaps {
            mut factory_locals,
            assigns,
            return_var,
            mut object_returning,
            returned_callees,
        } = maps;
        // A proc returning one of its own factory locals is object-returning.
        for (qname, rv) in &return_var {
            if let Some(rv) = rv
                && factory_locals.get(qname).is_some_and(|s| s.contains(rv))
            {
                object_returning.insert(*qname);
            }
        }

        propagate_object_returning(
            &return_var,
            &returned_callees,
            &assigns,
            &mut object_returning,
            &mut factory_locals,
        );

        let mut ranges = Vec::new();
        for (owner, names) in factory_locals {
            if names.is_empty() {
                continue;
            }
            match owner {
                BodyFrameId::Top(module) if module == cu.ir_module.procedure_ids.module_id() => {
                    ranges.push((owner, 0, u32::MAX, names));
                }
                BodyFrameId::Body(id) => {
                    let span = cu
                        .ir_module
                        .procedures
                        .get(&id)
                        .map(|body| body.span)
                        .or_else(|| cu.ir_module.methods.get(&id).and_then(|body| body.span))
                        .or_else(|| cu.ir_module.body_lowering(&id).map(|body| body.span));
                    if let Some(span) = span {
                        ranges.push((owner, span.start(), span.end(), names));
                    }
                }
                BodyFrameId::Top(_) => {}
            }
        }

        FactoryObjectAdvice {
            ranges,
            object_returning,
        }
    }
}

/// Retained scalar source syntax used by conditional return advice.
pub(super) fn returned_scalar_reference_name(
    word: &tcl_source::words::WordExpr,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let (reference, source) = word.sole_variable_substitution()?;
    (source.provenance == tcl_source::words::Provenance::Source).then_some(())?;
    tcl_syntax::naming::whole_scalar_variable_reference_name(reference, config).map(str::to_owned)
}

#[derive(Default)]
struct FactoryMaps {
    factory_locals: FxHashMap<BodyFrameId, HashSet<String>>,
    assigns: FxHashMap<BodyFrameId, FxHashMap<String, BodyFrameId>>,
    return_var: FxHashMap<BodyFrameId, Option<String>>,
    object_returning: FxHashSet<BodyFrameId>,
    returned_callees: FxHashMap<BodyFrameId, FxHashSet<BodyFrameId>>,
}

/// Conditional original return syntax supplies no completion or returned value.
pub(super) fn original_source_return_operand<'a>(
    module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    tokens: &'a crate::ir::CommandTokens,
) -> Option<&'a tcl_source::words::WordExpr> {
    let source = crate::registry_invocation::source_structure::original_registry_words_for_module(
        module, registry, tokens,
    )?;
    let input =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?
            .source_analysis_input()?;
    if source.arguments().len() != 1
        || source.with_source_schema(input.borrowed_context_registry(), |schema| {
            schema.facts().operation
                == tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::Return,
                )
        }) != Some(true)
    {
        return None;
    }
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        source.origins().get(1)?
    else {
        return None;
    };
    tokens.words().get(*written)
}

/// Retain possible assignments and unanimous returned scalar names for one
/// actual body. Source return advice remains independent of completion and
/// dispatch closure.
struct FactoryBodyFacts {
    names: HashSet<String>,
    assigns: FxHashMap<String, BodyFrameId>,
    returns: Vec<bool>,
    returned_variables: Vec<Option<String>>,
}

fn collect_factory_body_facts(
    qname: BodyFrameId,
    fu: &crate::compilation_unit::FunctionUnit,
    module: &crate::ir::Module,
    context: &tcl_registry::model::ContextRegistry,
    nominal_result: &impl Fn(
        &tcl_source::words::WordExpr,
        Option<&crate::ir::CommandTokens>,
    ) -> (bool, Option<BodyFrameId>),
    returned_callees: &mut FxHashMap<BodyFrameId, FxHashSet<BodyFrameId>>,
    config: tcl_lexer::LexerConfig,
) -> FactoryBodyFacts {
    let mut names = HashSet::new();
    let mut amap = FxHashMap::default();
    let mut returns = Vec::new();
    let mut returned_variables = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            let assignments = match stmt {
                crate::ir::Statement::AssignValue {
                    name,
                    value,
                    tokens,
                    ..
                } => tokens
                    .as_ref()
                    .and_then(|tokens| tokens.words().get(2))
                    .cloned()
                    .or_else(|| crate::value_shapes::value_word_with_config(value, config))
                    .map_or_default(|value| {
                        vec![crate::registry_invocation::AdvisoryValueAssignment {
                            name: name.clone(),
                            value,
                        }]
                    }),
                _ => stmt.tokens().map_or_default(|tokens| {
                    crate::registry_invocation::advisory_value_assignments_in_context(
                        context, tokens,
                    )
                }),
            };
            for word in stmt.tokens().into_iter().flat_map(|tokens| {
                original_source_return_operand(module, context.commands(), tokens)
                    .cloned()
                    .into_iter()
                    .chain(
                        crate::registry_invocation::advisory_return_values_in_context(
                            context, tokens,
                        ),
                    )
            }) {
                returned_variables.push(returned_scalar_reference_name(
                    &word,
                    fu.source_lexer_config(),
                ));
                let (object, callee) = nominal_result(&word, stmt.tokens());
                returns.push(object);
                if let Some(callee) = callee {
                    returned_callees.entry(qname).or_default().insert(callee);
                }
            }
            for assignment in assignments {
                let (object, callee) = nominal_result(&assignment.value, stmt.tokens());
                if object {
                    names.insert(assignment.name.clone());
                }
                if let Some(callee) = callee {
                    amap.insert(assignment.name, callee);
                }
            }
        }
        if let Some(crate::cfg::Terminator::Return {
            value_word, tokens, ..
        }) = &block.terminator
        {
            if let Some(word) = value_word {
                returned_variables.push(returned_scalar_reference_name(
                    word,
                    fu.source_lexer_config(),
                ));
            }
            let (object, callee) = value_word.as_ref().map_or((false, None), |word| {
                nominal_result(word, tokens.as_deref())
            });
            returns.push(object);
            if let Some(callee) = callee {
                returned_callees.entry(qname).or_default().insert(callee);
            }
        } else if block.terminator.is_none() {
            returns.push(false);
        }
    }
    FactoryBodyFacts {
        names,
        assigns: amap,
        returns,
        returned_variables,
    }
}

fn seed_factory_maps(
    qname: BodyFrameId,
    fu: &crate::compilation_unit::FunctionUnit,
    module: &crate::ir::Module,
    nominal_result: &impl Fn(
        &tcl_source::words::WordExpr,
        Option<&crate::ir::CommandTokens>,
    ) -> (bool, Option<BodyFrameId>),
    maps: &mut FactoryMaps,
    config: tcl_lexer::LexerConfig,
) {
    let Some(input) = module.source_metadata_input.as_ref() else {
        return;
    };
    let context = input.borrowed_context_registry();
    let FactoryBodyFacts {
        names,
        assigns: amap,
        returns,
        returned_variables,
    } = collect_factory_body_facts(
        qname,
        fu,
        module,
        context,
        nominal_result,
        &mut maps.returned_callees,
        config,
    );
    maps.factory_locals.insert(qname, names);
    maps.assigns.insert(qname, amap);
    let advisory_variable = returned_variables
        .first()
        .cloned()
        .flatten()
        .filter(|first| {
            returned_variables
                .iter()
                .all(|candidate| candidate.as_ref() == Some(first))
        });
    maps.return_var.insert(qname, advisory_variable);
    if returns.iter().any(|value| *value) {
        maps.object_returning.insert(qname);
    }
}

/// Propagate possible object returns through actual body-ID edges and each
/// body's own returned-variable assignment map.
fn propagate_object_returning(
    return_var: &FxHashMap<BodyFrameId, Option<String>>,
    returned_callees: &FxHashMap<BodyFrameId, FxHashSet<BodyFrameId>>,
    assigns: &FxHashMap<BodyFrameId, FxHashMap<String, BodyFrameId>>,
    object_returning: &mut FxHashSet<BodyFrameId>,
    factory_locals: &mut FxHashMap<BodyFrameId, HashSet<String>>,
) {
    loop {
        let mut changed = false;
        for (owner, callees) in returned_callees {
            if callees
                .iter()
                .any(|callee| object_returning.contains(callee))
            {
                changed |= object_returning.insert(*owner);
            }
        }
        for (owner, returned) in return_var {
            if let Some(callee) = returned
                .as_ref()
                .and_then(|name| assigns.get(owner)?.get(name))
                && object_returning.contains(callee)
            {
                changed |= object_returning.insert(*owner);
            }
        }
        if !changed {
            break;
        }
    }
    for (owner, assignments) in assigns {
        let names = assignments
            .iter()
            .filter(|(_, callee)| object_returning.contains(callee))
            .map(|(name, _)| name.clone());
        factory_locals.entry(*owner).or_default().extend(names);
    }
}
