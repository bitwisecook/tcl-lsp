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

//! IRULE5002 and IRULE5004: a `drop`, `reject` or `DNS::return` that survives unguarded along a path.

use super::flow_inventory::original_irules_flow_commands;
use super::flow_walk::{unreached_statements, walk_flow};
use super::{
    CodeFix, DropFlowState, FlowDispatch, IrulesCheckWarning, IrulesSourceQuery,
    OriginalEventPresence, OriginalIrulesFlowCommand, is_irules,
};
use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::Span;

/// Deduplicate drop states by `(dropped, dns_returned)`, keeping the first.
pub(super) fn dedupe_drop_states(states: Vec<DropFlowState>) -> Vec<DropFlowState> {
    let mut seen: HashSet<(bool, bool)> = HashSet::new();
    let mut out = Vec::new();
    for s in states {
        if seen.insert((s.dropped, s.dns_returned)) {
            out.push(s);
        }
    }
    out
}

pub(super) fn drop_flow_command(
    state: &DropFlowState,
    command: &OriginalIrulesFlowCommand,
) -> DropFlowState {
    let mut next = state.clone();
    if command.disables_all_events {
        next.dropped = false;
        next.drop_cmd.clear();
        next.drop_at = None;
    }
    if command.packet_actions.drops_connection {
        next.dropped = true;
        next.drop_cmd.clone_from(&command.command);
        next.drop_at = Some(command.span);
    }
    if command.packet_actions.returns_dns_packet {
        next.dns_returned = true;
        next.dns_at = Some(command.span);
    }
    next
}

/// Path-sensitive IRULE5002 / IRULE5004 — an unguarded `drop`/`reject`/
/// `discard` (no following `event disable all` / `return`) or a `DNS::return`
/// without a following `return`, on at least one path through an original event
/// body.
#[must_use]
pub fn find_unguarded_drop_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let Some(source) = IrulesSourceQuery::capture(cu, cu.ir_module.resolved_registry()) else {
        return out;
    };
    // Honour the complexity guard but walk the structured
    // IR body of each analysable original event handler.
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let OriginalEventPresence::Event(event) = event else {
            continue;
        };
        let proc = event.procedure;
        let Some(config) = source.function_config(fu) else {
            return Vec::new();
        };
        let missing_source = std::cell::Cell::new(false);
        let leaf = |state: &DropFlowState,
                    script: &Script,
                    statement: &Statement,
                    _out: &mut Vec<IrulesCheckWarning>| {
            let Some(commands) = original_irules_flow_commands(script, statement, &source, config)
            else {
                missing_source.set(true);
                return state.clone();
            };
            commands.iter().fold(state.clone(), |current, command| {
                drop_flow_command(&current, command)
            })
        };
        let unreached = unreached_statements(fu);
        let finals = walk_flow(
            &proc.body,
            &[DropFlowState::default()],
            &mut FlowDispatch {
                registry: cu.ir_module.resolved_registry(),
                out: &mut out,
                leaf: &leaf,
                dedupe: &dedupe_drop_states,
                unreached: &unreached,
                terminates: None,
            },
            None,
        );
        if missing_source.get() {
            return Vec::new();
        }
        for st in finals {
            if st.dropped
                && let Some(span) = st.drop_at
            {
                out.push(IrulesCheckWarning {
                    span,
                    code: DiagCode::Irule5002,
                    message: format!(
                        "'{}' without 'event disable all' or 'return' — other iRules and later \
                         priorities in this event will still execute, which may cause TCL errors.",
                        st.drop_cmd,
                    ),
                    replacement: None,
                    // Insert `event disable all` + `return` right after the drop
                    // (a zero-width edit at the drop command's end).
                    fixes: vec![CodeFix {
                        span: Span::new(span.end(), span.end()),
                        new_text: "\n    event disable all\n    return".to_owned(),
                        description: "Add 'event disable all' + 'return'".to_owned(),
                        // IRULE5002: adding `event disable all` + `return` stops later
                        // iRules and priorities running — the fix, and a control-flow change.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                });
            }
            if st.dns_returned
                && let Some(span) = st.dns_at
            {
                out.push(IrulesCheckWarning {
                    span,
                    code: DiagCode::Irule5004,
                    message: "'DNS::return' must be followed by 'return' to stop iRule processing."
                        .to_owned(),
                    replacement: None,
                    // Insert `return` right after `DNS::return`.
                    fixes: vec![CodeFix {
                        span: Span::new(span.end(), span.end()),
                        new_text: "\n    return".to_owned(),
                        description: "Add 'return' after DNS::return".to_owned(),
                        // IRULE5004: adding `return` stops iRule processing after
                        // `DNS::return` — a control-flow change.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                });
            }
        }
    }
    out
}
