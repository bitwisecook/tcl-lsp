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

//! The `unitScope` and `interprocedural` views: who can call a procedure and each procedure's summary.

use crate::ExplorerResult;
use crate::formatters::format_return_shape;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tcl_compiler::interprocedural::InterproceduralAnalysis;

/// Format a declared arity's arity string:
/// `"{min}+"` when unlimited, else `"{min}..{max}"`.
fn arity_str(arity: tcl_registry::Arity) -> String {
    if arity.is_unlimited() {
        format!("{}+", arity.min)
    } else {
        format!("{}..{}", arity.min, arity.max)
    }
}

/// Serialise the `unitScope` view: who else can call this file's procedures.
///
/// Shows the registry-declared unit boundaries the file crosses
/// (`package provide`, `source`, `namespace export`, …), whether the host
/// supplied a cross-file view, and the merged call-site evidence per callee
/// with the seed verdict at each argument position — the inputs
/// `tcl_compiler::unit_scope::params_constants_from_call_sites` reads, so a
/// surprising (or surprisingly absent) constant fold can be traced to the
/// evidence that produced it.
#[must_use]
pub fn serialise_unit_scope(result: &ExplorerResult) -> Value {
    let scope = &result.unit.caller_scope;
    // `scan_unit_linkage` already masks to `UNIT_LINKAGE_TRAITS`, so every
    // name here is a boundary — and `iter_names` is generated from the trait
    // declarations, so this cannot drift from them.
    let boundaries: Vec<Value> = scope
        .linkage
        .iter_names()
        .map(|k| Value::String(k.to_owned()))
        .collect();
    let callees: Vec<Value> = result.unit.ir_module.procedures.keys().copied()
        .filter_map(|id| {
            let evidence = scope.call_sites.get_for_body(&result.unit.ir_module, id)?;
            let procedure = result.unit.ir_module.procedures.get(&id)?;
            let name = &procedure.qualified_name;
            let max_index = evidence.slots.keys().copied().max().map_or(0, |m| m + 1);
            let positions: Vec<Value> = (0..max_index)
                .map(|index| {
                    json!({
                        "index": index.to_string(),
                        "verdict": match evidence.uniform_literal_at(index) {
                            Some(literal) => format!("uniform literal {literal:?}"),
                            None => "not uniform".to_owned(),
                        },
                    })
                })
                .collect();
            let arg_counts: Vec<Value> = evidence
                .arg_counts
                .iter()
                .map(|n| Value::String(n.to_string()))
                .collect();
            Some(json!({ "bodyId": id.ordinal(), "bodyKey": id.reporting_key(), "name": name, "positions": positions, "argCounts": arg_counts }))
        })
        .collect();
    json!({
        "boundaries": boundaries,
        "hasCrossFileEvidence": scope.has_cross_file_evidence,
        "seeding": if scope
            .linkage
            .intersects(tcl_registry::Traits::PROVIDES_PACKAGE | tcl_registry::Traits::EXPORTS_COMMAND)
        {
            "declined — the file publishes commands beyond any enumerable project"
        } else if !scope.has_cross_file_evidence
            && scope.linkage.intersects(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
        {
            "declined — another unit is loaded here and no cross-file view was supplied"
        } else {
            "allowed — evidence is treated as the complete caller set"
        },
        "callees": callees,
    })
}

/// Serialise the `interprocedural` view: per-procedure summaries followed
/// by `TclOO` method summaries.
#[must_use]
pub fn serialise_interproc(
    interproc: &InterproceduralAnalysis,
    param_constants: &BTreeMap<tcl_compiler::ir::ProcedureId, Vec<(String, u32, String)>>,
) -> Value {
    let mut out: Vec<Value> = Vec::new();

    let mut qnames: Vec<_> = interproc.procedures.keys().collect();
    qnames.sort_by(|left, right| {
        interproc.procedures[*left]
            .qualified_name
            .cmp(&interproc.procedures[*right].qualified_name)
            .then(left.cmp(right))
    });
    for qname in qnames {
        let s = &interproc.procedures[qname];
        out.push(json!({
            "bodyId": qname.ordinal(), "bodyKey": qname.reporting_key(),
            "name": s.qualified_name,
            "arity": arity_str(s.arity),
            "pure": s.pure,
            "foldable": s.can_fold_static_calls,
            "returnShape": format_return_shape(s),
            "calls": s.calls.iter().filter_map(|id| interproc.procedures.get(id).or_else(|| interproc.methods.get(id).map(|method| &method.base)).map(|callee| &callee.qualified_name)).collect::<Vec<_>>(),
            "callBodyIds": s.calls.iter().map(|id| id.ordinal()).collect::<Vec<_>>(),
            "callBodyKeys": s.calls.iter().map(|id| id.reporting_key()).collect::<Vec<_>>(),
            "hasBarrier": s.has_barrier,
            "hasUnknownCalls": s.has_unknown_calls,
            "writesGlobal": s.writes_global,
            // What a call does to its caller's places, or nothing where a
            // call to the procedure is a barrier.
            "transfer": interproc.transfers.describe(qname, &s.params),
            // The caller-uniform-literal SCCP seed this procedure was
            // analysed under — the fact that explains a folded condition on
            // a parameter (and, by its absence, an indirect call site the
            // scan could not enumerate).  See
            // `docs/design/compiler/interprocedural-call-site-seeding.md`.
            "paramConstants": param_constants
                .get(qname)
                .map_or_default(|seeds| {
                    seeds
                        .iter()
                        .map(|(param, _version, literal)| format!("{param} = {literal}"))
                        .collect::<Vec<_>>()
                }),
        }));
    }

    let mut methods: Vec<_> = interproc.methods.iter().collect();
    methods.sort_by(|(left_id, left), (right_id, right)| {
        left.base
            .qualified_name
            .cmp(&right.base.qualified_name)
            .then(left_id.cmp(right_id))
    });
    for (id, m) in methods {
        let mut writes_instance: Vec<&String> = m.writes_instance_vars.iter().collect();
        writes_instance.sort();
        out.push(json!({
            "bodyId": id.ordinal(), "bodyKey": id.reporting_key(),
            "name": m.base.qualified_name,
            "kind": "method",
            "methodKind": m.method_kind.as_str(),
            "className": m.class_name,
            "pure": m.base.pure,
            "calls": m.base.calls.iter().filter_map(|id| interproc.procedures.get(id).or_else(|| interproc.methods.get(id).map(|method| &method.base)).map(|callee| &callee.qualified_name)).collect::<Vec<_>>(),
            "callBodyIds": m.base.calls.iter().map(|id| id.ordinal()).collect::<Vec<_>>(),
            "callBodyKeys": m.base.calls.iter().map(|id| id.reporting_key()).collect::<Vec<_>>(),
            "hasBarrier": m.base.has_barrier,
            "hasUnknownCalls": m.base.has_unknown_calls,
            "writesGlobal": m.base.writes_global,
            "writesInstanceVars": writes_instance,
        }));
    }

    Value::Array(out)
}
