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

//! IRULE1201 and IRULE1202: HTTP commands after a committed response and second responses on one path.

use super::flow_inventory::original_irules_flow_commands;
use super::flow_walk::{unreached_statements, walk_flow};
use super::{
    FlowDispatch, HttpFlowState, IrulesCheckPayload, IrulesCheckWarning, IrulesSourceQuery,
    OriginalIrulesFlowCommand, is_irules,
};
use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::events::EventRegistry;

/// Per-event HTTP-flow warnings. Emits IRULE1201 + IRULE1202 for each handler
/// whose event registry entry implies an HTTP profile, path-sensitively.
#[must_use]
pub fn find_http_flow_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let Some(source) = IrulesSourceQuery::capture(cu, cu.ir_module.resolved_registry()) else {
        return out;
    };
    let events = EventRegistry::build();
    // Iterate the *analysable* functions (honouring the complexity
    // guard) but walk their structured IR body.
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let tcl_core_types::Knowledge::Proven(event) = event else {
            continue;
        };
        let bare_event = event.body.event();
        if !events
            .get_props(bare_event)
            .is_some_and(|props| props.implied_profiles.contains(&"HTTP"))
        {
            continue;
        }
        let proc = event.procedure;
        let unreached = unreached_statements(fu);
        let Some(config) = source.function_config(fu) else {
            return Vec::new();
        };
        let missing_source = std::cell::Cell::new(false);
        let leaf = |state: &HttpFlowState,
                    script: &Script,
                    statement: &Statement,
                    out: &mut Vec<IrulesCheckWarning>| {
            let Some(commands) = original_irules_flow_commands(script, statement, &source, config)
            else {
                missing_source.set(true);
                return *state;
            };
            commands.iter().fold(*state, |current, command| {
                apply_http_flow_command(current, command, bare_event, out)
            })
        };
        walk_flow(
            &proc.body,
            &[HttpFlowState {
                responded: false,
                respond_at: None,
            }],
            &mut FlowDispatch {
                registry: cu.ir_module.resolved_registry(),
                out: &mut out,
                leaf: &leaf,
                dedupe: &dedupe_flow_states,
                unreached: &unreached,
                terminates: None,
            },
            None,
        );
        if missing_source.get() {
            return Vec::new();
        }
    }
    // Distinct predecessor states can prove the same source command unsafe.
    // The proof multiplicity is internal; the editor should receive one
    // diagnostic per code/range/message.
    let mut seen = HashSet::new();
    out.retain(|warning| {
        seen.insert((
            warning.code,
            warning.span.start(),
            warning.span.end(),
            warning.message.clone(),
        ))
    });
    out
}

/// Deduplicate flow states by `(responded, respond_at)`.
pub(super) fn dedupe_flow_states(states: Vec<HttpFlowState>) -> Vec<HttpFlowState> {
    let mut seen: HashSet<(bool, i64)> = HashSet::new();
    let mut out = Vec::new();
    for s in states {
        let key = (s.responded, s.respond_at.map_or(-1, i64::from));
        if seen.insert(key) {
            out.push(s);
        }
    }
    out
}

/// Apply one command to a flow state, emitting IRULE1201/1202 as warranted.
fn apply_http_flow_command(
    state: HttpFlowState,
    command: &OriginalIrulesFlowCommand,
    event: &str,
    out: &mut Vec<IrulesCheckWarning>,
) -> HttpFlowState {
    let cmd = &command.command;
    let span = command.span;
    if command.may_commit_response {
        if state.responded {
            out.push(IrulesCheckWarning {
                span,
                code: DiagCode::Irule1202,
                message: format!(
                    "Multiple '{cmd}' calls possible in {event}. Only the first response takes effect.",
                ),
                payload: IrulesCheckPayload::RepeatedResponse {
                    command: cmd.clone(),
                    event: event.to_owned(),
                },
                replacement: None,
                fixes: Vec::new(),
            });
            return state;
        }
        return HttpFlowState {
            responded: true,
            respond_at: Some(span.start()),
        };
    }
    if state.responded && command.requires_http_context {
        out.push(IrulesCheckWarning {
            span,
            code: DiagCode::Irule1201,
            message: format!(
                "'{cmd}' used after response is committed. HTTP context is invalid after HTTP::respond/HTTP::redirect.",
            ),
            payload: IrulesCheckPayload::ContextAfterResponse {
                command: cmd.clone(),
                event: event.to_owned(),
            },
            replacement: None,
            fixes: Vec::new(),
        });
    }
    state
}
