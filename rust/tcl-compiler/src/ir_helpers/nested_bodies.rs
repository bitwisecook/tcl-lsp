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

//! The script bodies a structured statement contains, the namespace each runs in and whether a runtime-selected namespace can change command resolution.

use super::command_words::CommandWord;
use super::evaluated_substitutions::evaluated_command_substitutions;
use crate::ir::{ExecutionNamespace, Script, Statement};
use tcl_registry::CommandRegistry;

/// The nested script bodies a structured-control-flow statement contains —
/// every shape a flow-*insensitive* whole-body walk needs to recurse into
/// (`If`/`For`/`While`/`Foreach`/`Catch`/`Block`/`UpFrame`/`Switch`/`Try`).
/// `pub(crate)` so both [`crate::cfg_builder::global_write_info`] (per-proc
/// global-write summaries) and [`crate::var_observability`] (module-wide
/// trace-target summaries) share one recursive-descent shape instead of each
/// re-deriving it.
///
/// Distinct from [`collect_defs_from_script`]'s per-statement match: that
/// walk also extracts *defs* at each nesting level (iteration variables,
/// catch/try result vars) as part of the same pass, so it isn't a drop-in
/// replacement for a caller that only wants "which scripts nest inside this
/// statement".
#[must_use]
pub(crate) fn nested_bodies(stmt: &Statement) -> Vec<&Script> {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut bodies: Vec<&Script> = clauses.iter().map(|c| &c.body).collect();
            if let Some(e) = else_body {
                bodies.push(e);
            }
            bodies
        }
        Statement::For {
            init, next, body, ..
        } => vec![init, next, body],
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => vec![body],
        Statement::Switch {
            arms, default_body, ..
        } => {
            let mut bodies: Vec<&Script> = arms.iter().filter_map(|a| a.body.as_ref()).collect();
            if let Some(d) = default_body {
                bodies.push(d);
            }
            bodies
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let mut bodies = vec![body];
            bodies.extend(handlers.iter().map(|h| &h.body));
            if let Some(f) = finally_body {
                bodies.push(f);
            }
            bodies
        }
        _ => Vec::new(),
    }
}

/// Nested script bodies paired with the command namespace in which Tcl runs
/// them. This is the sole nested-execution context transform for semantic IR
/// walkers: `#0` selects global, relative zero inherits, and every other
/// caller selection remains unknown.
#[must_use]
pub(crate) fn nested_execution_bodies<'a>(
    stmt: &'a Statement,
    parent: &ExecutionNamespace,
) -> Vec<(&'a Script, ExecutionNamespace)> {
    let context = match stmt {
        Statement::UpFrame {
            absolute,
            frame_shift,
            ..
        } => parent.selected_frame(*absolute, *frame_shift),
        _ => parent.clone(),
    };
    nested_bodies(stmt)
        .into_iter()
        .map(|body| (body, context.clone()))
        .collect()
}

/// Whether executing `script` in a runtime-selected current namespace can
/// resolve any command differently from the compiler's lexical fallback.
///
/// Direct literal call heads, typed-lowering dependencies, and recovered
/// literal command substitutions all enter through this single inventory.
/// Absolute heads are namespace-invariant; relative literal heads may be
/// shadowed by the runtime receiver/caller namespace. Computed heads are not a
/// lexical fallback dependency: the CFG already represents them as opaque
/// runtime dispatch and may still analyse facts on either side of that barrier.
#[must_use]
pub(crate) fn requires_runtime_command_namespace(
    script: &Script,
    registry: &CommandRegistry,
) -> bool {
    let mut pending = vec![(script, ExecutionNamespace::RuntimeSelected)];
    while let Some((script, execution_namespace)) = pending.pop() {
        if script
            .command_binding_sites
            .iter()
            .any(|site| execution_namespace.for_head(&site.binding.name).is_none())
        {
            return true;
        }
        for stmt in &script.statements {
            let specialised_binding = match stmt {
                Statement::AssignExpr {
                    command_binding, ..
                }
                | Statement::Return {
                    command_binding, ..
                } => command_binding.as_ref(),
                Statement::ExprEval {
                    command_binding, ..
                } => Some(command_binding),
                _ => None,
            };
            if specialised_binding
                .is_some_and(|binding| execution_namespace.for_head(&binding.name).is_none())
            {
                // A nested command consumed by typed lowering does not appear
                // in the expression/text surfaces below. Its binding metadata is
                // therefore the authoritative source head for the same
                // runtime-selected namespace proof as an ordinary Call.
                return true;
            }
            if let Statement::Call {
                command, tokens, ..
            }
            | Statement::Barrier {
                command, tokens, ..
            } = stmt
            {
                let needs_runtime_namespace = match tokens {
                    Some(tokens) if tokens.synthetic.is_some() => false,
                    Some(tokens) => tokens
                        .words()
                        .first()
                        .and_then(|head| {
                            crate::registry_invocation::invocation_word(head).literal()
                        })
                        .is_some_and(|head| execution_namespace.for_head(head).is_none()),
                    None => execution_namespace.for_head(command).is_none(),
                };
                if needs_runtime_namespace {
                    return true;
                }
            }
            let embedded = evaluated_command_substitutions(stmt, registry);
            if embedded.commands.iter().any(|command| {
                command
                    .first()
                    .and_then(CommandWord::literal)
                    .is_some_and(|head| execution_namespace.for_head(head).is_none())
            }) {
                return true;
            }
            pending.extend(nested_execution_bodies(stmt, &execution_namespace));
        }
    }
    false
}
