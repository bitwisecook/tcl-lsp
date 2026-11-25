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

//! The original flow commands of one handler body: packet actions, response commits and collection operations.

use super::{
    CollectionSourceObligationKind, IrulesSourceQuery, OriginalIrulesFlowCommand,
    OriginalIrulesPacketActions,
};
use crate::ir::{Script, Statement};
use std::collections::HashSet;
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, Traits};

pub(super) struct OriginalIruleFlowInventory {
    pub(super) commands: Vec<OriginalIrulesFlowCommand>,
    pub(super) obligations: HashSet<CollectionSourceObligationKind>,
}

/// Actual source owners retain typed-lowering carriers and all argv children.
/// Missing source is distinct from an admitted command with no flow metadata.
pub(super) fn original_irules_flow_commands(
    script: &Script,
    statement: &Statement,
    source: &IrulesSourceQuery<'_>,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<OriginalIrulesFlowCommand>> {
    let inventory = original_irules_flow_inventory(script, statement, source, config)?;
    inventory
        .obligations
        .is_empty()
        .then_some(inventory.commands)
}

pub(super) fn original_irules_flow_inventory(
    script: &Script,
    statement: &Statement,
    source: &IrulesSourceQuery<'_>,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalIruleFlowInventory> {
    let mut obligations = HashSet::new();
    if let crate::ir::OriginalArgvInventoryDisposition::EffectOnly(marker) =
        statement.original_argv_inventory_disposition()
    {
        obligations.insert(CollectionSourceObligationKind::EffectOnly(marker));
        return Some(OriginalIruleFlowInventory {
            commands: Vec::new(),
            obligations,
        });
    }
    let parent = script.retained_source_tokens_for_statement(statement)?;
    parent
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(parent)?;
    let children = source.argument_calls(parent, config)?;
    let children = children
        .iter()
        .map(|child| child.tokens.as_ref())
        .collect::<Option<Vec<_>>>()?;
    let mut calls = Vec::new();
    for tokens in children.into_iter().chain(std::iter::once(parent)) {
        let possible =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                source.registry,
                source.metadata(),
                tokens,
            )?;
        if possible.unknown_residual {
            obligations.insert(CollectionSourceObligationKind::UnknownTarget);
        }
        if possible.may_be_absent {
            obligations.insert(CollectionSourceObligationKind::MayBeAbsent);
        }
        if let tcl_core_types::Knowledge::Proven(command) =
            possible_irule_flow_command(source.registry, tokens, &possible)?
        {
            calls.push(command);
        }
    }
    Some(OriginalIruleFlowInventory {
        commands: calls,
        obligations,
    })
}

pub(super) fn possible_irule_flow_command(
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
    possible: &crate::registry_invocation::RegistryInvocationAssistance,
) -> Option<tcl_core_types::Knowledge<OriginalIrulesFlowCommand>> {
    use tcl_registry::events::IrulesSourceFlowAction;
    let mut drops_connection = false;
    let mut returns_dns_packet = false;
    let mut may_commit_response = false;
    let mut requires_http_context = false;
    for candidate in &possible.candidates {
        drops_connection |=
            candidate.irules_source_flow_action == Some(IrulesSourceFlowAction::DropsConnection);
        returns_dns_packet |=
            candidate.irules_source_flow_action == Some(IrulesSourceFlowAction::ReturnsDnsPacket);
        may_commit_response |= candidate.source_may_commit_response;
        // The transaction requirement is declared on the command. A selected
        // subcommand (`HTTP::header insert`) resolves to its own traits, which
        // do not repeat it.
        requires_http_context |= candidate
            .possible_traits
            .contains(Traits::REQUIRES_HTTP_CONTEXT)
            || registry
                .get(&candidate.command)
                .is_some_and(|spec| spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT));
    }
    // Conditional advice keeps lookup closure as an obligation on both sides:
    // a guard counts by its candidates exactly as a drop does.
    let disables_all_events = !possible.candidates.is_empty()
        && possible.candidates.iter().all(|candidate| {
            candidate.irules_source_flow_action == Some(IrulesSourceFlowAction::DisablesAllEvents)
        });
    if !(drops_connection
        || returns_dns_packet
        || may_commit_response
        || requires_http_context
        || disables_all_events)
    {
        return Some(tcl_core_types::Knowledge::Absent);
    }
    let command = tokens.argv_texts.first()?.clone();
    Some(tcl_core_types::Knowledge::Proven(
        OriginalIrulesFlowCommand {
            command,
            span: Span::new(tokens.argv.first()?.start(), tokens.argv.last()?.end()),
            packet_actions: OriginalIrulesPacketActions {
                drops_connection,
                returns_dns_packet,
            },
            disables_all_events,
            may_commit_response,
            requires_http_context,
        },
    ))
}
