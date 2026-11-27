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

//! The object class a procedure or method returns, from its return paths.

use super::ObjectHandleFacts;
use super::constructor_harvest::constructor_class;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::ProcedureId;
use std::collections::HashMap;
use tcl_registry::CommandRegistry;

/// Callee proc qualified name → the class of the object it returns (the
/// factory-proc signal behind `set c [makeThing]`).
pub(super) fn retain_normal_procedure_result_types(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    facts: &mut ObjectHandleFacts,
) {
    let Some(known_classes) = cu.retained_source_class_names(registry) else {
        facts.normal_procedure_results.clear();
        return;
    };
    let units = procedure_implementation_units(cu);
    let mut results = crate::type_infer::NormalProcedureResultTypes::new();
    for (allocation, unit) in &units {
        join_procedure_result(&mut results, allocation, unit.return_type.clone());
    }
    // Every round propagates an already inferred normal callee result through
    // one more body. Cycles without an independent result remain overdefined.
    for _ in 0..=units.len() {
        let mut next = crate::type_infer::NormalProcedureResultTypes::new();
        for (allocation, unit) in &units {
            let Some(metadata) = crate::type_infer::TypePropagationMetadata::for_function(
                unit,
                cu.ir_module.resolved_registry(),
            ) else {
                join_procedure_result(&mut next, allocation, crate::types::TypeLattice::unknown());
                continue;
            };
            let result = crate::type_infer::infer_function_return_type_with_results(
                &unit.cfg,
                &unit.sccp,
                &unit.types,
                metadata,
                known_classes,
                &unit.ssa,
                Some(&results),
            );
            join_procedure_result(&mut next, allocation, result);
        }
        if next == results {
            break;
        }
        results = next;
    }
    facts.normal_procedure_results = results;
}

fn join_procedure_result(
    results: &mut crate::type_infer::NormalProcedureResultTypes,
    allocation: &tcl_names::command::CommandAllocation,
    result: crate::types::TypeLattice,
) {
    results
        .entry(allocation.clone())
        .and_modify(|previous| *previous = crate::types::type_join(previous, &result))
        .or_insert(result);
}

/// Match generic analysis units to the source owner's original body inventory.
/// An allocation site or procedure name alone cannot select a replacement body.
fn procedure_implementation_units(
    cu: &CompilationUnit,
) -> Vec<(&tcl_names::command::CommandAllocation, &FunctionUnit)> {
    let mut units = Vec::new();
    for implementation in cu.ir_module.procedure_implementation_bodies.iter() {
        let parameters = implementation
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        for (name, unit) in cu.procedures.iter().chain(cu.body_lowering_functions()) {
            let procedure = cu
                .ir_module
                .procedures
                .get(name)
                .or_else(|| cu.ir_module.body_lowering(name));
            let Some(procedure) = procedure else {
                continue;
            };
            if procedure.body.executed_source.as_deref() != Some(&implementation.source)
                || procedure.body.namespace_context.as_deref()
                    != Some(&implementation.namespace_key)
                || procedure
                    .params
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    != parameters
            {
                continue;
            }
            if let Some(allocation) = cu
                .ir_module
                .installed_procedure_body_units
                .get(name)
                .or_else(|| cu.ir_module.original_declaration_body_units.get(name))
                && allocation != &implementation.allocation
            {
                continue;
            }
            units.push((&implementation.allocation, unit));
        }
    }
    units
}

pub(super) fn returning_procs(cu: &CompilationUnit) -> HashMap<ProcedureId, String> {
    cu.procedures
        .values()
        .filter_map(|unit| {
            (unit.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(unit.return_type.class_name())
                .flatten()
                .and_then(|class| unit.body_id.map(|id| (id, class.to_owned())))
        })
        .collect()
}

/// Advisory factory classes, retaining an unknown return-path residual.
/// These flow candidates never populate the strict returned-object type map.
pub(super) fn returning_proc_candidates(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> HashMap<ProcedureId, String> {
    let mut returns = returning_procs(cu);
    let Some(context) =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module)
    else {
        return returns;
    };
    let metadata = Some(context);
    // A retained generic body can have original constructor evidence without
    // an executable FunctionUnit. This is a class candidate only.
    for (id, procedure) in cu.ir_module.original_source_procedure_bodies() {
        if returns.contains_key(id) {
            continue;
        }
        let Some(statement) = procedure.body.statements.last() else {
            continue;
        };
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement);
        let candidates = tokens.map_or_default(|tokens| {
            crate::registry_invocation::advisory_return_values_with_metadata_context(
                registry, metadata, tokens,
            )
        });
        for value in candidates {
            if let Some(class) = constructor_class(
                &value.legacy_text(),
                registry,
                tokens,
                metadata,
                source_analysis,
            ) {
                returns.insert(*id, class);
            }
        }
    }
    returns
}

/// Method [`FunctionUnit`] key (`::Class::method`) → the class of the object
/// it returns — the method-return counterpart of [`returning_procs`], behind
/// the `set b [$a make]` edge.
///
/// Direct declarations only: a receiver class that merely *inherits* the
/// method resolves nothing here (the lattice carries no MRO), so the edge
/// abstains rather than guess — the module-wide "no evidence ≠ no object"
/// contract.
pub(super) fn returning_methods(cu: &CompilationUnit) -> HashMap<ProcedureId, &str> {
    cu.methods
        .iter()
        .filter_map(|(key, fu)| {
            (fu.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(fu.return_type.class_name())
                .flatten()
                .map(|c| (*key, c))
        })
        .collect()
}
