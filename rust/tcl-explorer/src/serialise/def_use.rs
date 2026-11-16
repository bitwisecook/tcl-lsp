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

//! The def-use views: liveness, the data-flow graph and memory SSA.

use super::{retained_explorer_source, snapshot_body_key};
use crate::ExplorerResult;
use serde_json::{Value, json};
use tcl_compiler::dataflow_graph::extract_function_dataflow;
use tcl_compiler::memory_ssa::{MemoryLocationKind, MemoryOpKind, MemorySsaFunction};

/// Serialise def-use/liveness evidence without re-deriving it in the
/// front-end. Every chain is retained, including dead definitions and their
/// exact use-site kinds; the O109 collector is included as the optimiser's
/// authoritative dead-store result.
#[must_use]
pub fn serialise_liveness(result: &ExplorerResult) -> Value {
    let Some(retained) = retained_explorer_source(result, &result.source) else {
        return Value::Null;
    };
    let registry = retained.registry;
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let mut chains: Vec<_> = snap.unit.def_use.chains.values().collect();
                let report_label = |symbol| {
                    snap.unit.def_use.cell_key(&snap.unit.ssa, symbol)
                        .map(tcl_compiler::var_resolve::VariableCellKey::compatibility_name)
                        .or_else(|| {
                            (snap.unit.ssa.cell_key(symbol).is_none()
                                && snap.unit.def_use.source_variable_symbol(&snap.unit.ssa, symbol).is_some())
                                .then(|| snap.unit.ssa.var_name(symbol).to_owned())
                        })
                };
                chains.sort_by_key(|chain| (report_label(chain.key.0), chain.key.1));
                let chains: Vec<Value> = chains
                    .iter()
                    .map(|chain| {
                        json!({
                            "variable": report_label(chain.key.0),
                            "version": chain.key.1,
                            "definition": {
                                "block": chain.definition.block,
                                "kind": def_kind_label(chain.definition.kind),
                                "statementIndex": chain.definition.statement_index,
                            },
                            "uses": chain.uses.iter().map(|use_site| json!({
                                "block": use_site.block,
                                "kind": use_kind_label(use_site.kind),
                                "statementIndex": use_site.statement_index,
                                "class": use_class_label(use_site.class),
                            })).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                let dead_stores: Vec<Value> =
                    tcl_compiler::dead_stores::liveness_dead_stores(snap.unit, registry)
                        .iter()
                        .map(|dead| {
                            json!({
                                "block": dead.block,
                                "statementIndex": dead.statement_index,
                                "variable": dead.variable,
                                "version": dead.version,
                            })
                        })
                        .collect();
                json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "chains": chains, "deadStores": dead_stores })
            })
            .collect(),
    )
}

fn def_kind_label(kind: tcl_compiler::def_use::DefKind) -> &'static str {
    match kind {
        tcl_compiler::def_use::DefKind::Statement => "statement",
        tcl_compiler::def_use::DefKind::Phi => "phi",
        tcl_compiler::def_use::DefKind::Parameter => "parameter",
    }
}

fn use_kind_label(kind: tcl_compiler::def_use::UseKind) -> &'static str {
    match kind {
        tcl_compiler::def_use::UseKind::Operand => "operand",
        tcl_compiler::def_use::UseKind::VariableName => "variable-name",
        tcl_compiler::def_use::UseKind::PhiIncoming => "phi-incoming",
        tcl_compiler::def_use::UseKind::Terminator => "terminator",
    }
}

fn use_class_label(class: tcl_compiler::ssa::UseClass) -> &'static str {
    match class {
        tcl_compiler::ssa::UseClass::Substituted => "substituted",
        tcl_compiler::ssa::UseClass::Quoted => "quoted",
        tcl_compiler::ssa::UseClass::Name => "name",
    }
}

/// Serialise the `dataflow` view: the def-use data-flow graph, built over
/// `extract_dataflow_graph`.
///
/// Each graph stays paired with its stable function-owner kind, and alias
/// information comes from the retained Memory SSA when available.
#[must_use]
pub fn serialise_dataflow(result: &ExplorerResult) -> Value {
    let snaps = result.all_snapshots();
    let mut total_defs = 0u32;
    let mut total_uses = 0u32;
    let mut total_aliases = 0u32;
    let functions: Vec<Value> = snaps
        .iter()
        .map(|snap| {
            // Extract each graph beside its owner identity. Building a module
            // graph would sort only by display name and could pair a Tcl proc
            // with a same-named TclOO method's Memory SSA.
            let f = extract_function_dataflow(
                snap.name,
                &snap.unit.ssa,
                &snap.unit.def_use,
                Some(&snap.unit.sccp),
                snap.unit.memory_ssa.as_ref(),
                Some(&snap.unit.types),
            );
            total_defs = total_defs.saturating_add(f.total_defs);
            total_uses = total_uses.saturating_add(f.total_uses);
            total_aliases =
                total_aliases.saturating_add(u32::try_from(f.aliases.len()).unwrap_or(u32::MAX));
            serialise_dataflow_function(result, snap, &f)
        })
        .collect();

    json!({
        "functions": functions,
        "summary": {
            "totalDefs": total_defs,
            "totalUses": total_uses,
            "totalAliases": total_aliases,
            "functionCount": snaps.len(),
        },
    })
}

fn serialise_dataflow_function(
    result: &ExplorerResult,
    snap: &crate::FunctionSnapshot<'_>,
    f: &tcl_compiler::dataflow_graph::FunctionDataFlowGraph,
) -> Value {
    let memory = snap
        .unit
        .memory_ssa
        .as_ref()
        .map_or(Value::Null, serialise_memory_ssa);
    let nodes: Vec<Value> = f
        .nodes
        .iter()
        .map(|n| {
            json!({
                "name": n.name,
                "symbol": n.symbol.map(|symbol| symbol.0),
                "cell": n.cell,
                "version": n.version,
                "block": n.block,
                "defKind": n.def_kind,
                "statementIndex": n.statement_index,
                "lattice": n.lattice,
                "typeInfo": n.type_info,
                "isDead": n.is_dead,
                "useCount": n.use_count,
            })
        })
        .collect();
    let edges: Vec<Value> = f
        .edges
        .iter()
        .map(|e| {
            json!({
                "fromName": e.from_name,
                "fromSymbol": e.from_symbol.map(|symbol| symbol.0),
                "fromCell": e.from_cell,
                "fromVersion": e.from_version,
                "toBlock": e.to_block,
                "toStatementIndex": e.to_statement_index,
                "edgeKind": e.edge_kind.as_str(),
                "toName": e.to_name,
                "toSymbol": e.to_symbol.map(|symbol| symbol.0),
                "toCell": e.to_cell,
                "toVersion": e.to_version,
            })
        })
        .collect();
    let aliases: Vec<Value> = f
        .aliases
        .iter()
        .map(|a| {
            json!({
                "localName": a.local_name,
                "localKind": a.local_kind,
                "targetName": a.target_name,
                "targetKind": a.target_kind,
                "reason": a.reason,
            })
        })
        .collect();
    json!({
        "name": f.function_name,
        "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap),
        "kind": snap.kind.as_str(),
        "nodes": nodes,
        "edges": edges,
        "aliases": aliases,
        "memorySsa": memory,
        "summary": {
            "totalDefs": f.total_defs,
            "totalUses": f.total_uses,
            "deadDefs": f.dead_defs,
            "aliasedVars": f.aliased_vars,
        },
    })
}

fn memory_location_kind_label(kind: MemoryLocationKind) -> &'static str {
    match kind {
        MemoryLocationKind::Local => "local",
        MemoryLocationKind::Upvar => "upvar",
        MemoryLocationKind::Global => "global",
        MemoryLocationKind::NamespaceVar => "namespace-var",
        MemoryLocationKind::ArrayElement => "array-element",
        MemoryLocationKind::InstanceVar => "instance-var",
        MemoryLocationKind::Unknown => "unknown",
    }
}

fn memory_op_kind_label(kind: MemoryOpKind) -> &'static str {
    match kind {
        MemoryOpKind::Def => "def",
        MemoryOpKind::Use => "use",
        MemoryOpKind::Phi => "phi",
        MemoryOpKind::Clobber => "clobber",
    }
}

fn serialise_memory_ssa(memory: &MemorySsaFunction) -> Value {
    let alias_sets: Vec<Value> = memory
        .alias_sets
        .iter()
        .map(|set| {
            let locations: Vec<Value> = set
                .locations
                .iter()
                .map(|location| {
                    json!({
                        "kind": memory_location_kind_label(location.kind),
                        "name": location.name,
                        "qualifier": location.qualifier,
                    })
                })
                .collect();
            json!({ "reason": set.reason, "locations": locations })
        })
        .collect();
    let ops: Vec<Value> = memory
        .memory_ops
        .iter()
        .map(|op| {
            json!({
                "kind": memory_op_kind_label(op.kind),
                "location": {
                    "kind": memory_location_kind_label(op.location.kind),
                    "name": op.location.name,
                    "qualifier": op.location.qualifier,
                },
                "version": op.version,
                "reachingVersion": op.reaching_version,
                "block": op.block,
                "statementIndex": op.statement_index,
            })
        })
        .collect();
    json!({
        "aliasSets": alias_sets,
        "operations": ops,
        "counts": {
            "defs": memory.count_defs,
            "uses": memory.count_uses,
            "clobbers": memory.count_clobbers,
        },
        "wildcardAliasing": memory.has_wildcard_aliasing,
    })
}
