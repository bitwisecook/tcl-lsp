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

//! The control-flow views: the pre-SSA and post-SSA graphs, their routed edges, the dominator artefacts and the loop forest.

use super::{range_or_null, retained_explorer_source, snapshot_body_key};
use crate::ExplorerResult;
use crate::formatters::{
    format_lattice, format_type, preview, range_dict, stmt_color_class, stmt_summary,
};
use serde_json::{Map, Value, json};
use tcl_compiler::cfg::{Function, Terminator};
use tcl_compiler::cfg_layout::{build_cfg_edges, ordered_block_names};
use tcl_compiler::loops::{build_loop_forest, dominates};
use tcl_lexer::LineIndex;
use tcl_syntax::expr::ast::render_expr;

/// Serialise a block terminator.
pub(super) fn terminator_dict(
    func: &Function,
    term: Option<&Terminator>,
    li: &LineIndex,
    source: &str,
) -> Value {
    match term {
        None => Value::Null,
        Some(Terminator::Goto { target, span }) => json!({
            "type": "goto",
            "target": func.block_name(*target),
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span,
            ..
        }) => json!({
            "type": "branch",
            "condition": preview(&render_expr(condition), 80),
            "trueTarget": func.block_name(*true_target),
            "falseTarget": func.block_name(*false_target),
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Complete { route, span }) => json!({
            "type": "complete",
            "completion": completion_route_dict(*route),
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Return { value, span, .. }) => json!({
            "type": "return",
            "value": value.as_ref().map(|v| preview(v, 60)),
            "range": range_or_null(*span, li, source),
        }),
    }
}

fn completion_route_dict(
    route: tcl_registry::completion_route::InvocationCompletionRoute,
) -> Value {
    use tcl_registry::completion_route::InvocationCompletionRoute as Route;
    let detail = match route {
        Route::Tcl(code) => json!({"kind": "tcl", "code": code.as_int()}),
        Route::TclAlternatives(codes) => {
            json!({"kind": "tcl-alternatives", "codes": codes.iter().map(|code| code.as_int()).collect::<Vec<_>>()})
        }
        Route::Return(pending) => {
            json!({"kind": "return", "eventualCode": pending.eventual_code.as_int(), "remainingLevel": pending.remaining_level})
        }
        Route::Tailcall { code } => json!({"kind": "tailcall", "code": code.as_int()}),
        Route::TailcallOrError { code } => {
            json!({"kind": "tailcall-or-error", "code": code.as_int()})
        }
        Route::ProcessExit => json!({"kind": "process-exit"}),
        Route::ReturnOrError => json!({"kind": "return-or-error"}),
        Route::ExitOrError => json!({"kind": "exit-or-error"}),
        Route::CatchableExitOrError => json!({"kind": "catchable-exit-or-error"}),
        Route::Unknown => json!({"kind": "unknown"}),
        Route::UnknownAbrupt => json!({"kind": "unknown-abrupt"}),
    };
    json!({"route": detail, "normalPossible": route.normal_possible(), "abruptPossible": route.abrupt_possible()})
}

/// The successor block names of a terminator (terminator-only, no
/// exception edges).
pub(super) fn block_successors<'a>(func: &'a Function, term: Option<&Terminator>) -> Vec<&'a str> {
    term.map_or_default(Terminator::successors)
        .into_iter()
        .map(|id| func.block_name(id))
        .collect()
}

/// Serialise the routed control-flow edges of `func`. Lanes come from the
/// shared `cfg_layout`.
fn serialise_cfg_edges(func: &Function, order: &[String]) -> Value {
    let edges: Vec<Value> = build_cfg_edges(func, order)
        .into_iter()
        .map(|e| {
            json!({
                "from": e.src,
                "to": e.dst,
                "fromPos": e.src_pos,
                "toPos": e.dst_pos,
                "kind": e.kind.as_str(),
                "lane": e.lane,
            })
        })
        .collect();
    Value::Array(edges)
}

/// Serialise the pre-SSA CFG view.:
/// one entry per function, blocks in creation order, with routed edges.
#[must_use]
pub fn serialise_cfg_pre_ssa(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let cfg = &snap.unit.cfg;
            let order = ordered_block_names(cfg);
            let entry_name = cfg.block_name(cfg.entry);
            let blocks: Vec<Value> = order
                .iter()
                .map(|bn| {
                    let block = cfg.block_by_name(bn).expect("ordered block exists");
                    let statements: Vec<Value> = block
                        .statements
                        .iter()
                        .map(|stmt| {
                            json!({
                                "summary": stmt_summary(stmt),
                                "colorClass": stmt_color_class(stmt),
                                "range": range_dict(stmt.span(), li, source),
                            })
                        })
                        .collect();
                    json!({
                        "name": bn,
                        "isEntry": bn == entry_name,
                        "statements": statements,
                        "terminator": terminator_dict(cfg, block.terminator.as_ref(), li, source),
                        "successors": block_successors(cfg, block.terminator.as_ref()),
                    })
                })
                .collect();
            json!({
                "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap),
                "kind": snap.kind.as_str(),
                "entry": entry_name,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// Per-SSA-value `{version, lattice?, type?}` detail used by the post-SSA
/// CFG view's `uses`/`defs` maps.
fn ssa_value_detail(
    refs: &std::collections::HashMap<tcl_compiler::ssa::Symbol, tcl_compiler::ssa::Version>,
    sccp: &tcl_compiler::sccp::SccpResult,
    types: &std::collections::HashMap<
        tcl_compiler::ssa::ValueKey,
        tcl_compiler::types::TypeLattice,
    >,
    ssa: &tcl_compiler::ssa::SsaFunction,
) -> Value {
    let mut keys: Vec<(&tcl_compiler::ssa::Symbol, &u32)> = refs.iter().collect();
    keys.sort_by(|a, b| {
        ssa.var_name(*a.0)
            .cmp(ssa.var_name(*b.0))
            .then(a.1.cmp(b.1))
    });
    let mut out = Map::new();
    for (&sym, &ver) in keys {
        let mut d = Map::new();
        d.insert("version".to_owned(), json!(ver));
        if let Some(lat) = sccp.values.get(&(sym, ver)) {
            d.insert("lattice".to_owned(), json!(format_lattice(lat)));
        }
        if let Some(tl) = types.get(&(sym, ver))
            && tl.kind() != tcl_compiler::types::TypeKind::Unknown
        {
            d.insert("type".to_owned(), json!(format_type(tl)));
        }
        out.insert(ssa.var_name(sym).to_owned(), Value::Object(d));
    }
    Value::Object(out)
}

/// Serialise the post-SSA CFG view (`cfgPostSsa`): per-block phi nodes and
/// per-statement SSA `uses`/`defs` with lattice + type detail, plus a
/// function-level `analysis` block.
///
/// `analysis.deadStores` is the per-function liveness-based set
/// ([`tcl_compiler::dead_stores::liveness_dead_stores`]). The lattice/type
/// detail comes from the Rust
/// analyses; `constantBranches`/`unreachableBlocks`/`inferredTypes` come from
/// SCCP + the type lattice.
#[must_use]
pub fn serialise_cfg_post_ssa(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let registry = retained.registry;
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let cfg = &snap.unit.cfg;
            let ssa = &snap.unit.ssa;
            let sccp = &snap.unit.sccp;
            let types = &snap.unit.types;
            let order = ordered_block_names(cfg);
            let entry_name = cfg.block_name(cfg.entry);

            let blocks: Vec<Value> = order
                .iter()
                .map(|bn| {
                    let block = cfg.block_by_name(bn).expect("ordered block exists");
                    let bid = cfg.block_id(bn);
                    let ssa_block = bid.and_then(|id| ssa.blocks.get(&id));

                    let phis: Vec<Value> = ssa_block
                        .map_or_default(|sb| {
                            sb.phis
                                .iter()
                                .map(|phi| {
                                    // Key the incoming map by predecessor block
                                    // name; sort by name for stable output.
                                    let mut inc: Vec<(&str, u32)> = phi
                                        .incoming
                                        .iter()
                                        .map(|(pred, &ver)| (cfg.block_name(*pred), ver))
                                        .collect();
                                    inc.sort_unstable();
                                    let phi_name = ssa.var_name(phi.name);
                                    let incoming: Map<String, Value> = inc
                                        .into_iter()
                                        .map(|(pred, ver)| {
                                            (pred.to_owned(), json!(format!("{phi_name}#{ver}")))
                                        })
                                        .collect();
                                    let phi_type = types
                                        .get(&(phi.name, phi.version))
                                        .map_or(Value::Null, |tl| json!(format_type(tl)));
                                    json!({
                                        "name": phi_name,
                                        "version": phi.version,
                                        "incoming": incoming,
                                        "type": phi_type,
                                    })
                                })
                                .collect()
                        });

                    let statements: Vec<Value> = block
                        .statements
                        .iter()
                        .enumerate()
                        .map(|(idx, stmt)| {
                            let (uses, defs) =
                                ssa_block.and_then(|sb| sb.statements.get(idx)).map_or_else(
                                    || (json!({}), json!({})),
                                    |ss| {
                                        (
                                            ssa_value_detail(&ss.uses, sccp, types, ssa),
                                            ssa_value_detail(&ss.defs, sccp, types, ssa),
                                        )
                                    },
                                );
                            json!({
                                "summary": stmt_summary(stmt),
                                "colorClass": stmt_color_class(stmt),
                                "range": range_dict(stmt.span(), li, source),
                                "uses": uses,
                                "defs": defs,
                            })
                        })
                        .collect();

                    json!({
                        "name": bn,
                        "isEntry": bn == entry_name,
                        "isUnreachable": !bid.is_some_and(|id| sccp.executable_blocks.contains(&id)),
                        "phis": phis,
                        "statements": statements,
                        "terminator": terminator_dict(cfg, block.terminator.as_ref(), li, source),
                        "successors": block_successors(cfg, block.terminator.as_ref()),
                    })
                })
                .collect();

            json!({
                "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap),
                "kind": snap.kind.as_str(),
                "entry": entry_name,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
                "analysis": post_ssa_analysis(snap, registry),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the durable SSA dominator artefacts: immediate dominators,
/// dominance frontiers, and the materialised dominator tree.  Unlike the
/// post-SSA CFG summary, this retains the complete graph used by phi
/// placement and loop analysis, including method and synthetic body units.
#[must_use]
pub fn serialise_dominators(result: &ExplorerResult) -> Value {
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let ssa = &snap.unit.ssa;
                let mut blocks: Vec<_> = ssa.idom.keys().copied().collect();
                blocks.sort_unstable();
                let rows: Vec<Value> = blocks
                    .iter()
                    .map(|&block| {
                        let idom = ssa.idom.get(&block).and_then(|parent| {
                            parent.map(|parent| ssa.block_name(parent).to_owned())
                        });
                        let mut frontier: Vec<String> = ssa
                            .dominance_frontier
                            .get(&block)
                            .into_iter()
                            .flat_map(|ids| ids.iter().map(|id| ssa.block_name(*id).to_owned()))
                            .collect();
                        frontier.sort_unstable();
                        let mut children: Vec<String> = ssa
                            .dominator_tree
                            .get(&block)
                            .into_iter()
                            .flat_map(|ids| ids.iter().map(|id| ssa.block_name(*id).to_owned()))
                            .collect();
                        children.sort_unstable();
                        json!({
                            "block": ssa.block_name(block),
                            "idom": idom,
                            "frontier": frontier,
                            "children": children,
                        })
                    })
                    .collect();
                json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "entry": ssa.block_name(ssa.entry), "blocks": rows })
            })
            .collect(),
    )
}

/// The function-level `analysis` block of the post-SSA CFG view.
///
/// `deadStores` is the per-function liveness-based set
/// ([`tcl_compiler::dead_stores::liveness_dead_stores`]).
fn post_ssa_analysis(
    snap: &crate::FunctionSnapshot,
    registry: &tcl_registry::CommandRegistry,
) -> Value {
    let sccp = &snap.unit.sccp;
    let constant_branches: Vec<Value> = sccp
        .constant_branches
        .iter()
        .map(|b| {
            json!({
                "block": b.block,
                "kind": b.kind.label(),
                "condition": preview(&b.condition, 60),
                "value": b.value,
                "takenTarget": b.taken_target,
                "notTakenTarget": b.not_taken_target,
            })
        })
        .collect();

    let cfg = &snap.unit.cfg;
    let mut unreachable: Vec<&str> = cfg
        .blocks
        .keys()
        .filter(|b| !sccp.executable_blocks.contains(*b))
        .map(|id| cfg.block_name(*id))
        .collect();
    unreachable.sort_unstable();

    // `inferredTypes`: known/shimmered SSA-value types keyed by `name#ver`.
    let ssa = &snap.unit.ssa;
    let mut tkeys: Vec<_> = snap.unit.types.keys().collect();
    tkeys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
    let mut inferred = Map::new();
    for key in tkeys {
        let tl = &snap.unit.types[key];
        if matches!(
            tl.kind(),
            tcl_compiler::types::TypeKind::Known | tcl_compiler::types::TypeKind::Shimmered
        ) {
            inferred.insert(
                format!("{}#{}", ssa.var_name(key.0), key.1),
                json!(format_type(tl)),
            );
        }
    }

    // Liveness-based dead stores for this function, already in deterministic
    // block/statement order.
    let dead_store_values: Vec<Value> =
        tcl_compiler::dead_stores::liveness_dead_stores(snap.unit, registry)
            .iter()
            .map(|d| {
                json!({
                    "block": d.block,
                    "stmtIndex": d.statement_index,
                    "variable": d.variable,
                    "version": d.version,
                })
            })
            .collect();

    json!({
        "constantBranches": constant_branches,
        "deadStores": dead_store_values,
        "unreachableBlocks": unreachable,
        "inferredTypes": Value::Object(inferred),
    })
}

/// Serialise the `loops` view: the natural-loop forest per function, with
/// nesting depth, built over
/// `build_loop_forest`. Depth = 1 + the number of *other* loop headers
/// that dominate this loop's header.
#[must_use]
pub fn serialise_loops(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let executable = &snap.unit.sccp.executable_blocks;
            let forest = build_loop_forest(&snap.unit.cfg, &snap.unit.ssa, executable);
            let headers = forest.headers();
            let cfg = &snap.unit.cfg;
            let loops: Vec<Value> = forest
                .loops
                .iter()
                .map(|lp| {
                    // Loop nesting depth: count header loops whose header
                    // dominates this loop's header. `NaturalLoop` headers are
                    // block names; resolve each to its id for `dominates`.
                    let node = cfg.block_id(&lp.header);
                    let depth = 1 + headers
                        .iter()
                        .filter(|h| **h != lp.header)
                        .filter_map(|h| Some((cfg.block_id(h)?, node?)))
                        .filter(|&(dom, node)| dominates(&snap.unit.ssa, dom, node))
                        .count();
                    json!({
                        "header": lp.header,
                        "depth": depth,
                        "blockCount": lp.blocks.len(),
                        "blocks": lp.blocks,
                        "latches": lp.latches,
                    })
                })
                .collect();
            json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "loops": loops })
        })
        .collect();
    Value::Array(funcs)
}
