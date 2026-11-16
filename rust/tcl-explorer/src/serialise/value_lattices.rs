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

//! The per-value lattice views: SCCP, types, intervals, the taint lattice and rendered properties.

use super::snapshot_body_key;
use crate::ExplorerResult;
use crate::formatters::{
    format_lattice, format_taint, format_type, preview, range_dict, type_kind_name,
};
use serde_json::{Value, json};
use tcl_compiler::intervals::compute_intervals;
use tcl_compiler::shimmer::type_name;
use tcl_lexer::LineIndex;

/// Serialise the `renderedProperties` view: each SSA value's `may` / `must`
/// rendered-property flag names, per function. Values with no flags are
/// skipped, and functions with no entries are omitted. `iter_names()` yields
/// the set named flags in declaration order (NONE excluded).
#[must_use]
pub fn serialise_rendered_properties(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .filter_map(|snap| {
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.rendered_props.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let rp = &snap.unit.rendered_props[*key];
                    let may: Vec<&str> = rp.may.iter_names().map(|(n, _)| n).collect();
                    let must: Vec<&str> = rp.must.iter_names().map(|(n, _)| n).collect();
                    if may.is_empty() && must.is_empty() {
                        return None;
                    }
                    Some(json!({ "variable": ssa.var_name(key.0), "version": key.1, "may": may, "must": must }))
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `intervals` view: the integer-interval domain per tracked
/// SSA value, per function. — only bounded
/// (non-top) ranges are emitted; `lo`/`hi` are `null` for ±infinity.
#[must_use]
pub fn serialise_intervals(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let ssa = &snap.unit.ssa;
            let intervals = compute_intervals(&snap.unit.cfg, ssa, &snap.unit.sccp.values);
            let mut keys: Vec<_> = intervals.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let iv = intervals[*key];
                    if iv.is_top() || (iv.lo.is_none() && iv.hi.is_none()) {
                        return None;
                    }
                    Some(json!({ "variable": ssa.var_name(key.0), "version": key.1, "lo": iv.lo, "hi": iv.hi }))
                })
                .collect();
            json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "entries": entries })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `taintTracking` view: the complete taint lattice for every
/// tracked SSA value, per function. Warning rendering remains separate in
/// `taintWarnings`; this payload is the durable analysis state.
#[must_use]
pub fn serialise_taint_tracking(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .filter_map(|snap| {
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.taints.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .map(|key| {
                    let tl = &snap.unit.taints[*key];
                    json!({ "variable": ssa.var_name(key.0), "version": key.1, "taint": format_taint(tl) })
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// One statement's route record of the SCCP view: the route the resolved
/// invocation declared, how it answered, and what it stores on each
/// completion path.
fn route_json(
    explanation: &tcl_compiler::value_transfer::RouteExplanation,
    li: &LineIndex,
    source: &str,
) -> Value {
    json!({
        "command": explanation.command,
        "route": explanation.route,
        "answer": explanation.answer,
        "paths": explanation
            .paths
            .iter()
            .map(|path| json!({
                "completion": path.completion.label(),
                "stores": path.stores,
            }))
            .collect::<Vec<_>>(),
        "range": range_dict(explanation.span, li, source),
    })
}

/// One selection record of the SCCP view: the statement's range,
/// each kept arm's pattern range, and per member the arm selected and the
/// arm whose body runs — the final `default` pair by that name, since the
/// statement keeps it as its default body — with the count of its writes.
fn selection_json(
    record: &tcl_compiler::sccp::SelectionRecord,
    li: &LineIndex,
    source: &str,
) -> Value {
    let label = |arm: &Option<usize>| match arm {
        Some(arm) if record.is_default(*arm) => "default".to_owned(),
        Some(arm) => format!("arm {arm}"),
        None => "none".to_owned(),
    };
    json!({
        "range": range_dict(record.span, li, source),
        "armPatterns": record
            .arm_pattern_spans
            .iter()
            .map(|span| range_dict(*span, li, source))
            .collect::<Vec<_>>(),
        "selected": record.fact.selected.iter().map(label).collect::<Vec<_>>(),
        "bodies": record.fact.bodies.iter().map(label).collect::<Vec<_>>(),
        "writes": record.fact.writes.iter().map(Vec::len).collect::<Vec<_>>(),
    })
}

/// One loop the solver ran to its exit: where it leaves to, its passes, how
/// it left, and the value it published for each place it wrote.
fn enumerated_loop_json(
    record: &tcl_compiler::sccp::EnumeratedLoop,
    ssa: &tcl_compiler::ssa::SsaFunction,
    (li, source): (&LineIndex, &str),
) -> Value {
    use tcl_registry::value_transfer::ExitRule;
    let exit = match record.exit {
        ExitRule::Exhaustion => "exhaustion",
        ExitRule::FalseCondition => "false condition",
        ExitRule::Break => "break",
        ExitRule::NonNormalCompletion => "non-normal completion",
        ExitRule::EnumerationCap => "enumeration cap",
    };
    json!({
        "exitBlock": ssa.block_name(record.exit_block),
        "iterations": record.iterations,
        "exit": exit,
        "range": range_dict(record.span, li, source),
        "published": record
            .published
            .iter()
            .map(|(name, value)| json!({ "variable": name, "lattice": format_lattice(value) }))
            .collect::<Vec<_>>(),
    })
}

/// One edge refinement of the SCCP view: the edge, the place and version it
/// narrows, the domain, and the fact as the view spells it — a value as a
/// constant is (`'a'`), a finite set as its members, an existence, a type
/// or a range by name.
fn refinement_json(
    refinement: &tcl_compiler::sccp::EdgeRefinement,
    sccp: &tcl_compiler::sccp::SccpResult,
    ssa: &tcl_compiler::ssa::SsaFunction,
) -> Value {
    use tcl_registry::value_transfer::{BindingKind, DomainFact, Existence, FactDomain, FactView};
    let text = |bytes: &[u8]| crate::formatters::py_repr_str(&String::from_utf8_lossy(bytes));
    let fact = match &refinement.fact {
        FactView::Exact(value, _) => text(&value.bytes),
        FactView::Finite(values, _) => format!(
            "one of {}",
            values
                .iter()
                .map(|value| text(&value.bytes))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        FactView::Domain(DomainFact::Existence(existence)) => match existence {
            Existence::Bound(BindingKind::Either) => "bound".to_owned(),
            Existence::Bound(BindingKind::Scalar) => "bound scalar".to_owned(),
            Existence::Bound(BindingKind::Array) => "bound array".to_owned(),
            Existence::Unbound => "unbound".to_owned(),
            Existence::MayBound => "may be bound".to_owned(),
            Existence::Pending => "pending".to_owned(),
        },
        FactView::Domain(DomainFact::Type {
            intrep: Some(intrep),
            ..
        }) => format!("type {}", type_name(*intrep)),
        FactView::Domain(DomainFact::Range { lo, hi }) => {
            let bound = |bound: &Option<i64>, infinite: &str| {
                bound.map_or_else(|| infinite.to_owned(), |bound| bound.to_string())
            };
            format!("range [{}, {}]", bound(lo, "-inf"), bound(hi, "+inf"))
        }
        other => format!("{other:?}"),
    };
    let domain = match refinement.domain {
        FactDomain::ExactValue => "value",
        FactDomain::Type => "type",
        FactDomain::Existence => "existence",
        FactDomain::Range => "range",
        FactDomain::Segments => "segments",
        _ => "other",
    };
    json!({
        "from": ssa.block_name(refinement.edge.0),
        "to": ssa.block_name(refinement.edge.1),
        "variable": sccp.place_name(ssa, refinement.key.0).unwrap_or("?"),
        "version": refinement.key.1,
        "domain": domain,
        "fact": fact,
    })
}

/// The SCCP view's values of one function, by variable and version: each
/// version's lattice value, with the folded type the producing evaluation
/// states when it states one — the type with how the route built the value
/// (`bytearray (constructed)`).
fn lattice_values_json(unit: &tcl_compiler::compilation_unit::FunctionUnit) -> Vec<Value> {
    let ssa = &unit.ssa;
    let mut values: Vec<_> = unit.sccp.values.iter().collect();
    values.sort_by(|(a, _), (b, _)| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
    let folded = &unit.sccp.folded_types;
    values
        .into_iter()
        .map(|(&(symbol, version), lattice)| {
            let mut value = json!({
                "variable": ssa.var_name(symbol),
                "version": version,
                "lattice": format_lattice(lattice),
            });
            if let Some(folded) = folded.get(&(symbol, version)) {
                value["type"] = json!(folded.label());
            }
            value
        })
        .collect()
}

/// Serialise the complete SCCP lattice and executable CFG facts. The SSA CFG
/// tab intentionally keeps a compact annotation; this view is the durable
/// proof surface for constants, reachability, executable edges, each
/// executable branch edge's refinements, each loop the solver ran to its
/// exit, each opaque case list's selection and — per statement — the
/// value-transfer route the resolved invocation declared and how it
/// answered.
#[must_use]
pub fn serialise_sccp(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let ssa = &snap.unit.ssa;
                let values = lattice_values_json(snap.unit);
                let mut executable_blocks: Vec<String> = snap
                    .unit
                    .sccp
                    .executable_blocks
                    .iter()
                    .map(|id| ssa.block_name(*id).to_owned())
                    .collect();
                executable_blocks.sort_unstable();
                let mut executable_edges: Vec<Value> = snap
                    .unit
                    .sccp
                    .executable_edges
                    .iter()
                    .map(|(from, to)| json!({ "from": ssa.block_name(*from), "to": ssa.block_name(*to) }))
                    .collect();
                executable_edges.sort_by_key(std::string::ToString::to_string);
                let branches: Vec<Value> = snap
                    .unit
                    .sccp
                    .constant_branches
                    .iter()
                    .map(|branch| json!({
                        "block": branch.block,
                        "kind": branch.kind.label(),
                        "condition": preview(&branch.condition, 80),
                        "value": branch.value,
                        "takenTarget": branch.taken_target,
                        "notTakenTarget": branch.not_taken_target,
                        "range": branch.span.map(|span| range_dict(span, li, source)),
                    }))
                    .collect();
                let routes: Vec<Value> = snap
                    .unit
                    .sccp
                    .explanations
                    .iter()
                    .map(|explanation| route_json(explanation, li, source))
                    .collect();
                let selections: Vec<Value> = snap
                    .unit
                    .sccp
                    .selections
                    .iter()
                    .map(|record| selection_json(record, li, source))
                    .collect();
                // A refinement on an edge the solver never takes holds
                // nowhere, so the view names only the executable edges'.
                let refinements: Vec<Value> = snap
                    .unit
                    .sccp
                    .refinements
                    .iter()
                    .filter(|refinement| {
                        snap.unit.sccp.executable_edges.contains(&refinement.edge)
                    })
                    .map(|refinement| refinement_json(refinement, &snap.unit.sccp, ssa))
                    .collect();
                let loops: Vec<Value> = snap
                    .unit
                    .sccp
                    .loop_enumerations
                    .iter()
                    .map(|record| enumerated_loop_json(record, ssa, (li, source)))
                    .collect();
                let tally = &snap.unit.sccp.route_tally;
                json!({
                    "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap),
                    "kind": snap.kind.as_str(),
                    "values": values,
                    "executableBlocks": executable_blocks,
                    "executableEdges": executable_edges,
                    "constantBranches": branches,
                    "refinements": refinements,
                    "enumeratedLoops": loops,
                    "selections": selections,
                    "routes": routes,
                    "routeTally": {
                        "direct": tally.direct,
                        "expression": tally.expression,
                        "implementation": tally.implementation,
                    },
                })
            })
            .collect(),
    )
}

/// Serialise the `types` view: every tracked SSA value's lattice type per
/// function, including `?`/`*` entries.
#[must_use]
pub fn serialise_types(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .filter_map(|snap| {
            let mut entries: Vec<Value> = Vec::new();
            let rt = &snap.unit.return_type;
            entries.push(json!({
                "variable": "(return)",
                "version": 0,
                "type": format_type(rt),
                "kind": type_kind_name(rt.kind()),
            }));
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.types.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            for key in keys {
                let tl = &snap.unit.types[key];
                entries.push(json!({
                    "variable": ssa.var_name(key.0),
                    "version": key.1,
                    "type": format_type(tl),
                    "kind": type_kind_name(tl.kind()),
                }));
            }
            (!entries.is_empty()).then(
                || json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "entries": entries }),
            )
        })
        .collect();
    Value::Array(funcs)
}
