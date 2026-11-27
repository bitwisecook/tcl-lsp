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

//! iRules storage policy over the resolved cells: global and connection-scope concerns and the racy `static::` flow (IRULE4003, IRULE4005).

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use rustc_hash::FxHashSet;
use std::collections::HashSet;
use tcl_core_types::DiagCode;

impl Analyser {
    /// Apply F5 storage policy to the shared point-resolved Tcl cells.
    /// Aliases and absolute namespace spellings retain the same host effects.
    pub(in super::super) fn emit_irules_cell_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: Option<&str>,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if !self.profile.is_irules() {
            return;
        }
        let Some(points) = fu.ssa.point_contexts.as_ref() else {
            return;
        };
        let context = self.analysis_context();
        if context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key() {
            return;
        }
        let mut emitted = FxHashSet::default();
        for (&id, block) in &fu.cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                let before = points.before_statement(id, index);
                let after = points.after_statement(id, index);
                for access in irules_cell_accesses(statement, before, after, &context) {
                    self.emit_irules_cell_access(fu, event, statement, access, &mut emitted);
                }
                for name in irules_possible_namespace_writes(statement, before, after, &context) {
                    let span = fu.abs_span(statement.span());
                    if span.is_empty()
                        || !emitted.insert((DiagCode::Irule6001, span.start(), name.clone()))
                    {
                        continue;
                    }
                    self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Irule6001,
                        span,
                        format!("Writing through namespace variable '{name}' may select shared global state, forcing CMP compatibility mode and pinning the virtual server to a single TMM."),
                        Severity::Warning,
                    ));
                }
            }
        }
    }

    fn emit_irules_cell_access(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: Option<&str>,
        statement: &crate::ir::Statement,
        access: IrulesCellAccess,
        emitted: &mut FxHashSet<(DiagCode, u32, String)>,
    ) {
        use tcl_registry::f5::{BigIpExecutionContext, VariableStorageDomain};
        let IrulesCellAccess {
            place,
            import,
            implicit_destruction,
        } = access;
        if place.dynamic || !place.is_global() {
            return;
        }
        let domain =
            tcl_registry::f5::namespace_storage_domain(BigIpExecutionContext::TmmIRule, &place.ns);
        let code = match domain {
            VariableStorageDomain::WorkerNamespace
                if event != Some("RULE_INIT") && import.is_none() =>
            {
                DiagCode::Irule4001
            }
            VariableStorageDomain::CmpGlobal if !implicit_destruction => DiagCode::Irule6001,
            _ => return,
        };
        let target = crate::naming::qualify(&place.ns, &place.name);
        let span = fu.abs_span(statement.span());
        if span.is_empty() || !emitted.insert((code, span.start(), target.clone())) {
            return;
        }
        let message = if code == DiagCode::Irule4001 {
            format!(
                "Writing to '{target}' outside RULE_INIT changes persistent state for \
                     connections on this TMM. The update is not propagated to other TMMs."
            )
        } else if let Some(import) = import {
            format!(
                "'{import}' imports global namespace variable '{target}', forcing CMP compatibility mode and pinning the virtual server to a single TMM. Use 'static::{}' instead.",
                place.name
            )
        } else {
            format!(
                "Global namespace variable '{target}' forces CMP compatibility mode, \
                     pinning the virtual server to a single TMM. Use 'static::{}' instead.",
                place.name
            )
        };
        let fixes = if code == DiagCode::Irule6001 {
            self.irules_global_name_fix(fu, event, statement, &place)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(code, span, message, Severity::Warning)
                .with_fixes(fixes),
        );
    }

    fn irules_global_name_fix(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: Option<&str>,
        statement: &crate::ir::Statement,
        place: &tcl_names::place::Place,
    ) -> Option<crate::analyser::types::CodeFix> {
        let (crate::ir::Statement::AssignConst { name, .. }
        | crate::ir::Statement::AssignValue { name, .. }
        | crate::ir::Statement::AssignExpr { name, .. }
        | crate::ir::Statement::Incr { name, .. }) = statement
        else {
            return None;
        };
        if !tcl_syntax::naming::is_absolute(name) && event != Some("RULE_INIT") {
            return None;
        }
        let (base, index) = crate::naming::split_array_name(name);
        let qualified = crate::naming::qualify("::", base);
        let (namespace, tail) = crate::naming::key_holder_and_tail(&qualified);
        if namespace != "::" || tail != place.name {
            return None;
        }
        let span = self.narrow_to_assigned_name(fu.abs_span(statement.span()))?;
        let replacement = crate::naming::unrooted_member_key("static", tail);
        let replacement = match index {
            Some(index) => format!("{replacement}({index})"),
            None => replacement,
        };
        Some(crate::analyser::types::CodeFix {
            span,
            new_text: replacement.clone(),
            description: format!("Replace '{name}' with '{replacement}'"),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        })
    }

    /// IRULE4003: lifecycle concerns for actual shared connection cells.
    pub(in super::super) fn emit_connection_scope_concerns(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: &str,
        concerns: &std::collections::HashMap<crate::connection_scope::EventCell, HashSet<String>>,
        registry: &tcl_registry::CommandRegistry,
        module: &crate::ir::Module,
    ) {
        let points = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            crate::connection_scope::event_resolve_context(event),
            registry,
        );
        let mut emitted = FxHashSet::default();
        for (&id, block) in &fu.cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                if crate::connection_scope::statement_destroys(
                    fu,
                    id,
                    index,
                    registry,
                    Some(module),
                )
                .is_some_and(|destroys| destroys)
                {
                    continue;
                }
                for place in crate::place_bridge::def_places(
                    statement,
                    points.before_statement(id, index),
                    registry,
                ) {
                    if place.dynamic || place.ns != tcl_names::place::LOCAL_NS {
                        continue;
                    }
                    let Some(cell) = crate::connection_scope::cell_from_place(&place) else {
                        continue;
                    };
                    let Some(notes) = concerns.get(&cell) else {
                        continue;
                    };
                    let span = fu.abs_span(statement.span());
                    if span.is_empty() || !emitted.insert((span.start(), place.name.clone())) {
                        continue;
                    }
                    let mut notes: Vec<_> = notes.iter().map(String::as_str).collect();
                    notes.sort_unstable();
                    self.result
                        .diagnostics
                        .push(crate::analyser::types::Diagnostic::new(
                            DiagCode::Irule4003,
                            span,
                            format!("Variable '{}': {}", place.name, notes.join("; ")),
                            Severity::Hint,
                        ));
                }
            }
        }
    }

    /// IRULE4005: an actual worker cell write may be observed in another event.
    pub(in super::super) fn emit_racy_static_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: &str,
        racy_cells: &HashSet<crate::connection_scope::EventCell>,
        registry: &tcl_registry::CommandRegistry,
        module: &crate::ir::Module,
    ) {
        if self.disabled_diagnostics.contains("IRULE4005") {
            return;
        }
        let fallback_points;
        let points = if let Some(points) = &fu.ssa.point_contexts {
            points
        } else {
            fallback_points = crate::place_bridge::build_point_resolve_contexts_with_entry(
                &fu.cfg,
                crate::connection_scope::event_resolve_context(event),
                registry,
            );
            &fallback_points
        };
        let mut emitted = FxHashSet::default();
        for (&id, block) in &fu.cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                if crate::connection_scope::statement_destroys(
                    fu,
                    id,
                    index,
                    registry,
                    Some(module),
                )
                .is_some_and(|destroys| destroys)
                {
                    continue;
                }
                for place in crate::place_bridge::def_places(
                    statement,
                    points.before_statement(id, index),
                    registry,
                ) {
                    if place.dynamic || !place.is_global() {
                        continue;
                    }
                    let Some(cell) = crate::connection_scope::cell_from_place(&place) else {
                        continue;
                    };
                    if !racy_cells.contains(&cell) {
                        continue;
                    }
                    // Presentation follows the exact worker-cell match.
                    let name = crate::naming::qualify(&place.ns, &place.name);
                    let span = fu.abs_span(statement.span());
                    if span.is_empty() || !emitted.insert((span.start(), name.clone())) {
                        continue;
                    }
                    self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Irule4005, span,
                        format!("Persistent state: '{name}' is written outside RULE_INIT and read in \
                                 another event. Connections on this TMM share the value; updates \
                                 are not propagated to other TMMs."),
                        Severity::Warning,
                    ));
                }
            }
        }
    }
}

/// Domain diagnostics retain physical mutation and how an operand selected its
/// namespace. An implicit current-frame destruction creates no global value.
struct IrulesCellAccess {
    place: tcl_names::place::Place,
    import: Option<String>,
    implicit_destruction: bool,
}

/// Value writes come from the shared place bridge. Binding declarations use
/// the shared successful-continuation context and registry alias transition.
fn irules_cell_accesses(
    statement: &crate::ir::Statement,
    before: &crate::var_resolve::ResolveContext,
    after: &crate::var_resolve::ResolveContext,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<IrulesCellAccess> {
    let registry = context.commands();
    let mut accesses: Vec<_> = crate::place_bridge::statement_mutation_places_with_continuation(
        statement, before, after, registry,
    )
    .into_iter()
    .map(|place| IrulesCellAccess {
        place,
        import: None,
        implicit_destruction: false,
    })
    .collect();
    let crate::ir::Statement::Call {
        command,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        return accesses;
    };

    let Some(normal) =
        crate::registry_invocation::normal_transfer_invocation_in_context(context, tokens)
    else {
        return accesses;
    };
    if normal
        .variable_traits()
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
    {
        for (index, role) in normal.variable_roles() {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let Some(name) = normal.argument_literal(index) else {
                continue;
            };
            if crate::var_resolve::literal_namespace_access_origin(
                &name,
                before,
                registry,
                tcl_registry::TraceOperation::Unset,
            ) != crate::var_resolve::NamespaceAccessOrigin::CurrentFrame
            {
                continue;
            }
            let target = crate::var_resolve::resolve_literal_access(
                &name,
                before,
                false,
                registry,
                tcl_registry::TraceOperation::Unset,
            );
            for access in &mut accesses {
                if crate::var_resolve::cell_key(&access.place)
                    == crate::var_resolve::cell_key(&target)
                {
                    access.implicit_destruction = true;
                }
            }
        }
    }
    for alias in normal.variable_alias_transitions() {
        if alias.writes_value {
            continue;
        }
        let Some(local) = alias.local.literal() else {
            continue;
        };
        let place = crate::var_resolve::resolve_place(local, after, false, registry);
        let import = format!("{command} {local}");
        accesses.push(IrulesCellAccess {
            place,
            import: Some(import),
            implicit_destruction: false,
        });
    }
    accesses
}

/// Uncertain contents addresses retain their proved namespace-only domain.
/// This diagnostic evidence supplies neither a physical cell nor a value definition.
fn irules_possible_namespace_writes(
    statement: &crate::ir::Statement,
    before: &crate::var_resolve::ResolveContext,
    after: &crate::var_resolve::ResolveContext,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    let registry = context.commands();
    let Some(tokens) = statement.tokens() else {
        return Vec::new();
    };

    let Some(normal) =
        crate::registry_invocation::normal_transfer_invocation_in_context(context, tokens)
    else {
        return irules_possible_handler_namespace_writes(tokens, before, after, context);
    };
    let state = match normal.variable_binding_phase() {
        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments => before,
        tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation => after,
        tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol => {
            return Vec::new();
        }
    };
    normal
        .definition_names()
        .into_iter()
        .filter(|name| {
            crate::var_resolve::resolve_place(name, state, false, registry).kind
                == tcl_names::place::PlaceKind::Unknown
                && crate::var_resolve::possible_access_domain(name, state, registry)
                    == crate::var_resolve::PossibleVariableAccessDomain::NamespaceOnly
        })
        .collect()
}

/// Candidate output lookup can remain opaque (for example a channel callback)
/// while a namespace-only destination hazard is still possible. This query
/// creates neither a normal store nor a physical cell identity.
fn irules_possible_handler_namespace_writes(
    tokens: &crate::ir::CommandTokens,
    before: &crate::var_resolve::ResolveContext,
    after: &crate::var_resolve::ResolveContext,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    let registry = context.commands();
    let Some(possible) =
        crate::registry_invocation::possible_variable_name_operands_in_context(context, tokens)
    else {
        return Vec::new();
    };
    possible
        .phased_operands()
        .filter_map(|(role, word, phase, destroys)| {
            if role != tcl_registry::ArgRole::VarWrite {
                return None;
            }
            let name = word.as_registry_word().literal()?;
            let context = match phase {
                tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments => before,
                tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation => after,
                tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol => return None,
            };
            if destroys
                && crate::var_resolve::literal_namespace_access_origin(
                    name,
                    context,
                    registry,
                    tcl_registry::TraceOperation::Unset,
                ) == crate::var_resolve::NamespaceAccessOrigin::CurrentFrame
            {
                return None;
            }
            let place = crate::var_resolve::resolve_place(name, context, false, registry);
            if place.is_global()
                && tcl_registry::f5::namespace_storage_domain(
                    tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                    &place.ns,
                ) == tcl_registry::f5::VariableStorageDomain::WorkerNamespace
            {
                return None;
            }
            (crate::var_resolve::possible_access_domain(name, context, registry)
                == crate::var_resolve::PossibleVariableAccessDomain::NamespaceOnly)
                .then(|| name.to_owned())
        })
        .collect()
}
