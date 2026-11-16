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

//! The path-sensitive forward walk of a structured body that the flow checks thread their state sets through.

use super::{FlowDispatch, IrulesCheckWarning};
use crate::compilation_unit::FunctionUnit;
use crate::ir::{Script, Statement};
use std::collections::HashSet;
use tcl_lexer::Span;

/// The spans of the statements of `fu` that only blocks the solver proved
/// unreachable hold. A statement the flow graph does not hold at all (a body
/// it keeps as one call) is not here, and neither is one any executable
/// block holds.
pub(super) fn unreached_statements(fu: &FunctionUnit) -> HashSet<Span> {
    let mut held = HashSet::new();
    let mut reached = HashSet::new();
    for (id, block) in &fu.cfg.blocks {
        let live = fu.sccp.executable_blocks.contains(id);
        for span in block.statements.iter().map(|stmt| fu.abs_span(stmt.span())) {
            held.insert(span);
            if live {
                reached.insert(span);
            }
        }
    }
    held.retain(|span| !reached.contains(span));
    held
}

pub(super) fn walk_flow<S, L, D>(
    script: &Script,
    in_states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let mut states = in_states.to_vec();
    for stmt in &script.statements {
        if states.is_empty() {
            break;
        }
        if fd.unreached.contains(&stmt.span()) {
            continue;
        }
        // Written argv evaluates before dispatch, including return values and
        // structured headers. Braced bodies keep their own script phases.
        states = (fd.dedupe)(
            states
                .iter()
                .map(|state| (fd.leaf)(state, script, stmt, fd.out))
                .collect(),
        );
        // `None` ⇒ the statement terminates every current path (`return`).
        match flow_step(stmt, &states, fd, return_sink.as_deref_mut()) {
            None => {
                // Hand the at-return states to the enclosing catch (if any) so
                // flow resumes past it, then terminate this script's path.
                if let Some(sink) = return_sink {
                    sink.extend(states);
                }
                return Vec::new();
            }
            Some(next) => states = next,
        }
    }
    states
}

/// Transfer one structured statement over the flow-state set, or `None` when
/// it (`return`) terminates the current paths.  Control-flow arms recurse
/// through [`walk_flow`] after their written argv has been evaluated.
fn flow_step<S, L, D>(
    stmt: &Statement,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Option<Vec<S>>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    if let Some(region) = stmt
        .tokens()
        .and_then(crate::ir::CommandTokens::evaluated_body)
    {
        return Some(flow_step_region(region, states, fd, return_sink));
    }
    if let Some(tokens) = stmt.tokens()
        && fd.terminates.map_or_else(
            || {
                !crate::registry_invocation::normal_handler_completion_route(
                    fd.registry,
                    None,
                    tokens,
                )
                .normal_possible()
            },
            |terminates| terminates(tokens),
        )
    {
        return None;
    }
    Some(match stmt {
        Statement::Return { .. } => return None,
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut next = Vec::new();
            for st in states {
                for clause in clauses {
                    next.extend(walk_flow(
                        &clause.body,
                        one(st),
                        fd,
                        return_sink.as_deref_mut(),
                    ));
                }
                if let Some(eb) = else_body {
                    next.extend(walk_flow(eb, one(st), fd, return_sink.as_deref_mut()));
                } else {
                    // condition-false path keeps the incoming state.
                    next.push(st.clone());
                }
            }
            (fd.dedupe)(next)
        }
        Statement::Switch {
            arms, default_body, ..
        } => flow_step_switch(arms, default_body.as_ref(), states, fd, return_sink),
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. } => {
            let mut next = Vec::new();
            for st in states {
                next.push(st.clone()); // zero iterations
                next.extend(walk_flow(body, one(st), fd, return_sink.as_deref_mut()));
            }
            (fd.dedupe)(next)
        }
        Statement::Catch { body, .. } => {
            let mut next = Vec::new();
            for st in states {
                // `catch` swallows TCL_RETURN: both the body's normal exits and
                // any at-return states continue past the catch.  Fall back to
                // the incoming state if the body yields nothing.
                let mut catch_returns = Vec::new();
                let mut body_out = walk_flow(body, one(st), fd, Some(&mut catch_returns));
                body_out.append(&mut catch_returns);
                if body_out.is_empty() {
                    next.push(st.clone());
                } else {
                    next.extend(body_out);
                }
            }
            (fd.dedupe)(next)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => flow_step_try(
            body,
            handlers,
            finally_body.as_ref(),
            states,
            fd,
            return_sink,
        ),
        // A grouping block (namespace/uplevel body) runs in sequence.
        Statement::Block { body, .. } => walk_flow(body, states, fd, return_sink),
        _ => states.to_vec(),
    })
}

/// Diagnostic projection of the same source-positioned region used by CFG
/// construction. Possible entry retains the unchanged incoming alternative.
fn flow_step_region<S, L, D>(
    region: &crate::execution_region::EvaluatedBodyRegion,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    use tcl_registry::native_compilation::PossibleBodyTopology;
    if region.selection == crate::execution_region::RegionSelection::Never {
        return states.to_vec();
    }
    let mut next = if region.selection == crate::execution_region::RegionSelection::MaySkip
        || region.possible_bodies.is_some()
    {
        states.to_vec()
    } else {
        Vec::new()
    };
    let phases: Vec<_> = region.phases.iter().collect();
    let Some(possible) = &region.possible_bodies else {
        next.extend(flow_region_phases(&phases, states, fd, return_sink));
        return (fd.dedupe)(next);
    };
    match &possible.topology {
        PossibleBodyTopology::Sequence(_) | PossibleBodyTopology::Captured(_) => {
            next.extend(flow_region_phases(&phases, states, fd, return_sink));
        }
        PossibleBodyTopology::Loop {
            initial, repeated, ..
        } => {
            let initial_phases = selected_region_phases(region, &possible.arguments, initial);
            let entry = flow_region_phases(&initial_phases, states, fd, return_sink.as_deref_mut());
            next.extend(entry.clone());
            let repeated_phases = selected_region_phases(region, &possible.arguments, repeated);
            next.extend(flow_region_phases(
                &repeated_phases,
                &entry,
                fd,
                return_sink,
            ));
        }
        PossibleBodyTopology::Alternatives(_)
        | PossibleBodyTopology::Conditional(_)
        | PossibleBodyTopology::CaseAlternatives(_) => {
            for phase in &region.phases {
                next.extend(flow_region_phases(
                    &[phase],
                    states,
                    fd,
                    return_sink.as_deref_mut(),
                ));
            }
        }
    }
    (fd.dedupe)(next)
}

fn selected_region_phases<'a>(
    region: &'a crate::execution_region::EvaluatedBodyRegion,
    arguments: &[usize],
    selected: &[usize],
) -> Vec<&'a crate::execution_region::ExecutionPhase> {
    selected
        .iter()
        .filter_map(|argument| {
            arguments
                .iter()
                .position(|candidate| candidate == argument)
                .and_then(|index| region.phases.get(index))
        })
        .collect()
}

fn flow_region_phases<S, L, D>(
    phases: &[&crate::execution_region::ExecutionPhase],
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let mut continuing = states.to_vec();
    for phase in phases {
        let mut captured = Vec::new();
        continuing = if phase.abrupt == crate::execution_region::RegionTarget::Exit {
            walk_flow(&phase.script, &continuing, fd, Some(&mut captured))
        } else {
            walk_flow(&phase.script, &continuing, fd, return_sink.as_deref_mut())
        };
        continuing.extend(captured);
    }
    continuing
}

/// The `switch` arm of [`flow_step`]: every arm body (and the default body) is
/// a feasible path, plus the no-match fall-through that keeps the incoming
/// state.
fn flow_step_switch<S, L, D>(
    arms: &[crate::ir::SwitchArm],
    default_body: Option<&Script>,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    let mut next = Vec::new();
    for st in states {
        // no-match fall-through path.
        next.push(st.clone());
        for arm in arms {
            if let Some(body) = &arm.body {
                next.extend(walk_flow(body, one(st), fd, return_sink.as_deref_mut()));
            }
        }
        if let Some(db) = default_body {
            next.extend(walk_flow(db, one(st), fd, return_sink.as_deref_mut()));
        }
    }
    (fd.dedupe)(next)
}

/// The `try` arm of [`flow_step`]: the body and each handler are feasible
/// paths; a `finally` body runs in sequence after each; an empty result keeps
/// the incoming state.
fn flow_step_try<S, L, D>(
    body: &Script,
    handlers: &[crate::ir::TryHandler],
    finally_body: Option<&Script>,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    let mut next = Vec::new();
    for st in states {
        let mut branch = walk_flow(body, one(st), fd, return_sink.as_deref_mut());
        for handler in handlers {
            branch.extend(walk_flow(
                &handler.body,
                one(st),
                fd,
                return_sink.as_deref_mut(),
            ));
        }
        if let Some(fb) = finally_body {
            let mut final_out = Vec::new();
            for mid in &branch {
                final_out.extend(walk_flow(fb, one(mid), fd, return_sink.as_deref_mut()));
            }
            branch = final_out;
        }
        if branch.is_empty() {
            next.push(st.clone());
        } else {
            next.extend(branch);
        }
    }
    (fd.dedupe)(next)
}
