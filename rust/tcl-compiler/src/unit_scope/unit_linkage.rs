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

//! The registry-declared unit boundaries a module crosses, found by walking every script it owns.

use crate::ir::Module as IrModule;
use tcl_registry::{CommandRegistry, Traits};

/// Possible registry-declared unit boundaries under retained source metadata.
/// Missing or foreign input contributes no invented boundary trait; the caller
/// collector independently retracts seeds when it cannot own the source context.
///
/// A pure union of [`CommandRegistry::unit_linkage`] over every resolved
/// `Call`/`Barrier` statement in the module — top level, every
/// proc/method/body-unit, and every nested control-flow body.  No command
/// name appears here: `package provide`, `source`, `namespace export` and
/// friends are recognised only through the traits their specs carry, so
/// teaching the compiler about a new boundary command is a registry edit
/// (see [`tcl_registry::traits::UNIT_LINKAGE_TRAITS`]).
///
/// Neither a raw-text `package provide` substring scan nor an IR walk keyed
/// on the `package`/`provide` word pair would do: the first both
/// over-triggers (any script merely *mentioning* the phrase in a comment or
/// string disables every interprocedural seed in the file) and under-triggers
/// (`package\tprovide`, `::package provide`); the second knows a command by
/// name and misses every other way a file admits to being part of a larger
/// program.
#[must_use]
pub fn scan_unit_linkage(
    ir_module: &IrModule,
    registry: &CommandRegistry,
    _dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Traits {
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module);
    let mut found = Traits::empty();
    let mut visit = |script: &crate::ir::Script, stmt: &crate::ir::Statement| {
        let Some(tokens) = script.retained_source_tokens_for_statement(stmt) else {
            return;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let context = if metadata.is_some() {
            binding
                .original_invocation_metadata_for_module(tokens, ir_module, registry)
                .map(Some)
        } else if ir_module.source_entry.metadata_context.is_standalone() {
            binding
                .original_invocation_metadata_for_owner(
                    tokens,
                    &ir_module.source_entry.metadata_context,
                    registry,
                )
                .ok()
        } else {
            None
        };
        let Some(context) = context else {
            return;
        };
        let invocation = context.filter(|metadata| binding.original_invocation_source_purpose(
            tokens, *metadata, registry,
        ) == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical))
            .and_then(|metadata| crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(registry, metadata, tokens))
            .or_else(|| crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(registry, context, tokens));
        if let Some(invocation) = invocation {
            found |= invocation
                .facts
                .traits
                .intersection(tcl_registry::UNIT_LINKAGE_TRAITS);
        }
        // Possibility withdraws caller closure, independently of execution.
        if let Some(assistance) =
            crate::registry_invocation::registry_invocation_assistance_with_metadata_context(
                registry, context, tokens,
            )
        {
            for candidate in assistance.candidates {
                found |= candidate
                    .possible_traits
                    .intersection(tcl_registry::UNIT_LINKAGE_TRAITS);
            }
        }
    };
    walk_module_scripts(ir_module, &mut visit);
    found
}

/// Visit every statement of every script the module owns — top level,
/// procedures, `TclOO` methods, and synthetic body units — descending into
/// nested control-flow bodies.
pub(super) fn walk_module_scripts<'a>(
    ir_module: &'a IrModule,
    visit: &mut impl FnMut(&'a crate::ir::Script, &'a crate::ir::Statement),
) {
    walk_script(&ir_module.top_level, visit);
    for proc in ir_module.procedures.values() {
        walk_script(&proc.body, visit);
    }
    for method in ir_module.methods.values() {
        walk_script(&method.body, visit);
    }
    for (_, unit) in ir_module.body_lowerings() {
        walk_script(&unit.body, visit);
    }
}

/// Visit every statement in `script`, descending into every nested
/// control-flow body.  Written as a module-level pair with
/// [`walk_statement`] rather than nested inside its callers so the
/// mutual recursion reads as two ordinary functions.
fn walk_script<'a>(
    script: &'a crate::ir::Script,
    visit: &mut impl FnMut(&'a crate::ir::Script, &'a crate::ir::Statement),
) {
    for stmt in &script.statements {
        walk_statement(script, stmt, visit);
    }
}

/// The [`walk_script`] half that dispatches one statement.
fn walk_statement<'a>(
    script: &'a crate::ir::Script,
    stmt: &'a crate::ir::Statement,
    visit: &mut impl FnMut(&'a crate::ir::Script, &'a crate::ir::Statement),
) {
    use crate::ir::Statement;
    visit(script, stmt);
    // Only the body-bearing arms have anything left to do — everything else
    // is a leaf the visit above has already seen.
    match stmt {
        Statement::Block { body, .. }
        | Statement::UpFrame { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => walk_script(body, visit),
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                walk_script(&clause.body, visit);
            }
            if let Some(body) = else_body {
                walk_script(body, visit);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(init, visit);
            walk_script(next, visit);
            walk_script(body, visit);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(body, visit);
            for handler in handlers {
                walk_script(&handler.body, visit);
            }
            if let Some(body) = finally_body {
                walk_script(body, visit);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(body) = &arm.body {
                    walk_script(body, visit);
                }
            }
            if let Some(body) = default_body {
                walk_script(body, visit);
            }
        }
        Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Return { .. } => {}
    }
}
