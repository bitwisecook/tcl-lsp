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

//! Places outside a procedure's frame: the `Name` arguments that name them, the locals linked to them and the global and namespace writes of a component.

use std::collections::{BTreeSet, HashMap};

use tcl_registry::frame_effect::FrameLevel;
use tcl_registry::value_transfer::{BindingKind, ExistenceOutcome};
use tcl_syntax::formal_params::FormalArgumentBinding;

use super::parameter_roles::{linked_cleanly, links_of};
use super::source_context::SourceSummaryContext;
use super::{ModuleInputs, ModuleProcedures, OuterSourcePlace, ParamRole};
use crate::cfg::Function as CfgFunction;
use crate::ir::{CommandTokens, Statement};
use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;

impl ModuleProcedures<'_> {
    /// The `Name` arguments of `qname`'s calls that name a place outside its
    /// frame ([`OuterArgument`]). `None` where one names a place the
    /// summary cannot name — a local linked within the frame, to an object's
    /// or a connection's variable, or to more than one place — so the
    /// procedure has no summary.
    pub(super) fn outer_arguments(
        &self,
        qname: &crate::ir::ProcedureId,
        inputs: &ModuleInputs<'_>,
    ) -> Option<Vec<OuterArgument>> {
        let cfg = inputs.summary_cfg(qname)?;
        let source = self.source.as_ref()?;
        let linked = linked_locals(cfg, qname, source)?;
        let mut found = Vec::new();
        let mut blocks: Vec<_> = cfg.blocks.iter().collect();
        blocks.sort_unstable_by_key(|(id, _)| **id);
        for (&id, block) in blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                if statement.synthetic_marker().is_some() {
                    continue;
                }
                let parent = source.statement_tokens(Some(*qname), statement)?;
                let site = (id == cfg.entry).then_some((id, index));
                self.outer_call_arguments(
                    OuterCall {
                        tokens: &parent,
                        site,
                    },
                    &linked,
                    inputs,
                    &mut found,
                )?;
                for child in source.children(&parent)? {
                    self.outer_call_arguments(
                        OuterCall {
                            tokens: &child,
                            site: None,
                        },
                        &linked,
                        inputs,
                        &mut found,
                    )?;
                }
            }
        }
        Some(found)
    }

    /// [`Self::outer_arguments`] for one genuine original call.
    fn outer_call_arguments(
        &self,
        call: OuterCall<'_>,
        linked: &HashMap<tcl_core_types::NameBytes, LinkedLocal>,
        inputs: &ModuleInputs<'_>,
        found: &mut Vec<OuterArgument>,
    ) -> Option<()> {
        let source = self.source.as_ref()?;
        let Some(selected) = source.procedure_call(call.tokens) else {
            source.operation(call.tokens)?;
            return Some(());
        };
        let callee = &selected.id;
        let declared = source.procedure(callee)?;
        let words = selected.literal_arguments(inputs.ir)?;
        let (formals, grammar) = self.formals(callee)?;
        let shape = tcl_syntax::formal_params::formal_argument_count_shape(
            formals.iter().map(|formal| {
                (
                    tcl_syntax::formal_params::is_variadic_formal_name(&formal.name),
                    formal.default.is_some(),
                )
            }),
            grammar,
        );
        if !shape.accepts(words.len()) {
            return Some(());
        }
        let bindings =
            tcl_syntax::formal_params::bind_formal_arguments(&formals, words.len(), grammar)
                .ok()?;
        for (param, local) in links_of(callee, declared, &self.frame_facts) {
            let binding = bindings.iter().find(|binding| match binding {
                FormalArgumentBinding::Value { parameter, .. }
                | FormalArgumentBinding::Default { parameter }
                | FormalArgumentBinding::Rest { parameter, .. }
                | FormalArgumentBinding::CallerLink { parameter, .. } => *parameter == param,
            })?;
            let argument = match binding {
                FormalArgumentBinding::Value { argument, .. } => {
                    if words.get(*argument)?.is_none() {
                        continue;
                    }
                    selected.arguments.written_argument(*argument)?
                }
                // Defaults need their own retained formal-value source owner.
                FormalArgumentBinding::Default { .. }
                | FormalArgumentBinding::Rest { .. }
                | FormalArgumentBinding::CallerLink { .. } => return None,
            };
            let (cfg, ssa) = self.flow_of(callee)?;
            if !linked_cleanly(cfg, &ssa, &local, &formals.get(param)?.name, callee, source) {
                return None;
            }
            let mut receiver = None;
            for statement in &cfg.blocks.get(&cfg.entry)?.statements {
                if links_local(statement, &local, callee, source)? {
                    let original = source.statement_tokens(Some(*callee), statement)?;
                    let selected = source.alias_receiver_for_local(&original, &local)?;
                    if receiver.replace(selected).is_some() {
                        return None;
                    }
                }
            }
            let place = source.argument_source_place(call.tokens, argument, receiver?)?;
            let (places, unconditional) = match place.symbol().slot() {
                SignatureSourceVariableSlot::Local { simple, .. } => match linked.get(simple) {
                    None | Some(LinkedLocal::Caller) => continue,
                    Some(LinkedLocal::Unnameable) => return None,
                    Some(LinkedLocal::Outer(place, link)) => (
                        vec![place.as_ref().clone()],
                        call.site.is_some_and(|(block, index)| {
                            link.is_some_and(|link| link.0 == block && link.1 < index)
                        }),
                    ),
                },
                SignatureSourceVariableSlot::C { .. } | SignatureSourceVariableSlot::Jim(_) => {
                    (vec![place], call.site.is_some())
                }
            };
            found.push(OuterArgument {
                callee: *callee,
                param,
                places,
                unconditional,
            });
        }
        Some(())
    }

    /// The global and namespace places a set of procedures may write: those
    /// its bodies write, those its callees' summaries state, and those a
    /// callee's `Name` parameter names through a qualified argument or a
    /// local linked to a namespace's variable ([`Self::outer_arguments`]).
    /// A place one call names, which every normal completion makes and
    /// nothing else in the set writes, takes the callee's outcomes on it;
    /// every other is may-bound afterwards, and may-unbound too where a body
    /// the set runs destroys a variable or a callee may unbind such a place.
    pub(super) fn component_outer_writes(
        &self,
        component: &[crate::ir::ProcedureId],
        calls: &HashMap<crate::ir::ProcedureId, BTreeSet<crate::ir::ProcedureId>>,
        inputs: &ModuleInputs<'_>,
    ) -> Option<Vec<(OuterSourcePlace, ExistenceOutcome)>> {
        let mut names: Vec<OuterSourcePlace> = Vec::new();
        let mut precise: Vec<(OuterSourcePlace, Vec<ExistenceOutcome>)> = Vec::new();
        let mut destroys = false;
        let summaries = self.summaries.borrow();
        let source = self.source.as_ref()?;
        for qname in component {
            let facts = self.outer_facts.get(qname)?;
            let writes = match facts.original_source_writes() {
                Some(writes) => writes.to_vec(),
                None if !facts.opaque_global_frame => source.authored_outer_writes(qname)?,
                None => return None,
            };
            for place in writes {
                insert_outer_source(&mut names, place);
            }
            destroys |= destroys_a_variable(source, qname)?;
            for callee in &calls[qname] {
                let Some(summary) = summaries.get(callee) else {
                    if component.contains(callee) {
                        continue;
                    }
                    return None;
                };
                destroys |= summary
                    .global_sources
                    .iter()
                    .any(|(_, outcome)| *outcome == ExistenceOutcome::Unbind);
                for (place, _) in &summary.global_sources {
                    insert_outer_source(&mut names, place.clone());
                }
            }
            for argument in self.outer_arguments(qname, inputs)? {
                let outcomes = summaries.get(&argument.callee).and_then(|summary| {
                    match summary.params.get(argument.param) {
                        Some(ParamRole::Name { outcomes, .. }) => Some(outcomes.clone()),
                        _ => None,
                    }
                });
                match (argument.unconditional, argument.places.as_slice(), outcomes) {
                    (true, [place], Some(outcomes)) if component.len() == 1 => {
                        precise.push((place.clone(), outcomes));
                    }
                    (_, _, outcomes) => {
                        destroys |= outcomes
                            .is_none_or(|outcomes| outcomes.contains(&ExistenceOutcome::Unbind));
                        for place in argument.places {
                            insert_outer_source(&mut names, place);
                        }
                    }
                }
            }
        }
        let shared: Vec<bool> = precise
            .iter()
            .map(|(place, _)| {
                names.iter().any(|other| other.symbol() == place.symbol())
                    || precise
                        .iter()
                        .filter(|(other, _)| other.symbol() == place.symbol())
                        .count()
                        > 1
            })
            .collect();
        destroys |= precise
            .iter()
            .zip(&shared)
            .any(|((_, outcomes), &shared)| shared && outcomes.contains(&ExistenceOutcome::Unbind));
        let mut places = Vec::new();
        for ((place, outcomes), shared) in precise.into_iter().zip(shared) {
            if destroys || shared {
                insert_outer_source(&mut names, place);
            } else {
                places.extend(outcomes.into_iter().map(|outcome| (place.clone(), outcome)));
            }
        }
        for place in names {
            if destroys {
                places.push((place.clone(), ExistenceOutcome::Unbind));
            }
            places.push((place, ExistenceOutcome::MayBind(BindingKind::Either)));
        }
        Some(places)
    }
}

/// One `Name` argument of a call a procedure makes that names a place
/// outside the procedure's frame: a qualified name, or a local the
/// procedure links to a namespace's variable.
#[derive(Debug, Clone)]
pub(super) struct OuterArgument {
    /// The exact retained callee body.
    callee: crate::ir::ProcedureId,
    /// The `Name` parameter's index.
    param: usize,
    /// The places the argument may name: one, or for a relative name its
    /// namespace's and the global namespace's.
    places: Vec<OuterSourcePlace>,
    /// Whether every normal completion of the procedure makes the call with
    /// the argument naming that place: a statement of the entry block, after
    /// the one link the argument reads.
    unconditional: bool,
}

/// A call [`ModuleProcedures::outer_arguments`] reads: its head, each
/// argument's literal value (`None` for one that substitutes), and its
/// `(block, index)` where it is a statement of the entry block.
#[derive(Clone, Copy)]
struct OuterCall<'c> {
    pub(super) tokens: &'c CommandTokens,
    pub(super) site: Option<(crate::cfg::BlockId, usize)>,
}

/// Where a local a procedure links outside its frame lives.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LinkedLocal {
    /// A namespace's variable, by qualified name, linked by the statement at
    /// `(block, index)`.
    Outer(Box<OuterSourcePlace>, Option<(crate::cfg::BlockId, usize)>),
    /// The place one frame up a `Name` parameter names, which the
    /// parameter's role states.
    Caller,
    /// A place the summary cannot name.
    Unnameable,
}

/// Each local `cfg` links outside its frame, with where it lives
/// ([`LinkedLocal`]), from the registry's alias declarations and scope-alias
/// plans; a local linked twice, to two places, is unnameable.
fn linked_locals(
    cfg: &CfgFunction,
    qname: &crate::ir::ProcedureId,
    source: &SourceSummaryContext<'_>,
) -> Option<HashMap<tcl_core_types::NameBytes, LinkedLocal>> {
    use tcl_registry::{CallerFrameSelection, VariableAliasTarget};
    let mut linked = HashMap::new();
    for tokens in source.body_commands(qname)? {
        let mut sites = Vec::new();
        for (&block, body) in &cfg.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if statement.synthetic_marker().is_none()
                    && source.statement_tokens(Some(*qname), statement).as_ref() == Some(&tokens)
                {
                    sites.push((block, index));
                }
            }
        }
        let at = (sites.len() == 1).then(|| sites[0]);
        let (aliases, dialect) = alias_facts(&tokens, source)?;
        for alias in aliases {
            let local = source.alias_local(&tokens, &alias)?;
            let place = match &alias.target {
                VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                    let level = match frame {
                        CallerFrameSelection::DefaultCaller => Some(FrameLevel::Relative(1)),
                        CallerFrameSelection::Explicit(level) => {
                            level.literal().and_then(|level| {
                                dialect.and_then(|dialect| {
                                    FrameLevel::parse_for_dialect(level, dialect)
                                })
                            })
                        }
                    };
                    match (level, variable.literal()) {
                        (Some(FrameLevel::Relative(1)), None) => LinkedLocal::Caller,
                        (Some(FrameLevel::Absolute(0)), Some(_)) => source
                            .alias_source_place(&tokens, &alias)
                            .map_or(LinkedLocal::Unnameable, |place| {
                                LinkedLocal::Outer(Box::new(place), at)
                            }),
                        _ => LinkedLocal::Unnameable,
                    }
                }
                _ => source
                    .alias_source_place(&tokens, &alias)
                    .map_or(LinkedLocal::Unnameable, |place| {
                        LinkedLocal::Outer(Box::new(place), at)
                    }),
            };
            let same = linked.get(&local).is_some_and(|held| match (held, &place) {
                (LinkedLocal::Outer(held, _), LinkedLocal::Outer(new, _)) => {
                    held.symbol() == new.symbol()
                }
                (held, new) => held == new,
            });
            if !same {
                let place = if linked.contains_key(&local) {
                    LinkedLocal::Unnameable
                } else {
                    place
                };
                linked.insert(local, place);
            }
        }
        if let Some(operation) = source.operation(&tokens) {
            let metadata = source.metadata(&tokens)?;
            let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
            let frame =
                operation.with_metadata_schema(source.registry(), metadata, realm, |schema| {
                    Some(schema.semantics.value.alias_frame())
                })?;
            if matches!(
                frame,
                Some(
                    tcl_registry::value_transfer::AliasFrame::Object
                        | tcl_registry::value_transfer::AliasFrame::Connection
                )
            ) {
                // An object/connection local is not a source namespace coordinate.
                if operation
                    .facts
                    .arg_roles
                    .iter()
                    .any(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
                {
                    return None;
                }
            }
        }
    }
    Some(linked)
}

fn insert_outer_source(places: &mut Vec<OuterSourcePlace>, place: OuterSourcePlace) {
    if !places.iter().any(|other| other.symbol() == place.symbol()) {
        places.push(place);
    }
}

/// Whether some statement of `cfg` may destroy a variable.
fn destroys_a_variable(
    source: &SourceSummaryContext<'_>,
    qname: &crate::ir::ProcedureId,
) -> Option<bool> {
    let mut destroys = false;
    for tokens in source.body_commands(qname)? {
        if source.procedure_call(&tokens).is_none() {
            destroys |= source
                .operation(&tokens)?
                .facts
                .traits
                .contains(tcl_registry::Traits::DESTROYS_VARIABLE);
        }
    }
    Some(destroys)
}

/// Alias declarations from the same selected original argv, including captures.
pub(super) fn alias_facts(
    tokens: &CommandTokens,
    source: &SourceSummaryContext<'_>,
) -> Option<(
    Vec<tcl_registry::VariableCellAliasTransition>,
    Option<tcl_native_model::InvocationDialect>,
)> {
    if source.procedure_call(tokens).is_some() {
        return Some((Vec::new(), None));
    }
    let operation = source.operation(tokens)?;
    let transitions = source.transitions(tokens)?;
    Some((
        transitions
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::VariableCellAlias(alias) => Some(alias.clone()),
                _ => None,
            })
            .collect(),
        operation.dialect,
    ))
}

/// A computed outside-cell target prevents a closed conditional summary.
pub(super) fn aliases_unnamed_place(
    tokens: &CommandTokens,
    source: &SourceSummaryContext<'_>,
) -> Option<bool> {
    use tcl_registry::{CallerFrameSelection, VariableAliasTarget};
    let (aliases, dialect) = alias_facts(tokens, source)?;
    Some(aliases.iter().any(|alias| match &alias.target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable } => variable.literal().is_none(),
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => namespace.literal().is_none() || variable.literal().is_none(),
        VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            let caller = match frame {
                CallerFrameSelection::DefaultCaller => true,
                CallerFrameSelection::Explicit(level) => level
                    .literal()
                    .and_then(|level| {
                        dialect.and_then(|dialect| FrameLevel::parse_for_dialect(level, dialect))
                    })
                    .is_some_and(|level| level == FrameLevel::Relative(1)),
            };
            !caller && variable.literal().is_none()
        }
    }))
}

pub(super) fn links_local(
    statement: &Statement,
    local: &str,
    qname: &crate::ir::ProcedureId,
    source: &SourceSummaryContext<'_>,
) -> Option<bool> {
    if statement.synthetic_marker().is_some() {
        return Some(false);
    }
    let tokens = source.statement_tokens(Some(*qname), statement)?;
    let (aliases, _) = alias_facts(&tokens, source)?;
    Some(aliases.iter().any(|alias| {
        alias.local.literal().is_some_and(|name| {
            tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_none() && name == local
        })
    }))
}
