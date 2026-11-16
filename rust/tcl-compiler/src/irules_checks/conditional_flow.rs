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

//! Source-order flow advice over a complete census of leaf-handler commands.

use super::collect_flow::find_conditional_collection_source_pairing_advice;
use super::drop_flow::{dedupe_drop_states, drop_flow_command};
use super::flow_inventory::original_irules_flow_inventory;
use super::flow_walk::{unreached_statements, walk_flow};
use super::http_flow::dedupe_flow_states;
use super::source_query::leaf_handler_source_completion;
use super::{
    CollectionSourceObligationKind, ConditionalIruleSourceFlowAdvice,
    ConditionalIruleSourceFlowFinding, ConditionalIruleSourceOwner, DropFlowState, FlowDispatch,
    HttpFlowState, IrulesCheckWarning, IrulesSourceQuery, OriginalIrulesFlowCommand,
};
use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;

/// Source-order flow advice under the same complete leaf-handler census.
/// Descriptor fallthrough is only a source premise: actual lookup, effects,
/// activation and normal continuation remain unresolved in the returned owner.
#[must_use]
pub fn find_conditional_irule_source_flow_advice<'unit>(
    cu: &'unit CompilationUnit,
    registry: &'unit CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
    find_conditional_collection_source_pairing_advice(cu, registry, dialect)?
        .conditional_source_flow()
}

/// Whether every possible source schema for the statement completes some
/// other way than falling through (`return`).
fn statement_ends_its_path(
    source: &IrulesSourceQuery<'_>,
    tokens: &crate::ir::CommandTokens,
) -> bool {
    let Some(metadata) = source.metadata() else {
        return false;
    };
    crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
        source.registry,
        Some(metadata),
        tokens,
    )
    .is_some_and(|possible| {
        !possible.candidates.is_empty()
            && possible.candidates.iter().all(|candidate| {
                candidate
                    .with_possible_source_schema(
                        source.registry,
                        metadata,
                        tokens,
                        leaf_handler_source_completion,
                    )
                    .is_some_and(|completion| {
                        completion != tcl_registry::registry::InvocationCompletion::FallsThrough
                    })
            })
    })
}

fn unguarded_drop_findings(finals: Vec<DropFlowState>) -> Vec<IrulesCheckWarning> {
    let mut warnings = Vec::new();
    for pending in finals {
        if pending.dropped
            && let Some(span) = pending.drop_at
        {
            warnings.push(
                conditional_source_flow_finding(
                    span,
                    DiagCode::Irule5002,
                    &format!(
                        "Possible '{}' has no independently established guard against later processing.",
                        pending.drop_cmd
                    ),
                )
                .warning,
            );
        }
        if pending.dns_returned
            && let Some(span) = pending.dns_at
        {
            warnings.push(
                conditional_source_flow_finding(
                    span,
                    DiagCode::Irule5004,
                    "Possible DNS packet return has no independently established guard against later processing.",
                )
                .warning,
            );
        }
    }
    warnings
}

/// One handler's findings and the lookup obligations its statements carry.
fn conditional_handler_flow(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    source: &IrulesSourceQuery<'_>,
    events: &EventRegistry,
    id: crate::ir::ProcedureId,
) -> Option<(
    Vec<IrulesCheckWarning>,
    HashSet<CollectionSourceObligationKind>,
)> {
    let mut warnings = Vec::new();
    let function = unit.procedures.get(&id)?;
    let event = source.event(function)?.into_event()?;
    let config = source.function_config(function)?;
    let unreached = unreached_statements(function);
    let http = events
        .get_props(event.body.event())
        .is_some_and(|props| props.implied_profiles.contains(&"HTTP"));
    // The walk follows the structured body path by path: an `if` or
    // `switch` arm, a `catch` body and a loop each keep their own states,
    // and a `return` ends its path. A statement's lookup obligations stay
    // on the advice instead of refusing it.
    let missing_source = std::cell::Cell::new(false);
    let obligations = std::cell::RefCell::new(HashSet::new());
    let commands = |script: &Script, statement: &Statement| {
        let Some(inventory) = original_irules_flow_inventory(script, statement, source, config)
        else {
            missing_source.set(true);
            return Vec::new();
        };
        obligations.borrow_mut().extend(inventory.obligations);
        inventory.commands
    };
    let terminates = |tokens: &crate::ir::CommandTokens| statement_ends_its_path(source, tokens);
    if http {
        let event_name = event.body.event();
        let leaf = |state: &HttpFlowState,
                    script: &Script,
                    statement: &Statement,
                    out: &mut Vec<IrulesCheckWarning>| {
            commands(script, statement)
                .iter()
                .fold(*state, |current, command| {
                    conditional_http_flow_command(current, command, event_name, out)
                })
        };
        walk_flow(
            &event.procedure.body,
            &[HttpFlowState {
                responded: false,
                respond_at: None,
            }],
            &mut FlowDispatch {
                registry,
                out: &mut warnings,
                leaf: &leaf,
                dedupe: &dedupe_flow_states,
                unreached: &unreached,
                terminates: Some(&terminates),
            },
            None,
        );
    }
    let leaf = |state: &DropFlowState,
                script: &Script,
                statement: &Statement,
                _out: &mut Vec<IrulesCheckWarning>| {
        commands(script, statement)
            .iter()
            .fold(state.clone(), |current, command| {
                drop_flow_command(&current, command)
            })
    };
    let mut unused = Vec::new();
    let finals = walk_flow(
        &event.procedure.body,
        &[DropFlowState::default()],
        &mut FlowDispatch {
            registry,
            out: &mut unused,
            leaf: &leaf,
            dedupe: &dedupe_drop_states,
            unreached: &unreached,
            terminates: Some(&terminates),
        },
        None,
    );
    if missing_source.get() {
        return None;
    }
    warnings.extend(unguarded_drop_findings(finals));
    Some((warnings, obligations.into_inner()))
}

pub(super) fn conditional_irule_source_flow_advice<'unit>(
    owner: &ConditionalIruleSourceOwner<'unit>,
) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
    let source = IrulesSourceQuery::capture(owner.unit, owner.registry)?;
    let events = EventRegistry::build();
    let mut owner = owner.clone();
    owner
        .obligations
        .insert(CollectionSourceObligationKind::NormalContinuation);
    let mut warnings = Vec::new();
    let handlers = owner
        .flow_commands
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    for id in handlers {
        let (found, obligations) =
            conditional_handler_flow(owner.unit, owner.registry, &source, &events, id)?;
        warnings.extend(found);
        owner.obligations.extend(obligations);
    }
    // Several predecessor states can reach the same command; the editor
    // receives one finding per code, range and message.
    let mut seen = HashSet::new();
    warnings.retain(|warning| {
        seen.insert((
            warning.code,
            warning.span.start(),
            warning.span.end(),
            warning.message.clone(),
        ))
    });
    Some(ConditionalIruleSourceFlowAdvice {
        owner,
        findings: warnings
            .into_iter()
            .map(|warning| ConditionalIruleSourceFlowFinding { warning })
            .collect(),
    })
}

/// Apply one command to a response-flow state under conditional advice.
fn conditional_http_flow_command(
    state: HttpFlowState,
    command: &OriginalIrulesFlowCommand,
    event: &str,
    out: &mut Vec<IrulesCheckWarning>,
) -> HttpFlowState {
    if command.may_commit_response {
        if state.responded {
            out.push(
                conditional_source_flow_finding(
                    command.span,
                    DiagCode::Irule1202,
                    &format!(
                        "Possible response operation '{}' follows another response candidate in {event}.",
                        command.command
                    ),
                )
                .warning,
            );
            return state;
        }
        return HttpFlowState {
            responded: true,
            respond_at: Some(command.span.start()),
        };
    }
    if state.responded && command.requires_http_context {
        out.push(
            conditional_source_flow_finding(
                command.span,
                DiagCode::Irule1201,
                &format!(
                    "Possible HTTP-context operation '{}' follows a response candidate in {event}.",
                    command.command
                ),
            )
            .warning,
        );
    }
    state
}

fn conditional_source_flow_finding(
    span: Span,
    code: DiagCode,
    source_hazard: &str,
) -> ConditionalIruleSourceFlowFinding {
    ConditionalIruleSourceFlowFinding {
        warning: IrulesCheckWarning {
            span,
            code,
            message: format!(
                "Conditional source flow (runtime reachability and completion unproved): {source_hazard}"
            ),
            replacement: None,
            fixes: Vec::new(),
        },
    }
}
