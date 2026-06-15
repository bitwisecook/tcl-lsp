//! JSON serialisation of an [`ExplorerResult`] into the explorer contract
//! shape (`docs/design/contracts/wasm-explorer-view.md` for the `wasm`
//! slice; the rest is the de-facto contract `explorer-core.js` reads).
//!
//! Faithful port of `tooling/cli/serialise.py`, brought up one view-family
//! at a time (EXP-1b..N). Each `serialise_*` helper mirrors the matching
//! Python `_serialise_*` and is verified against it by the differential
//! parity test. `serialise_result` assembles the top-level object; keys
//! not yet ported are simply absent (the parity harness compares only the
//! keys present on both sides as families land).

use serde_json::{Map, Value, json};

use tcl_compiler::cfg::{Function, Terminator};
use tcl_compiler::cfg_layout::{build_cfg_edges, ordered_block_names};
use tcl_compiler::dataflow_graph::{FunctionInputs, extract_dataflow_graph};
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::interprocedural::InterproceduralAnalysis;
use tcl_compiler::interval_bounds::find_interval_bounds;
use tcl_compiler::intervals::compute_intervals;
use tcl_compiler::ir::{Module, Script, Statement};
use tcl_compiler::irules_checks::{
    find_collect_flow_warnings, find_hoistable_set_warnings, find_http_flow_warnings,
    find_unguarded_drop_warnings, find_unnormalised_getter_warnings,
};
use tcl_compiler::loops::{build_loop_forest, dominates};
use tcl_compiler::optimiser::{apply_optimisations, optimise};
use tcl_compiler::segmenter::segment_commands;
use tcl_compiler::shimmer::{
    find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu, type_name,
};
use tcl_compiler::taint::find_taint_warnings_for_cu;
use tcl_lexer::{LineIndex, Span, TokenType};
use tcl_registry::{available_dialects, registry_for_dialect};
use tcl_syntax::expr::ast::render_expr;

use crate::ExplorerResult;
use crate::formatters::{
    format_lattice, format_return_shape, format_taint, format_type, preview, range_dict,
    stmt_color_class, stmt_kind, stmt_summary, type_kind_name,
};
use crate::views::{Severity, VIEW_META};

/// Serialise the `meta` view: dialect list, view-tab table, and the
/// severity vocabulary. Mirrors `_serialise_meta`.
#[must_use]
pub fn serialise_meta() -> Value {
    let dialects: Vec<Value> = available_dialects()
        .iter()
        .map(|d| Value::String((*d).to_owned()))
        .collect();
    let views: Vec<Value> = VIEW_META
        .iter()
        .map(|&(id, label, group)| json!({ "id": id, "label": label, "group": group }))
        .collect();
    let severities: Vec<Value> = Severity::ALL
        .iter()
        .map(|s| Value::String(s.as_str().to_owned()))
        .collect();
    json!({
        "dialects": dialects,
        "views": views,
        "severities": severities,
    })
}

/// `range_dict` for an optional span, emitting `null` when absent
/// (mirrors `range_dict(r) if r else None`).
fn range_or_null(span: Option<Span>, li: &LineIndex, source: &str) -> Value {
    span.map_or(Value::Null, |s| range_dict(s, li, source))
}

/// Serialise an IR script (a list of statement nodes). Mirrors
/// `_serialise_script`; children are emitted only for If/For/Switch, as on
/// the Python side.
fn serialise_script(script: &Script, li: &LineIndex, source: &str) -> Value {
    let nodes: Vec<Value> = script
        .statements
        .iter()
        .map(|stmt| {
            let mut node = json!({
                "kind": stmt_kind(stmt),
                "summary": stmt_summary(stmt),
                "colorClass": stmt_color_class(stmt),
                "range": range_dict(stmt.span(), li, source),
            });
            if let Some(children) = serialise_children(stmt, li, source) {
                node["children"] = Value::Array(children);
            }
            node
        })
        .collect();
    Value::Array(nodes)
}

/// The `children` array for the structured statements the IR view expands
/// (If/For/Switch); `None` for every other statement kind.
fn serialise_children(stmt: &Statement, li: &LineIndex, source: &str) -> Option<Vec<Value>> {
    match stmt {
        Statement::If {
            clauses,
            else_body,
            else_span,
            ..
        } => {
            let mut children: Vec<Value> = clauses
                .iter()
                .enumerate()
                .map(|(i, clause)| {
                    json!({
                        "label": format!("clause {}: {}", i + 1, preview(&render_expr(&clause.condition), 60)),
                        "range": range_dict(clause.condition_span, li, source),
                        "body": serialise_script(&clause.body, li, source),
                    })
                })
                .collect();
            if let Some(body) = else_body {
                children.push(json!({
                    "label": "else",
                    "range": range_or_null(*else_span, li, source),
                    "body": serialise_script(body, li, source),
                }));
            }
            Some(children)
        }
        Statement::For {
            init,
            init_span,
            condition,
            condition_span,
            next,
            next_span,
            body,
            body_span,
            ..
        } => Some(vec![
            json!({ "label": "init", "range": range_dict(*init_span, li, source), "body": serialise_script(init, li, source) }),
            json!({
                "label": format!("condition: {}", preview(&render_expr(condition), 60)),
                "range": range_dict(*condition_span, li, source),
                "body": [],
            }),
            json!({ "label": "next", "range": range_dict(*next_span, li, source), "body": serialise_script(next, li, source) }),
            json!({ "label": "body", "range": range_dict(*body_span, li, source), "body": serialise_script(body, li, source) }),
        ]),
        Statement::Switch {
            arms,
            default_body,
            default_span,
            ..
        } => {
            let mut children: Vec<Value> = arms
                .iter()
                .map(|arm| {
                    let kind = if arm.fallthrough {
                        "fallthrough"
                    } else {
                        "arm"
                    };
                    let body = arm
                        .body
                        .as_ref()
                        .map_or_else(|| Value::Array(vec![]), |b| serialise_script(b, li, source));
                    json!({
                        "label": format!("{kind}: {}", preview(&arm.pattern, 48)),
                        "range": range_dict(arm.pattern_span, li, source),
                        "body": body,
                    })
                })
                .collect();
            if let Some(body) = default_body {
                children.push(json!({
                    "label": "default",
                    "range": range_or_null(*default_span, li, source),
                    "body": serialise_script(body, li, source),
                }));
            }
            Some(children)
        }
        _ => None,
    }
}

/// Serialise the `ir` view. Mirrors `_serialise_ir`:
/// `{ topLevel: [...], procedures: { qname: { params, range, body } } }`.
#[must_use]
pub fn serialise_ir(module: &Module, li: &LineIndex, source: &str) -> Value {
    let mut procedures = Map::new();
    let mut qnames: Vec<&String> = module.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        let proc = &module.procedures[qname];
        procedures.insert(
            qname.clone(),
            json!({
                "params": proc.params,
                "range": range_dict(proc.span, li, source),
                "body": serialise_script(&proc.body, li, source),
            }),
        );
    }
    json!({
        "topLevel": serialise_script(&module.top_level, li, source),
        "procedures": Value::Object(procedures),
    })
}

/// Serialise a block terminator. Mirrors `_terminator_dict`.
fn terminator_dict(term: Option<&Terminator>, li: &LineIndex, source: &str) -> Value {
    match term {
        None => Value::Null,
        Some(Terminator::Goto { target, span }) => json!({
            "type": "goto",
            "target": target,
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span,
        }) => json!({
            "type": "branch",
            "condition": preview(&render_expr(condition), 80),
            "trueTarget": true_target,
            "falseTarget": false_target,
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Return { value, span, .. }) => json!({
            "type": "return",
            "value": value.as_ref().map(|v| preview(v, 60)),
            "range": range_or_null(*span, li, source),
        }),
    }
}

/// The successor block names of a terminator (terminator-only, no
/// exception edges) — mirrors Python's `_block_successors`.
fn block_successors(term: Option<&Terminator>) -> Vec<&str> {
    term.map(Terminator::successors).unwrap_or_default()
}

/// Serialise the routed control-flow edges of `func`. Mirrors
/// `_serialise_cfg_edges`; lanes come from the shared `cfg_layout`.
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

/// Serialise the pre-SSA CFG view. Mirrors `_serialise_cfg_pre_ssa`:
/// one entry per function, blocks in creation order, with routed edges.
#[must_use]
pub fn serialise_cfg_pre_ssa(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .map(|snap| {
            let cfg = &snap.unit.cfg;
            let order = ordered_block_names(cfg);
            let blocks: Vec<Value> = order
                .iter()
                .map(|bn| {
                    let block = &cfg.blocks[bn];
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
                        "isEntry": *bn == cfg.entry,
                        "statements": statements,
                        "terminator": terminator_dict(block.terminator.as_ref(), li, source),
                        "successors": block_successors(block.terminator.as_ref()),
                    })
                })
                .collect();
            json!({
                "name": snap.name,
                "entry": cfg.entry,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// Format a declared arity. Mirrors `_serialise_interproc`'s arity string:
/// `"{min}+"` when unlimited (`max == u32::MAX`), else `"{min}..{max}"`.
fn arity_str(arity: tcl_compiler::interprocedural::Arity) -> String {
    if arity.max == u32::MAX {
        format!("{}+", arity.min)
    } else {
        format!("{}..{}", arity.min, arity.max)
    }
}

/// Serialise the `renderedProperties` view: each SSA value's `may` / `must`
/// rendered-property flag names, per function. Mirrors
/// `_serialise_rendered_properties` — values with no flags are skipped, and
/// functions with no entries are omitted. `iter_names()` yields the set
/// named flags in declaration order (NONE excluded), matching Python's
/// enum-member filter.
#[must_use]
pub fn serialise_rendered_properties(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .filter_map(|snap| {
            let mut keys: Vec<&(String, u32)> = snap.unit.rendered_props.keys().collect();
            keys.sort();
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let rp = &snap.unit.rendered_props[*key];
                    let may: Vec<&str> = rp.may.iter_names().map(|(n, _)| n).collect();
                    let must: Vec<&str> = rp.must.iter_names().map(|(n, _)| n).collect();
                    if may.is_empty() && must.is_empty() {
                        return None;
                    }
                    Some(json!({ "variable": key.0, "version": key.1, "may": may, "must": must }))
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Renderer severity for a shimmer/thunking code. Mirrors
/// `annotations.shimmer_severity` (`_DANGER_SHIMMER_CODES = {"S102"}`).
fn shimmer_severity(code: &str) -> &'static str {
    if code == "S102" { "error" } else { "warning" }
}

/// Serialise the `shimmer` view: intrep-shimmer (S100/S101) and
/// loop-thunking (S102) warnings. Mirrors `_serialise_shimmer`, combining
/// both warning kinds into one list. The shimmer analysis matches Python,
/// so this view is strictly gated by the differential harness.
#[must_use]
pub fn serialise_shimmer(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let mut out: Vec<Value> = Vec::new();
    for w in find_shimmer_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code,
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(&w.code),
            "variable": w.variable,
            "fromType": type_name(w.from_type),
            "toType": type_name(w.to_type),
            "command": w.command,
            "inLoop": w.in_loop,
        }));
    }
    for w in find_thunking_warnings_for_cu(&result.unit) {
        out.push(json!({
            "code": w.code,
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(&w.code),
            "variable": w.variable,
            "typeA": type_name(w.type_a),
            "typeB": type_name(w.type_b),
        }));
    }
    Value::Array(out)
}

/// Build the optimised-source explorer result, or `None` when the
/// optimiser leaves the source unchanged. Shared by the `optimisedSource`,
/// `irOptimised`, and `cfgPreSsaOptimised` keys (the "double pipeline" —
/// re-run the whole pipeline on the rewritten source). Returns the result
/// plus the optimised source string (whose ranges the optimised views
/// index into).
fn optimised_result(result: &ExplorerResult) -> Option<(crate::ExplorerResult, String)> {
    let registry = registry_for_dialect(&result.dialect);
    let optimised = apply_optimisations(&result.source, &optimise(&result.source, registry));
    if optimised == result.source {
        return None;
    }
    let unit = crate::run_pipeline(&optimised, &result.dialect);
    Some((unit, optimised))
}

/// Serialise the `gvn` view: redundant-computation hints (GVN/CSE + PRE +
/// LICM). Mirrors `_serialise_gvn` — `{code, message, expression, range,
/// firstRange, severity: info}`. Composes the three ported `*_for_cu`
/// finders and de-duplicates on `(code, span, first_span)`. Optimiser-
/// derived → `_NO_PARITY_KEYS`, pinned by a Rust unit test.
#[must_use]
pub fn serialise_gvn(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let dialect = Some(result.dialect.as_str());
    let mut all = find_redundancies_for_cu(&result.unit, registry, dialect);
    all.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    all.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));

    let mut seen = std::collections::HashSet::new();
    let out: Vec<Value> = all
        .iter()
        .filter(|w| {
            seen.insert((
                w.code.clone(),
                w.span.start(),
                w.span.end(),
                w.first_span.start(),
                w.first_span.end(),
            ))
        })
        .map(|w| {
            json!({
                "code": w.code,
                "message": w.message,
                "expression": w.expression_text,
                "range": range_dict(w.span, li, source),
                "firstRange": range_dict(w.first_span, li, source),
                "severity": "info",
            })
        })
        .collect();
    Value::Array(out)
}

/// Renderer severity for a taint code. Mirrors `annotations.taint_severity`
/// (`T1*` prefix or `T3001`-`T3004` → error, else warning).
fn taint_severity(code: &str) -> &'static str {
    if code.starts_with("T1") || matches!(code, "T3001" | "T3002" | "T3003" | "T3004") {
        "error"
    } else {
        "warning"
    }
}

/// Serialise the `taint` view: information-flow sink warnings. Mirrors
/// `_serialise_taint`. Composes the taint passes via the ported
/// `find_taint_warnings_for_cu`.
#[must_use]
pub fn serialise_taint(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let dialect = Some(result.dialect.as_str());
    let out: Vec<Value> = find_taint_warnings_for_cu(&result.unit, registry, dialect)
        .iter()
        .map(|w| {
            json!({
                "code": w.code,
                "message": w.message,
                "range": range_dict(w.span, li, source),
                "severity": taint_severity(&w.code),
                "variable": w.variable,
                "sinkCommand": w.sink_command,
            })
        })
        .collect();
    Value::Array(out)
}

/// Serialise the `optimisations` view: the optimiser rewrites found for
/// the source. Mirrors `_serialise_optimisations` — `{code, message,
/// range, replacement}` per rewrite. Runs the ported `optimise` pass over
/// the cached per-dialect registry.
#[must_use]
pub fn serialise_optimisations(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let opts: Vec<Value> = optimise(source, registry)
        .iter()
        .map(|o| {
            json!({
                "code": o.code,
                "message": o.message,
                "range": range_dict(o.span, li, source),
                "replacement": o.replacement,
            })
        })
        .collect();
    Value::Array(opts)
}

/// Serialise the `intervals` view: the integer-interval domain per tracked
/// SSA value, per function. Mirrors `_serialise_intervals` — only bounded
/// (non-top) ranges are emitted; `lo`/`hi` are `null` for ±infinity.
#[must_use]
pub fn serialise_intervals(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .map(|snap| {
            let intervals =
                compute_intervals(&snap.unit.cfg, &snap.unit.ssa, &snap.unit.sccp.values);
            let mut keys: Vec<&(String, u32)> = intervals.keys().collect();
            keys.sort();
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let iv = intervals[*key];
                    if iv.is_top() || (iv.lo.is_none() && iv.hi.is_none()) {
                        return None;
                    }
                    Some(json!({ "variable": key.0, "version": key.1, "lo": iv.lo, "hi": iv.hi }))
                })
                .collect();
            json!({ "name": snap.name, "entries": entries })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `taintTracking` view: every tainted SSA value's taint
/// lattice per function. Mirrors `_serialise_taint_tracking` — only
/// tainted entries, functions with none omitted.
#[must_use]
pub fn serialise_taint_tracking(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .filter_map(|snap| {
            let mut keys: Vec<&(String, u32)> = snap.unit.taints.keys().collect();
            keys.sort();
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let tl = &snap.unit.taints[*key];
                    tl.colours
                        .contains(tcl_compiler::taint::TaintColour::TAINTED)
                        .then(|| json!({ "variable": key.0, "version": key.1, "taint": format_taint(tl) }))
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Per-SSA-value `{version, lattice?, type?}` detail used by the post-SSA
/// CFG view's `uses`/`defs` maps. Mirrors the inline dict in
/// `_serialise_cfg_post_ssa`.
fn ssa_value_detail(
    refs: &std::collections::HashMap<String, tcl_compiler::ssa::Version>,
    sccp: &tcl_compiler::sccp::SccpResult,
    types: &std::collections::HashMap<
        tcl_compiler::ssa::ValueKey,
        tcl_compiler::types::TypeLattice,
    >,
) -> Value {
    let mut keys: Vec<(&String, &u32)> = refs.iter().collect();
    keys.sort();
    let mut out = Map::new();
    for (name, &ver) in keys {
        let mut d = Map::new();
        d.insert("version".to_owned(), json!(ver));
        if let Some(lat) = sccp.values.get(&(name.clone(), ver)) {
            d.insert("lattice".to_owned(), json!(format_lattice(lat)));
        }
        if let Some(tl) = types.get(&(name.clone(), ver))
            && tl.kind != tcl_compiler::types::TypeKind::Unknown
        {
            d.insert("type".to_owned(), json!(format_type(tl)));
        }
        out.insert(name.clone(), Value::Object(d));
    }
    Value::Object(out)
}

/// Serialise the post-SSA CFG view (`cfgPostSsa`): per-block phi nodes and
/// per-statement SSA `uses`/`defs` with lattice + type detail, plus a
/// function-level `analysis` block. Mirrors `_serialise_cfg_post_ssa`.
///
/// `_NO_PARITY`: `analysis.deadStores` is always `[]` (the Rust liveness
/// pass is unported) and the lattice/type detail comes from the divergent
/// Rust analyses; `constantBranches`/`unreachableBlocks`/`inferredTypes`
/// come from SCCP + the type lattice.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn serialise_cfg_post_ssa(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .map(|snap| {
            let cfg = &snap.unit.cfg;
            let ssa = &snap.unit.ssa;
            let sccp = &snap.unit.sccp;
            let types = &snap.unit.types;
            let order = ordered_block_names(cfg);

            let blocks: Vec<Value> = order
                .iter()
                .map(|bn| {
                    let block = &cfg.blocks[bn];
                    let ssa_block = ssa.blocks.get(bn);

                    let phis: Vec<Value> = ssa_block
                        .map(|sb| {
                            sb.phis
                                .iter()
                                .map(|phi| {
                                    let mut inc: Vec<(&String, &u32)> =
                                        phi.incoming.iter().collect();
                                    inc.sort();
                                    let incoming: Map<String, Value> = inc
                                        .into_iter()
                                        .map(|(pred, &ver)| {
                                            (pred.clone(), json!(format!("{}#{ver}", phi.name)))
                                        })
                                        .collect();
                                    let phi_type = types
                                        .get(&(phi.name.clone(), phi.version))
                                        .map_or(Value::Null, |tl| json!(format_type(tl)));
                                    json!({
                                        "name": phi.name,
                                        "version": phi.version,
                                        "incoming": incoming,
                                        "type": phi_type,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

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
                                            ssa_value_detail(&ss.uses, sccp, types),
                                            ssa_value_detail(&ss.defs, sccp, types),
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
                        "isEntry": *bn == cfg.entry,
                        "isUnreachable": !sccp.executable_blocks.contains(bn),
                        "phis": phis,
                        "statements": statements,
                        "terminator": terminator_dict(block.terminator.as_ref(), li, source),
                        "successors": block_successors(block.terminator.as_ref()),
                    })
                })
                .collect();

            json!({
                "name": snap.name,
                "entry": cfg.entry,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
                "analysis": post_ssa_analysis(snap),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// The function-level `analysis` block of the post-SSA CFG view.
fn post_ssa_analysis(snap: &crate::FunctionSnapshot) -> Value {
    let sccp = &snap.unit.sccp;
    let constant_branches: Vec<Value> = sccp
        .constant_branches
        .iter()
        .map(|b| {
            json!({
                "block": b.block,
                "condition": preview(&b.condition, 60),
                "value": b.value,
                "takenTarget": b.taken_target,
                "notTakenTarget": b.not_taken_target,
            })
        })
        .collect();

    let mut unreachable: Vec<&String> = snap
        .unit
        .cfg
        .blocks
        .keys()
        .filter(|b| !sccp.executable_blocks.contains(*b))
        .collect();
    unreachable.sort();

    // `inferredTypes`: known/shimmered SSA-value types keyed by `name#ver`.
    let mut tkeys: Vec<&(String, u32)> = snap.unit.types.keys().collect();
    tkeys.sort();
    let mut inferred = Map::new();
    for key in tkeys {
        let tl = &snap.unit.types[key];
        if matches!(
            tl.kind,
            tcl_compiler::types::TypeKind::Known | tcl_compiler::types::TypeKind::Shimmered
        ) {
            inferred.insert(format!("{}#{}", key.0, key.1), json!(format_type(tl)));
        }
    }

    json!({
        "constantBranches": constant_branches,
        // The Rust liveness pass that fills dead stores is unported.
        "deadStores": Vec::<Value>::new(),
        "unreachableBlocks": unreachable,
        "inferredTypes": Value::Object(inferred),
    })
}

/// Serialise the `dataflow` view: the def-use data-flow graph. Mirrors
/// `dataflow_graph_to_dict` over `extract_dataflow_graph`.
///
/// `_NO_PARITY`: `extract_dataflow_graph` sorts functions whereas Python
/// emits top-level-first, and alias info comes from memory-SSA (not built
/// here, so `aliases` are limited); the node/edge detail also follows the
/// (divergent) Rust analyses.
#[must_use]
pub fn serialise_dataflow(result: &ExplorerResult) -> Value {
    let snaps = result.snapshots();
    let inputs: Vec<FunctionInputs<'_>> = snaps
        .iter()
        .map(|s| FunctionInputs {
            name: s.name,
            ssa: &s.unit.ssa,
            du: &s.unit.def_use,
            sccp: Some(&s.unit.sccp),
            mem: s.unit.memory_ssa.as_ref(),
        })
        .collect();
    let graph = extract_dataflow_graph(&inputs);

    let functions: Vec<Value> = graph
        .functions
        .iter()
        .map(|f| {
            let nodes: Vec<Value> = f
                .nodes
                .iter()
                .map(|n| {
                    json!({
                        "name": n.name,
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
                        "fromVersion": e.from_version,
                        "toBlock": e.to_block,
                        "toStatementIndex": e.to_statement_index,
                        "edgeKind": e.edge_kind.as_str(),
                        "toName": e.to_name,
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
                "nodes": nodes,
                "edges": edges,
                "aliases": aliases,
                "summary": {
                    "totalDefs": f.total_defs,
                    "totalUses": f.total_uses,
                    "deadDefs": f.dead_defs,
                    "aliasedVars": f.aliased_vars,
                },
            })
        })
        .collect();

    json!({
        "functions": functions,
        "summary": {
            "totalDefs": graph.total_defs(),
            "totalUses": graph.total_uses(),
            "totalAliases": graph.total_aliases(),
            "functionCount": graph.functions.len(),
        },
    })
}

/// Serialise the `irulesFlow` view: iRules flow / performance warnings.
/// Mirrors `_serialise_irules_flow` — `{code, message, range, severity}`,
/// composing the five `irules_checks` finders. Empty for non-iRules
/// dialects, so it strict-matches Python's empty list on the tcl corpus.
#[must_use]
pub fn serialise_irules_flow(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let dialect = Some(result.dialect.as_str());
    let cu = &result.unit;
    let mut warnings = find_unnormalised_getter_warnings(cu, registry, dialect);
    warnings.extend(find_unguarded_drop_warnings(cu, dialect));
    warnings.extend(find_collect_flow_warnings(cu, registry, dialect));
    warnings.extend(find_http_flow_warnings(cu, dialect));
    warnings.extend(find_hoistable_set_warnings(cu, dialect));

    let out: Vec<Value> = warnings
        .iter()
        .map(|w| {
            json!({
                "code": w.code,
                "message": w.message,
                "range": range_dict(w.span, li, source),
                "severity": "warning",
            })
        })
        .collect();
    Value::Array(out)
}

/// Serialise the `loops` view: the natural-loop forest per function, with
/// nesting depth. Mirrors `_serialise_loops` over the ported
/// `build_loop_forest`. Depth = 1 + the number of *other* loop headers
/// that dominate this loop's header.
#[must_use]
pub fn serialise_loops(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .map(|snap| {
            let executable = &snap.unit.sccp.executable_blocks;
            let forest = build_loop_forest(&snap.unit.cfg, &snap.unit.ssa, executable);
            let headers = forest.headers();
            let loops: Vec<Value> = forest
                .loops
                .iter()
                .map(|lp| {
                    let depth = 1 + headers
                        .iter()
                        .filter(|h| **h != lp.header && dominates(&snap.unit.ssa, h, &lp.header))
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
            json!({ "name": snap.name, "loops": loops })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `bounds` view: interval-driven out-of-range findings per
/// function. Mirrors `_serialise_bounds`.
///
/// `_NO_PARITY`: the Rust `find_interval_bounds` takes no `execution_intent`
/// and emits no divide-by-zero (`W233`) findings, so `divzero` is always
/// `[]` and the findings come from the (divergent) Rust interval analysis.
#[must_use]
pub fn serialise_bounds(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .map(|snap| {
            let findings: Vec<Value> = find_interval_bounds(
                &snap.unit.cfg,
                &snap.unit.ssa,
                &snap.unit.sccp.values,
                &snap.unit.sccp.executable_blocks,
            )
            .iter()
            .map(|f| {
                json!({
                    "code": f.code,
                    "command": f.command,
                    "indexVar": f.index_var,
                    "lo": f.index_interval.lo,
                    "hi": f.index_interval.hi,
                    "length": f.length,
                    "reason": f.reason,
                })
            })
            .collect();
            json!({ "name": snap.name, "findings": findings, "divzero": Vec::<Value>::new() })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `interprocedural` view: per-procedure summaries followed
/// by `TclOO` method summaries. Mirrors `_serialise_interproc`.
#[must_use]
pub fn serialise_interproc(interproc: &InterproceduralAnalysis) -> Value {
    let mut out: Vec<Value> = Vec::new();

    let mut qnames: Vec<&String> = interproc.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        let s = &interproc.procedures[qname];
        out.push(json!({
            "name": qname,
            "arity": arity_str(s.arity),
            "pure": s.pure,
            "foldable": s.can_fold_static_calls,
            "returnShape": format_return_shape(s),
            "calls": s.calls,
            "hasBarrier": s.has_barrier,
            "hasUnknownCalls": s.has_unknown_calls,
            "writesGlobal": s.writes_global,
        }));
    }

    let mut mqnames: Vec<&String> = interproc.methods.keys().collect();
    mqnames.sort();
    for mqname in mqnames {
        let m = &interproc.methods[mqname];
        let mut writes_instance: Vec<&String> = m.writes_instance_vars.iter().collect();
        writes_instance.sort();
        out.push(json!({
            "name": mqname,
            "kind": "method",
            "methodKind": m.method_kind,
            "className": m.class_name,
            "pure": m.base.pure,
            "calls": m.base.calls,
            "hasBarrier": m.base.has_barrier,
            "hasUnknownCalls": m.base.has_unknown_calls,
            "writesGlobal": m.base.writes_global,
            "writesInstanceVars": writes_instance,
        }));
    }

    Value::Array(out)
}

/// Serialise the `types` view: every tracked SSA value's lattice type per
/// function, including `?`/`*` entries. Mirrors `_serialise_types`.
#[must_use]
pub fn serialise_types(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .filter_map(|snap| {
            let mut entries: Vec<Value> = Vec::new();
            let rt = &snap.unit.return_type;
            entries.push(json!({
                "variable": "(return)",
                "version": 0,
                "type": format_type(rt),
                "kind": type_kind_name(rt.kind),
            }));
            let mut keys: Vec<&(String, u32)> = snap.unit.types.keys().collect();
            keys.sort();
            for key in keys {
                let tl = &snap.unit.types[key];
                entries.push(json!({
                    "variable": key.0,
                    "version": key.1,
                    "type": format_type(tl),
                    "kind": type_kind_name(tl.kind),
                }));
            }
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `segments` view: the `SegmentedCommand` list with each
/// command's closer-inclusive range, source slice, words (in `word_piece`
/// form with per-word shape flags), and forward-attached preceding comment.
/// Mirrors `_serialise_segments`.
///
/// `name` is the first word's text; `subcommand` is always `null` (the CST
/// segment path does not resolve subcommands). `braced` is whether the
/// word's first fragment is a braced `{…}` (`Str`) token; `quoted` whether
/// its raw begins with `"` — both derived from the representative token, as
/// the Python CST derives them from the first fragment.
#[must_use]
pub fn serialise_segments(source: &str) -> Value {
    let bytes = source.as_bytes();
    let segments: Vec<Value> = segment_commands(source)
        .iter()
        .map(|seg| {
            let words: Vec<Value> = seg
                .argv
                .iter()
                .enumerate()
                .map(|(i, tok)| {
                    let start = tok.span.start() as usize;
                    json!({
                        "text": seg.texts[i],
                        "startOffset": tok.span.start(),
                        "endOffset": tok.span.end(),
                        "single": seg.single_token_word[i],
                        "braced": tok.kind == TokenType::Str,
                        "quoted": bytes.get(start) == Some(&b'"'),
                        "expand": seg.expand_word.as_ref().is_some_and(|e| e[i]),
                    })
                })
                .collect();
            let (start, end) = (seg.span.start() as usize, seg.span.end() as usize);
            json!({
                "name": seg.texts.first().cloned().unwrap_or_default(),
                "startOffset": seg.span.start(),
                "endOffset": seg.span.end(),
                "slice": source.get(start..end).unwrap_or(""),
                "precedingComment": seg.preceding_comment,
                "subcommand": Value::Null,
                "words": words,
            })
        })
        .collect();
    Value::Array(segments)
}

/// Serialise the `eventOrder` view: iRules `when EVENT [priority N] { body }`
/// handlers in canonical firing order. Faithful port of `extract_event_order`
/// (`compiler/irules_flow.py`) composed with `_serialise_event_order`.
///
/// Reuses the segmenter to find `when` commands (the body must be a braced
/// block) and the reuse-positive `EventRegistry::{order_events,
/// event_multiplicity}` for the canonical ordering / multiplicity class. Empty
/// for ordinary Tcl (no `when` blocks), so it is strictly gateable against the
/// Tcl corpus; the `when`-handler behaviour is pinned by a Rust unit test.
#[must_use]
pub fn serialise_event_order(source: &str, line_index: &LineIndex) -> Value {
    use std::collections::HashMap;

    use tcl_registry::events::EventRegistry;

    /// One discovered `when` handler.
    struct Handler {
        priority: i64,
        /// File-order index among matched handlers (tie-breaker).
        idx: usize,
        /// The `EVENT` token's span (for the navigable range).
        span: Span,
    }

    let mut per_event: HashMap<String, Vec<Handler>> = HashMap::new();
    let mut matched = 0usize;
    for seg in segment_commands(source) {
        if seg.texts.first().map(String::as_str) != Some("when") || seg.argv.len() < 3 {
            continue;
        }
        // The body (last word) must be a braced block (`Str`), matching the
        // Python `body_tok.type is TokenType.STR` guard.
        let Some(body) = seg.argv.last() else {
            continue;
        };
        if body.kind != TokenType::Str {
            continue;
        }
        let event = seg.texts[1].clone();
        let span = seg.argv[1].span;
        // `when EVENT priority N { body }` — base priority defaults to 500.
        // `when EVENT priority N { body }`; a missing/non-integer priority
        // keeps the 500 default (matching Python).
        let mut priority = 500;
        if seg.texts.len() >= 5
            && seg.texts[2] == "priority"
            && let Ok(parsed) = seg.texts[3].parse::<i64>()
        {
            priority = parsed;
        }
        per_event.entry(event).or_default().push(Handler {
            priority,
            idx: matched,
            span,
        });
        matched += 1;
    }

    // Each event's handlers fire lowest-priority first, file order breaking ties.
    for handlers in per_event.values_mut() {
        handlers.sort_by_key(|h| (h.priority, h.idx));
    }

    let registry = EventRegistry::build();
    let keys: Vec<String> = per_event.keys().cloned().collect();
    let mut entries = Vec::new();
    for event in registry.order_events(&keys) {
        let multiplicity = registry.event_multiplicity(&event);
        let handlers = &per_event[&event];
        let mut prev_priority: Option<i64> = None;
        let mut offset = 0;
        for handler in handlers {
            if prev_priority == Some(handler.priority) {
                offset += 1;
            } else {
                offset = 0;
                prev_priority = Some(handler.priority);
            }
            entries.push(json!({
                "event": event,
                "base_priority": handler.priority,
                "priority_offset": offset,
                "multiplicity": multiplicity,
                "range": range_dict(handler.span, line_index, source),
            }));
        }
    }
    Value::Array(entries)
}

/// Serialise the `stats` summary. Mirrors `compute_stats`.
///
/// `_NO_PARITY`: `deadStores` is `0` (the Rust liveness pass is unported)
/// and the warning counts come from the Rust analyses (which diverge from
/// Python). `dataflow*` counts are omitted — `dataflow` is unported, and
/// Python only adds them when a dataflow graph is present.
fn serialise_stats(result: &ExplorerResult) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let dialect = Some(result.dialect.as_str());

    let unreachable: usize = result
        .snapshots()
        .iter()
        .map(|s| {
            s.unit
                .cfg
                .blocks
                .keys()
                .filter(|b| !s.unit.sccp.executable_blocks.contains(*b))
                .count()
        })
        .sum();

    let shimmer = find_shimmer_warnings_for_cu(&result.unit, registry).len()
        + find_thunking_warnings_for_cu(&result.unit).len();
    let mut gvn = find_redundancies_for_cu(&result.unit, registry, dialect);
    gvn.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    gvn.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));
    let taint = find_taint_warnings_for_cu(&result.unit, registry, dialect).len();
    let rewrites = optimise(&result.source, registry).len();

    json!({
        "procedures": result.unit.ir_module.procedures.len(),
        "functions": result.snapshots().len(),
        "blocks": result.total_blocks(),
        "deadStores": 0,
        "unreachableBlocks": unreachable,
        "rewrites": rewrites,
        "shimmerWarnings": shimmer,
        "gvnWarnings": gvn.len(),
        "taintWarnings": taint,
        "irulesFlowWarnings": 0,
    })
}

/// One source-callout annotation, before serialisation.
struct Ann {
    span: Span,
    label: String,
    kind: &'static str,
    severity: &'static str,
    priority: i32,
}

/// Collect compiler-barrier annotations from a script, recursing into
/// If/For/Switch bodies only — mirroring Python's `_walk_barriers`.
fn walk_barriers(script: &Script, scope: &str, out: &mut Vec<Ann>) {
    for stmt in &script.statements {
        match stmt {
            Statement::Barrier { span, reason, .. } => out.push(Ann {
                span: *span,
                label: format!("{scope}: compiler barrier ({reason})"),
                kind: "barrier",
                severity: "warning",
                priority: 2,
            }),
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_barriers(&c.body, scope, out);
                }
                if let Some(e) = else_body {
                    walk_barriers(e, scope, out);
                }
            }
            Statement::For {
                init, body, next, ..
            } => {
                walk_barriers(init, scope, out);
                walk_barriers(body, scope, out);
                walk_barriers(next, scope, out);
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_barriers(b, scope, out);
                    }
                }
                if let Some(d) = default_body {
                    walk_barriers(d, scope, out);
                }
            }
            _ => {}
        }
    }
}

/// Serialise the `annotations` + `annotationsByLine` source-callout views.
/// Mirrors `collect_annotations` (all sources, GUI default), then
/// `_serialise_annotation` + `_group_annotations_by_line`.
///
/// `_NO_PARITY`: aggregates the optimiser / shimmer / gvn / taint sources
/// (which diverge from Python) and **omits dead-store callouts** — the Rust
/// liveness pass that fills `FunctionAnalysis.dead_stores` is unported.
/// constant-branch + unreachable-block callouts come from `sccp`.
#[allow(clippy::too_many_lines)]
fn serialise_annotations(result: &ExplorerResult, li: &LineIndex, source: &str) -> (Value, Value) {
    let registry = registry_for_dialect(&result.dialect);
    let dialect = Some(result.dialect.as_str());
    let mut anns: Vec<Ann> = Vec::new();

    // Barriers (IR walk).
    walk_barriers(&result.unit.ir_module.top_level, "::top", &mut anns);
    let mut qnames: Vec<&String> = result.unit.ir_module.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        walk_barriers(
            &result.unit.ir_module.procedures[qname].body,
            qname,
            &mut anns,
        );
    }

    // Constant branches + unreachable blocks (from SCCP), per function.
    for snap in result.snapshots() {
        for branch in &snap.unit.sccp.constant_branches {
            if let Some(span) = branch.span {
                let dir = if branch.value { "true" } else { "false" };
                anns.push(Ann {
                    span,
                    label: format!(
                        "{}: branch is always {dir}; takes {}",
                        snap.name, branch.taken_target
                    ),
                    kind: "constantBranch",
                    severity: "info",
                    priority: 0,
                });
            }
        }
        let mut unreachable: Vec<&String> = snap
            .unit
            .cfg
            .blocks
            .keys()
            .filter(|b| !snap.unit.sccp.executable_blocks.contains(*b))
            .collect();
        unreachable.sort();
        for bn in unreachable {
            let block = &snap.unit.cfg.blocks[bn];
            let span = block
                .statements
                .first()
                .map(Statement::span)
                .or_else(|| block.terminator.as_ref().and_then(Terminator::span));
            if let Some(span) = span {
                anns.push(Ann {
                    span,
                    label: format!("{}: unreachable block {bn}", snap.name),
                    kind: "unreachable",
                    severity: "warning",
                    priority: 3,
                });
            }
        }
    }

    // Optimiser rewrites.
    for o in optimise(source, registry) {
        anns.push(Ann {
            span: o.span,
            label: format!(
                "{}: {} -> {}",
                o.code,
                o.message,
                preview(&o.replacement, 40)
            ),
            kind: "optimisation",
            severity: "info",
            priority: -1,
        });
    }
    // Shimmer + thunking.
    for w in find_shimmer_warnings_for_cu(&result.unit, registry) {
        let sev = shimmer_severity(&w.code);
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "shimmer",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    for w in find_thunking_warnings_for_cu(&result.unit) {
        let sev = shimmer_severity(&w.code);
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "thunking",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    // GVN.
    let mut gvn = find_redundancies_for_cu(&result.unit, registry, dialect);
    gvn.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    gvn.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));
    for w in gvn {
        let label = if w.message.is_empty() {
            format!("{}: {}", w.code, w.expression_text)
        } else {
            format!("{}: {}", w.code, w.message)
        };
        anns.push(Ann {
            span: w.span,
            label,
            kind: "gvn",
            severity: "info",
            priority: 1,
        });
    }
    // Taint.
    for w in find_taint_warnings_for_cu(&result.unit, registry, dialect) {
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "taint",
            severity: taint_severity(&w.code),
            priority: 0,
        });
    }

    // Sort: (start_offset, priority, span_width, label).
    anns.sort_by(|a, b| {
        a.span
            .start()
            .cmp(&b.span.start())
            .then(a.priority.cmp(&b.priority))
            .then((a.span.end() - a.span.start()).cmp(&(b.span.end() - b.span.start())))
            .then(a.label.cmp(&b.label))
    });

    let dicts: Vec<Value> = anns
        .iter()
        .map(|a| {
            json!({
                "range": range_dict(a.span, li, source),
                "label": a.label,
                "kind": a.kind,
                "severity": a.severity,
                "priority": a.priority,
            })
        })
        .collect();

    // Group annotation indices by 0-based source line.
    let mut by_line: Map<String, Value> = Map::new();
    for (idx, a) in anns.iter().enumerate() {
        let line = li.position_at(a.span.start()).line.to_string();
        by_line
            .entry(line)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .unwrap()
            .push(json!(idx));
    }

    (Value::Array(dicts), Value::Object(by_line))
}

/// Serialise a full pipeline result to the explorer contract JSON.
///
/// Currently emits the ported view families; subsequent EXP-* increments
/// add one family per step. The argument is accepted now so the signature
/// is stable as views that read `result` land.
#[must_use]
pub fn serialise_result(result: &ExplorerResult) -> Value {
    let li = LineIndex::new(&result.source);
    let mut out = Map::new();
    out.insert("meta".to_owned(), serialise_meta());
    out.insert(
        "ir".to_owned(),
        serialise_ir(&result.unit.ir_module, &li, &result.source),
    );
    out.insert(
        "cfgPreSsa".to_owned(),
        serialise_cfg_pre_ssa(result, &li, &result.source),
    );
    out.insert(
        "cfgPostSsa".to_owned(),
        serialise_cfg_post_ssa(result, &li, &result.source),
    );
    if let Some(interproc) = &result.unit.interproc {
        out.insert("interprocedural".to_owned(), serialise_interproc(interproc));
    }
    out.insert("types".to_owned(), serialise_types(result));
    out.insert("cst".to_owned(), crate::cst::serialise_cst(&result.source));
    out.insert("segments".to_owned(), serialise_segments(&result.source));
    out.insert("loops".to_owned(), serialise_loops(result));
    out.insert("intervals".to_owned(), serialise_intervals(result));
    out.insert("bounds".to_owned(), serialise_bounds(result));
    out.insert(
        "renderedProperties".to_owned(),
        serialise_rendered_properties(result),
    );
    out.insert(
        "optimisations".to_owned(),
        serialise_optimisations(result, &li, &result.source),
    );
    out.insert(
        "shimmer".to_owned(),
        serialise_shimmer(result, &li, &result.source),
    );
    out.insert(
        "taintWarnings".to_owned(),
        serialise_taint(result, &li, &result.source),
    );
    out.insert("gvn".to_owned(), serialise_gvn(result, &li, &result.source));
    out.insert("taintTracking".to_owned(), serialise_taint_tracking(result));
    out.insert("dataflow".to_owned(), serialise_dataflow(result));
    out.insert(
        "irulesFlow".to_owned(),
        serialise_irules_flow(result, &li, &result.source),
    );
    out.insert(
        "eventOrder".to_owned(),
        serialise_event_order(&result.source, &li),
    );
    out.insert(
        "asm".to_owned(),
        crate::asm::serialise_asm(result, &li, &result.source),
    );

    // Double-pipeline keys: re-run on the optimiser's rewritten source.
    let opt = optimised_result(result);
    out.insert(
        "optimisedSource".to_owned(),
        opt.as_ref()
            .map_or(Value::Null, |(_, s)| Value::String(s.clone())),
    );
    out.insert(
        "irOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            serialise_ir(&r.unit.ir_module, &LineIndex::new(s), s)
        }),
    );
    out.insert(
        "cfgPreSsaOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            serialise_cfg_pre_ssa(r, &LineIndex::new(s), s)
        }),
    );
    out.insert(
        "cfgPostSsaOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            serialise_cfg_post_ssa(r, &LineIndex::new(s), s)
        }),
    );
    out.insert(
        "asmOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            crate::asm::serialise_asm(r, &LineIndex::new(s), s)
        }),
    );

    // WASM views: the WASM emitter is unported (zero Rust files), so these
    // degrade to `null` — `explorer-core.js` renders the WASM tab's empty
    // state. Python emits real disassembly here, so they are `_NO_PARITY`
    // until the emitter chunk lands.
    out.insert("wasm".to_owned(), Value::Null);
    out.insert("wasmOptimised".to_owned(), Value::Null);

    let (annotations, by_line) = serialise_annotations(result, &li, &result.source);
    out.insert("annotations".to_owned(), annotations);
    out.insert("annotationsByLine".to_owned(), by_line);
    out.insert("stats".to_owned(), serialise_stats(result));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_pipeline;

    #[test]
    fn meta_lists_all_dialects_views_and_severities() {
        let meta = serialise_meta();
        assert_eq!(meta["dialects"].as_array().unwrap().len(), 14);
        assert_eq!(meta["views"].as_array().unwrap().len(), 24);
        assert_eq!(meta["severities"], json!(["error", "warning", "info"]));
        // First view tab matches the Python table head.
        assert_eq!(
            meta["views"][0],
            json!({ "id": "greentree", "label": "Green Tree", "group": "compiler" })
        );
    }

    #[test]
    fn serialise_result_includes_meta_and_ir() {
        let result = run_pipeline("set x 1", "tcl8.6");
        let value = serialise_result(&result);
        assert!(value.get("meta").is_some());
        let ir = &value["ir"];
        assert!(ir["topLevel"].is_array());
        assert!(ir["procedures"].is_object());
    }

    #[test]
    fn ir_expands_if_clauses_and_collects_procs() {
        let result = run_pipeline(
            "proc f {x} { if {$x > 0} { puts hi } else { puts lo } }",
            "tcl8.6",
        );
        let ir = serialise_result(&result)["ir"].clone();
        let body = &ir["procedures"]["::f"]["body"];
        let if_node = &body[0];
        assert_eq!(if_node["kind"], "IRIf");
        assert_eq!(if_node["colorClass"], "ir-control");
        // Two children: clause 1 and the else arm.
        let children = if_node["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0]["label"], "clause 1: $x > 0");
        assert_eq!(children[1]["label"], "else");
    }

    #[test]
    fn optimised_source_reflects_constant_fold() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let opt = serialise_result(&result)["optimisedSource"].clone();
        // The constant fold changes the source, so it is a non-null string.
        let s = opt.as_str().expect("optimised source string");
        assert!(s.contains("set x 3"));
    }

    #[test]
    fn optimised_source_null_when_unchanged() {
        let result = run_pipeline("puts hello", "tcl8.6");
        let v = serialise_result(&result);
        assert_eq!(v["optimisedSource"], Value::Null);
        // The double-pipeline keys are also null when nothing changed.
        assert_eq!(v["irOptimised"], Value::Null);
        assert_eq!(v["cfgPreSsaOptimised"], Value::Null);
    }

    #[test]
    fn optimised_ir_and_cfg_present_when_source_changes() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let v = serialise_result(&result);
        // The fold rewrites the source, so the optimised views populate and
        // their IR reflects the folded `set x 3`.
        assert!(v["irOptimised"]["topLevel"].is_array());
        assert!(v["cfgPreSsaOptimised"].is_array());
        assert!(!v["cfgPreSsaOptimised"].as_array().unwrap().is_empty());
    }

    #[test]
    fn gvn_reports_redundant_computation() {
        let result = run_pipeline(
            "set x [list 1 2 3]\nset a [llength $x]\nset b [llength $x]\nputs $a$b",
            "tcl8.6",
        );
        let gvn = serialise_result(&result)["gvn"].clone();
        let arr = gvn.as_array().unwrap();
        let o105 = arr
            .iter()
            .find(|w| w["code"] == "O105")
            .expect("O105 finding");
        assert_eq!(o105["expression"], "llength $x");
        assert_eq!(o105["severity"], "info");
        assert!(o105["firstRange"]["startOffset"].is_number());
    }

    #[test]
    fn irules_flow_empty_for_plain_tcl() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        assert_eq!(serialise_result(&result)["irulesFlow"], json!([]));
    }

    #[test]
    fn event_order_empty_for_plain_tcl() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        assert_eq!(serialise_result(&result)["eventOrder"], json!([]));
    }

    #[test]
    fn event_order_orders_when_handlers_by_firing_order() {
        // Two HTTP_REQUEST handlers (one with explicit priority) plus
        // RULE_INIT and HTTP_RESPONSE. Verified byte-for-byte against Python
        // `_serialise_event_order` for this snippet.
        let src = "when RULE_INIT { set ::x 1 }\n\
                   when HTTP_REQUEST priority 100 { log local0. a }\n\
                   when HTTP_REQUEST { log local0. b }\n\
                   when HTTP_RESPONSE { log local0. c }";
        let result = run_pipeline(src, "f5-irules");
        let order = serialise_result(&result)["eventOrder"].clone();
        let rows = order.as_array().expect("eventOrder is an array");
        assert_eq!(rows.len(), 4);

        // RULE_INIT fires first (init), then HTTP_REQUEST (priority 100 before
        // the default 500), then the default HTTP_REQUEST, then HTTP_RESPONSE.
        assert_eq!(rows[0]["event"], "RULE_INIT");
        assert_eq!(rows[0]["multiplicity"], "init");
        assert_eq!(rows[1]["event"], "HTTP_REQUEST");
        assert_eq!(rows[1]["base_priority"], 100);
        assert_eq!(rows[1]["multiplicity"], "per_request");
        assert_eq!(rows[2]["event"], "HTTP_REQUEST");
        assert_eq!(rows[2]["base_priority"], 500);
        assert_eq!(rows[3]["event"], "HTTP_RESPONSE");
        // The range points at the `EVENT` token, not `when`.
        assert_eq!(rows[0]["range"]["startOffset"], 5);
    }

    #[test]
    fn event_order_tie_break_increments_offset() {
        // Two handlers for the same event at the same (default) priority: the
        // second gets priority_offset 1.
        let src = "when HTTP_REQUEST { log local0. a }\n\
                   when HTTP_REQUEST { log local0. b }";
        let result = run_pipeline(src, "f5-irules");
        let order = serialise_result(&result)["eventOrder"].clone();
        let rows = order.as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["priority_offset"], 0);
        assert_eq!(rows[1]["priority_offset"], 1);
    }

    #[test]
    fn dataflow_emits_nodes_and_summary() {
        let result = run_pipeline("set x 1\nset y [expr {$x + 1}]\nputs $y", "tcl8.6");
        let df = serialise_result(&result)["dataflow"].clone();
        assert!(df["functions"].is_array());
        let summary = &df["summary"];
        assert!(summary["totalDefs"].as_u64().unwrap() >= 2);
        assert_eq!(
            summary["functionCount"],
            df["functions"].as_array().unwrap().len()
        );
        let f0 = &df["functions"].as_array().unwrap()[0];
        assert!(f0["nodes"].is_array());
        assert!(f0["edges"].is_array());
        assert!(f0["aliases"].is_array());
    }

    #[test]
    fn loops_detects_for_loop_with_depth() {
        let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
        let loops = serialise_result(&result)["loops"].clone();
        let top = &loops.as_array().unwrap()[0];
        let lps = top["loops"].as_array().unwrap();
        assert_eq!(lps.len(), 1, "one natural loop for a single for-loop");
        let lp = &lps[0];
        assert_eq!(lp["depth"], 1);
        assert!(lp["header"].as_str().unwrap().contains("header"));
        assert!(!lp["latches"].as_array().unwrap().is_empty());
        assert_eq!(
            lp["blockCount"].as_u64().unwrap(),
            lp["blocks"].as_array().unwrap().len() as u64
        );
    }

    #[test]
    fn loops_empty_for_straightline_code() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        let loops = serialise_result(&result)["loops"].clone();
        assert!(
            loops.as_array().unwrap()[0]["loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bounds_emits_per_function_shape() {
        let result = run_pipeline("set l [list a b c]\nlindex $l 0", "tcl8.6");
        let bounds = serialise_result(&result)["bounds"].clone();
        let top = &bounds.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        assert!(top["findings"].is_array());
        // The Rust interval analysis emits no divide-by-zero findings.
        assert_eq!(top["divzero"], json!([]));
    }

    #[test]
    fn cfg_post_ssa_has_phis_uses_defs_and_analysis() {
        let result = run_pipeline("set x 1\nset y [expr {$x + 1}]\nputs $y", "tcl8.6");
        let post = serialise_result(&result)["cfgPostSsa"].clone();
        let top = &post.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        assert!(top["analysis"]["constantBranches"].is_array());
        assert!(top["analysis"]["deadStores"].is_array());
        assert!(top["analysis"]["inferredTypes"].is_object());
        let block = &top["blocks"].as_array().unwrap()[0];
        assert!(block["phis"].is_array());
        assert!(block["isEntry"].as_bool().unwrap());
        // A statement carries SSA uses/defs maps.
        let stmt = &block["statements"].as_array().unwrap()[0];
        assert!(stmt["uses"].is_object());
        assert!(stmt["defs"].is_object());
    }

    #[test]
    fn cfg_post_ssa_optimised_present_when_source_changes() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let v = serialise_result(&result);
        assert!(v["cfgPostSsaOptimised"].is_array());
    }

    #[test]
    fn stats_reports_function_and_block_counts() {
        let result = run_pipeline("proc f {} { return 1 }\nf", "tcl8.6");
        let stats = serialise_result(&result)["stats"].clone();
        assert_eq!(stats["procedures"], 1);
        // top-level + ::f.
        assert_eq!(stats["functions"], 2);
        assert!(stats["blocks"].as_u64().unwrap() >= 1);
        assert!(stats["rewrites"].is_number());
        assert!(stats["shimmerWarnings"].is_number());
    }

    #[test]
    fn annotations_collect_callouts_and_group_by_line() {
        let result = run_pipeline(
            "set a 5\nset b [expr {$a + 1}]\nset c [expr {$a + 1}]\nputs $b$c",
            "tcl8.6",
        );
        let v = serialise_result(&result);
        let anns = v["annotations"].as_array().unwrap();
        assert!(!anns.is_empty(), "expected source callouts");
        for a in anns {
            assert!(a["range"]["startOffset"].is_number());
            assert!(a["label"].is_string());
            assert!(a["kind"].is_string());
        }
        // byLine groups annotation indices under string line keys.
        let by_line = v["annotationsByLine"].as_object().unwrap();
        let total: usize = by_line.values().map(|v| v.as_array().unwrap().len()).sum();
        assert_eq!(total, anns.len());
    }

    #[test]
    fn asm_emits_structured_instructions_for_top_level() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        let asm = serialise_result(&result)["asm"].clone();
        let top = &asm.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        assert_eq!(top["kind"], "top");
        assert!(top["text"].as_str().unwrap().contains("ByteCode"));
        let ops: Vec<&str> = top["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "instr")
            .map(|r| r["op"].as_str().unwrap())
            .collect();
        // The store and the invoke are present; each instr has a fullText.
        assert!(ops.contains(&"storeStk"));
        assert!(ops.contains(&"invokeStk1"));
        assert!(ops.contains(&"done"));
        let first_instr = top["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "instr")
            .unwrap();
        assert!(
            first_instr["fullText"]
                .as_str()
                .unwrap()
                .starts_with("push1")
        );
    }

    #[test]
    fn asm_resolves_jump_targets() {
        // A conditional produces a jump whose target resolves to a row idx.
        let result = run_pipeline("if {$x} { puts hi }", "tcl8.6");
        let asm = serialise_result(&result)["asm"].clone();
        let top = &asm.as_array().unwrap()[0];
        let has_jump = top["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["jumpTarget"].is_object());
        assert!(has_jump, "expected at least one resolved jump target");
    }

    #[test]
    fn taint_tracking_marks_exec_tainted_vars() {
        let result = run_pipeline("set x [exec ls]\nset y $x", "tcl8.6");
        let tracking = serialise_result(&result)["taintTracking"].clone();
        let top = &tracking.as_array().unwrap()[0];
        let vars: Vec<&str> = top["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["variable"].as_str().unwrap())
            .collect();
        assert!(vars.contains(&"x"));
        assert!(vars.contains(&"y"));
        assert_eq!(top["entries"][0]["taint"], "tainted");
    }

    #[test]
    fn taint_reports_eval_sink() {
        let result = run_pipeline("set x [gets stdin]\neval $x", "tcl8.6");
        let taint = serialise_result(&result)["taintWarnings"].clone();
        let arr = taint.as_array().unwrap();
        let t = arr
            .iter()
            .find(|w| w["code"] == "T100")
            .expect("T100 warning");
        assert_eq!(t["variable"], "x");
        assert_eq!(t["sinkCommand"], "eval");
        assert_eq!(t["severity"], "error");
    }

    #[test]
    fn shimmer_reports_intrep_conversion() {
        let result = run_pipeline("set x [list 1 2 3]\nincr x", "tcl8.6");
        let shimmer = serialise_result(&result)["shimmer"].clone();
        let arr = shimmer.as_array().unwrap();
        let s100 = arr
            .iter()
            .find(|w| w["code"] == "S100")
            .expect("S100 warning");
        assert_eq!(s100["variable"], "x");
        assert_eq!(s100["fromType"], "list");
        assert_eq!(s100["toType"], "int");
        assert_eq!(s100["command"], "incr");
        assert_eq!(s100["severity"], "warning");
    }

    #[test]
    fn optimisations_reports_constant_fold_with_range() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let opts = serialise_result(&result)["optimisations"].clone();
        let arr = opts.as_array().unwrap();
        assert!(!arr.is_empty(), "expected at least one optimisation");
        // Every entry carries the contract fields.
        for o in arr {
            assert!(o["code"].is_string());
            assert!(o["message"].is_string());
            assert!(o["replacement"].is_string());
            assert!(o["range"]["startOffset"].is_number());
        }
        // The constant-fold rewrite is present.
        assert!(arr.iter().any(|o| o["code"] == "O101"));
    }

    #[test]
    fn rendered_properties_flags_slash_and_dash() {
        let result = run_pipeline("set p /var/log\nset u -flag", "tcl8.6");
        let rp = serialise_result(&result)["renderedProperties"].clone();
        let entries = rp.as_array().unwrap()[0]["entries"].as_array().unwrap();
        let p = entries.iter().find(|e| e["variable"] == "p").unwrap();
        assert!(
            p["may"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "HAS_FORWARD_SLASH")
        );
        let u = entries.iter().find(|e| e["variable"] == "u").unwrap();
        assert!(
            u["must"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "STARTS_WITH_DASH")
        );
    }

    #[test]
    fn intervals_reports_bounded_loop_counter() {
        let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
        let intervals = serialise_result(&result)["intervals"].clone();
        let top = &intervals.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        let entries = top["entries"].as_array().unwrap();
        // At least one bounded entry for `i` with a concrete lower bound.
        assert!(
            entries
                .iter()
                .any(|e| e["variable"] == "i" && e["lo"].is_number())
        );
    }

    #[test]
    fn segments_reports_words_with_shape_flags() {
        let segs = serialise_segments("string length \"hi\"");
        let arr = segs.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let cmd = &arr[0];
        assert_eq!(cmd["name"], "string");
        assert_eq!(cmd["subcommand"], Value::Null);
        let words = cmd["words"].as_array().unwrap();
        assert_eq!(words.len(), 3);
        assert_eq!(words[0]["text"], "string");
        assert_eq!(words[2]["quoted"], true);
        assert_eq!(words[2]["braced"], false);
    }

    #[test]
    fn segments_marks_braced_words() {
        let segs = serialise_segments("if {$x} { puts hi }");
        let words = segs[0]["words"].as_array().unwrap();
        // `{$x}` is a braced word.
        assert_eq!(words[1]["braced"], true);
        assert_eq!(words[1]["quoted"], false);
    }

    #[test]
    fn types_reports_known_int_for_constant_assignments() {
        let result = run_pipeline("set x 1\nset y 2", "tcl8.6");
        let types = serialise_result(&result)["types"].clone();
        let top = types
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "::top")
            .unwrap();
        let x = top["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["variable"] == "x")
            .unwrap();
        assert_eq!(x["type"], "int");
        assert_eq!(x["kind"], "known");
    }

    #[test]
    fn interproc_summarises_procs_with_arity_and_return_shape() {
        let result = run_pipeline("proc id {x} { return $x }\nid 5", "tcl8.6");
        let interproc = serialise_result(&result)["interprocedural"].clone();
        let add = interproc
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "::id")
            .expect("::id summary present");
        assert_eq!(add["arity"], "1..1");
        assert_eq!(add["returnShape"], "passthrough(x)");
        assert!(add["pure"].is_boolean());
        assert!(add["calls"].is_array());
    }

    #[test]
    fn cfg_pre_ssa_has_entry_block_and_branch_terminator() {
        let result = run_pipeline("if {$x} { puts a } else { puts b }", "tcl8.6");
        let cfg = serialise_result(&result)["cfgPreSsa"].clone();
        let top = &cfg[0];
        assert_eq!(top["name"], "::top");
        assert_eq!(top["entry"], "entry_1");
        let entry = top["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "entry_1")
            .unwrap();
        assert_eq!(entry["isEntry"], true);
        // The entry block branches on the `if` condition.
        assert_eq!(entry["terminator"]["type"], "branch");
        // Edges carry routing lanes from cfg_layout.
        assert!(
            top["edges"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["lane"].is_u64())
        );
    }

    #[test]
    fn ir_widens_braced_condition_range_to_closer() {
        // `{$x}` opens at the `{`; the range must cover the closing `}`.
        let src = "if {$x} { puts 1 }";
        let result = run_pipeline(src, "tcl8.6");
        let ir = serialise_result(&result)["ir"].clone();
        let cond_range = &ir["topLevel"][0]["children"][0]["range"];
        let start = usize::try_from(cond_range["startOffset"].as_u64().unwrap()).unwrap();
        let end = usize::try_from(cond_range["endOffset"].as_u64().unwrap()).unwrap();
        assert_eq!(&src[start..end], "{$x}");
    }
}
