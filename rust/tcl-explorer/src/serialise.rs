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

//! JSON serialisation of an [`ExplorerResult`] into the explorer contract
//! shape (`docs/design/contracts/wasm-explorer-view.md` for the `wasm`
//! slice; the rest is the de-facto contract `explorer-core.js` reads).
//!
//! `serialise_result` assembles the top-level object from the per-view
//! `serialise_*` helpers.

use serde_json::{Map, Value, json};

use tcl_compiler::cfg::{Function, Terminator};
use tcl_compiler::cfg_layout::{build_cfg_edges, ordered_block_names};
use tcl_compiler::dataflow_graph::{FunctionInputs, extract_dataflow_graph};
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::interprocedural::InterproceduralAnalysis;
use tcl_compiler::interval_bounds::{find_divide_by_zero, find_interval_bounds};
use tcl_compiler::intervals::compute_intervals;
use tcl_compiler::ir::{Module, Script, Statement};
use tcl_compiler::irules_checks::{
    find_collect_flow_warnings, find_hoistable_set_warnings, find_http_flow_warnings,
    find_unguarded_drop_warnings, find_unnormalised_getter_warnings,
};
use tcl_compiler::loops::{build_loop_forest, dominates};
use tcl_compiler::optimiser::{apply_optimisations, find_dead_stores, optimise, optimise_by_pass};
use tcl_compiler::segmenter::{segment_commands, segment_commands_with_offset_and_config};
use tcl_compiler::shimmer::{
    find_byte_array_warnings_for_cu, find_sharing_warnings_for_cu, find_shimmer_warnings_for_cu,
    find_thunking_warnings_for_cu, type_name,
};
use tcl_compiler::taint::find_taint_warnings_for_cu;
use tcl_lexer::{LexerConfig, LineIndex, Span, TokenType};
use tcl_registry::{available_dialects, registry_for_dialect};
use tcl_syntax::expr::ast::render_expr;

use crate::ExplorerResult;
use crate::formatters::{
    format_lattice, format_return_shape, format_taint, format_type, preview, range_dict,
    stmt_color_class, stmt_kind, stmt_summary, type_kind_name,
};
use crate::views::{Severity, VIEW_META};

/// Serialise the `meta` view: dialect list, view-tab table, and the
/// severity vocabulary.
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
fn range_or_null(span: Option<Span>, li: &LineIndex, source: &str) -> Value {
    span.map_or(Value::Null, |s| range_dict(s, li, source))
}

/// Serialise an IR script (a list of statement nodes). Children are emitted
/// only for If/For/Switch.
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

/// Serialise the `wasm` view: drive the eval-fallback WASM emitter (the same
/// `wasm_codegen_module` `tcl compwasm` uses) and emit the rich per-instruction
/// explorer shape (`wasm_explorer::wasm_to_explorer_json` — resolved `call`
/// targets, paired `br`/`br_if` targets, block-pairing indices, per-instruction
/// ranges). The synthetic `(module)` entry additionally carries the full WAT
/// `text` and the module-wide counts the TUI `wasm` view renders, so both the
/// text renderer and the web GUI read one shape.
fn serialise_wasm(module: &Module, source: &str) -> Value {
    use tcl_compiler::codegen::wasm::wasm_codegen_module;

    let mut wasm = wasm_codegen_module(module, source);
    let total_instr: usize = wasm.functions.iter().map(|f| f.body.len()).sum();
    let function_count = wasm.functions.len();
    let wat = wasm.to_wat();

    // Rich per-instruction entries (module header first, then functions).
    let li = LineIndex::new(source);
    let mut entries = crate::wasm_explorer::wasm_to_explorer_json(&wasm, &li, source);

    // Augment the synthetic `(module)` header with the WAT text + counts the
    // TUI renderer needs (it reads `text` on the module entry and
    // `name`/`instrCount` on each function entry, both already present).
    if let Some(Value::Object(header)) = entries.first_mut() {
        header.insert("text".to_owned(), Value::String(wat));
        header.insert("functionCount".to_owned(), json!(function_count));
        header.insert("totalInstrCount".to_owned(), json!(total_instr));
    }
    Value::Array(entries)
}

/// Serialise the `ir` view.:
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

/// Serialise a block terminator.
fn terminator_dict(
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
        }) => json!({
            "type": "branch",
            "condition": preview(&render_expr(condition), 80),
            "trueTarget": func.block_name(*true_target),
            "falseTarget": func.block_name(*false_target),
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
/// exception edges).
fn block_successors<'a>(func: &'a Function, term: Option<&Terminator>) -> Vec<&'a str> {
    term.map(Terminator::successors)
        .unwrap_or_default()
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
        .snapshots()
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
                "name": snap.name,
                "entry": entry_name,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// Format a declared arity's arity string:
/// `"{min}+"` when unlimited, else `"{min}..{max}"`.
fn arity_str(arity: tcl_registry::Arity) -> String {
    if arity.is_unlimited() {
        format!("{}+", arity.min)
    } else {
        format!("{}..{}", arity.min, arity.max)
    }
}

/// Serialise the `renderedProperties` view: each SSA value's `may` / `must`
/// rendered-property flag names, per function. Values with no flags are
/// skipped, and functions with no entries are omitted. `iter_names()` yields
/// the set named flags in declaration order (NONE excluded).
#[must_use]
pub fn serialise_rendered_properties(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
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
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Renderer severity for a shimmer/thunking code. `S102` is the sole
/// danger-level code (rendered as `error`); all others are `warning`.
fn shimmer_severity(code: &str) -> &'static str {
    if code == "S102" { "error" } else { "warning" }
}

/// Serialise the `shimmer` view: intrep-shimmer (S100/S101), loop-thunking
/// (S102), and shared-value copy-on-write (S103) warnings, combining
/// the warning kinds into one list. This view is strictly gated by the
/// differential harness.
#[must_use]
pub fn serialise_shimmer(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let mut out: Vec<Value> = Vec::new();
    for w in find_shimmer_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "fromType": type_name(w.from_type),
            "toType": type_name(w.to_type),
            "command": w.command,
            "inLoop": w.in_loop,
        }));
    }
    for w in find_thunking_warnings_for_cu(&result.unit) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "typeA": type_name(w.type_a),
            "typeB": type_name(w.type_b),
        }));
    }
    // Shared-value copy-on-write (S103) — a mutation that duplicates the
    // whole value because another live variable still holds it.
    for w in find_sharing_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "sharedWith": w.shared_with,
            "command": w.command,
        }));
    }
    // Byte-array corruption (S110) — a correctness shimmer with the same
    // value-shape shape as the S100/S101 family.
    for w in find_byte_array_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "fromType": type_name(w.from_type),
            "toType": type_name(w.to_type),
            "command": w.command,
            "inLoop": w.in_loop,
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
/// LICM). — `{code, message, expression, range,
/// firstRange, severity: info}`. Composes the three `*_for_cu`
/// finders and de-duplicates on `(code, span, first_span)`. Optimiser-
/// derived, pinned by a Rust unit test.
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
                w.code,
                w.span.start(),
                w.span.end(),
                w.first_span.start(),
                w.first_span.end(),
            ))
        })
        .map(|w| {
            json!({
                "code": w.code.as_str(),
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

/// Renderer severity for a taint code.
/// (`T1*` prefix or `T3001`-`T3004` → error, else warning).
fn taint_severity(code: &str) -> &'static str {
    if code.starts_with("T1") || matches!(code, "T3001" | "T3002" | "T3003" | "T3004") {
        "error"
    } else {
        "warning"
    }
}

/// Serialise the `taint` view: information-flow sink warnings. Composes the
/// taint passes via `find_taint_warnings_for_cu`.
#[must_use]
pub fn serialise_taint(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let dialect = Some(result.dialect.as_str());
    let out: Vec<Value> = find_taint_warnings_for_cu(&result.unit, registry, dialect)
        .iter()
        .map(|w| {
            json!({
                "code": w.code.as_str(),
                "message": w.message,
                "range": range_dict(w.span, li, source),
                "severity": taint_severity(w.code.as_str()),
                "variable": w.variable,
                "sinkCommand": w.sink_command,
            })
        })
        .collect();
    Value::Array(out)
}

/// Serialise the `optimisations` view: the optimiser rewrites found for
/// the source. — `{code, message,
/// range, replacement}` per rewrite. Runs the `optimise` pass over
/// the cached per-dialect registry.
#[must_use]
pub fn serialise_optimisations(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let opts: Vec<Value> = optimise(source, registry)
        .iter()
        .map(|o| {
            json!({
                "code": o.code.as_str(),
                "message": o.message,
                "range": range_dict(o.span, li, source),
                "replacement": o.replacement,
            })
        })
        .collect();
    Value::Array(opts)
}

/// Serialise the Rust-native `optimiserPasses` view: each optimiser pass in
/// `PassId::all()` order with the optimisations it produced (raw, before the
/// overlap arbitration the `optimisations` view applies).
///
/// **Rust-native** — surfaces the actual pipeline: which pass found what, in
/// execution order.
#[must_use]
pub fn serialise_optimiser_passes(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let passes: Vec<Value> = optimise_by_pass(&result.unit, registry, Some(&result.dialect))
        .iter()
        .map(|(pass, opts)| {
            let optimisations: Vec<Value> = opts
                .iter()
                .map(|o| {
                    json!({
                        "code": o.code.as_str(),
                        "message": o.message,
                        "range": range_dict(o.span, li, source),
                        "replacement": o.replacement,
                    })
                })
                .collect();
            json!({
                "id": pass.as_str(),
                "label": pass.label(),
                "count": optimisations.len(),
                "optimisations": optimisations,
            })
        })
        .collect();
    Value::Array(passes)
}

/// Serialise the `intervals` view: the integer-interval domain per tracked
/// SSA value, per function. — only bounded
/// (non-top) ranges are emitted; `lo`/`hi` are `null` for ±infinity.
#[must_use]
pub fn serialise_intervals(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
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
            json!({ "name": snap.name, "entries": entries })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `taintTracking` view: every tainted SSA value's taint
/// lattice per function. — only
/// tainted entries, functions with none omitted.
#[must_use]
pub fn serialise_taint_tracking(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .snapshots()
        .iter()
        .filter_map(|snap| {
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.taints.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let tl = &snap.unit.taints[*key];
                    tl.colours
                        .contains(tcl_compiler::taint::TaintColour::TAINTED)
                        .then(|| json!({ "variable": ssa.var_name(key.0), "version": key.1, "taint": format_taint(tl) }))
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "entries": entries }))
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
            && tl.kind != tcl_compiler::types::TypeKind::Unknown
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
    let registry = registry_for_dialect(&result.dialect);
    let funcs: Vec<Value> = result
        .snapshots()
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
                        .map(|sb| {
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
                "name": snap.name,
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
            tl.kind,
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

/// Serialise the `dataflow` view: the def-use data-flow graph, built over
/// `extract_dataflow_graph`.
///
/// `extract_dataflow_graph` sorts functions, and alias info comes from
/// memory-SSA (not built here, so `aliases` are limited); the node/edge
/// detail follows the Rust analyses.
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
            types: Some(&s.unit.types),
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

/// Serialise the `irulesFlow` view: iRules flow / performance warnings —
/// `{code, message, range, severity}`, composing the five `irules_checks`
/// finders. Empty for non-iRules dialects (an empty list on the tcl corpus).
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
                "code": w.code.as_str(),
                "message": w.message,
                "range": range_dict(w.span, li, source),
                "severity": "warning",
            })
        })
        .collect();
    Value::Array(out)
}

/// Serialise the `loops` view: the natural-loop forest per function, with
/// nesting depth, built over
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
            json!({ "name": snap.name, "loops": loops })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `bounds` view: interval-driven out-of-range findings per
/// function.
///
/// Both interval-driven passes share the same SCCP-executable-block filter:
/// `find_interval_bounds` (W230/W231/W232 out-of-range index access) and
/// `find_divide_by_zero` (W233 provably-`[0,0]` divisor).
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
                    "code": f.code.as_str(),
                    "command": f.command,
                    "indexVar": f.index_var,
                    "lo": f.index_interval.lo,
                    "hi": f.index_interval.hi,
                    "length": f.length,
                    "reason": f.reason,
                })
            })
            .collect();
            let divzero: Vec<Value> = find_divide_by_zero(
                &snap.unit.cfg,
                &snap.unit.ssa,
                &snap.unit.sccp.values,
                &snap.unit.sccp.executable_blocks,
            )
            .iter()
            .map(|d| json!({ "code": "W233", "op": d.op }))
            .collect();
            json!({ "name": snap.name, "findings": findings, "divzero": divzero })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `interprocedural` view: per-procedure summaries followed
/// by `TclOO` method summaries.
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
/// function, including `?`/`*` entries.
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
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.types.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            for key in keys {
                let tl = &snap.unit.types[key];
                entries.push(json!({
                    "variable": ssa.var_name(key.0),
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
///
/// `name` is the first word's text; `subcommand` is always `null` (the CST
/// segment path does not resolve subcommands). `braced` is whether the
/// word's first fragment is a braced `{…}` (`Str`) token; `quoted` whether
/// its raw begins with `"` — both derived from the representative token's
/// first fragment.
#[must_use]
pub fn serialise_segments(source: &str, config: LexerConfig) -> Value {
    let bytes = source.as_bytes();
    let segments: Vec<Value> = segment_commands_with_offset_and_config(source, 0, config)
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
/// handlers in canonical firing order.
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
        // The body (last word) must be a braced block (`Str`): the guard
        // requires `body.kind == TokenType::Str`.
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
        // keeps the 500 default.
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

/// Serialise the Rust-native `structuralIndex` view: the lexer's structural
/// pre-scan (`tcl_lexer::structural_index`) — where commands begin, the
/// bracket/brace balance, and the inert spans where `[`/`]`/`{`/`}` are
/// *literal* (brace words, comments, `${…}`, escapes). This acceleration
/// layer drives incremental reparse.
#[must_use]
pub fn serialise_structural_index(source: &str, li: &LineIndex) -> Value {
    use tcl_lexer::{BraceIndex, BracketIndex, command_boundaries, script_is_complete};

    fn pos(li: &LineIndex, off: u32) -> Value {
        let p = li.position_at(off);
        json!({ "line": p.line, "col": p.character.get(), "offset": p.offset })
    }
    fn inert_spans(li: &LineIndex, source: &str, spans: &[(u32, u32, bool)]) -> Value {
        Value::Array(
            spans
                .iter()
                .map(|&(start, end, terminated)| {
                    let (s, e) = (start as usize, end as usize);
                    json!({
                        "start": start,
                        "end": end,
                        "terminated": terminated,
                        "startPos": pos(li, start),
                        "text": source.get(s..e).unwrap_or(""),
                    })
                })
                .collect(),
        )
    }

    let brackets = BracketIndex::build(source);
    let braces = BraceIndex::build(source);
    let boundaries: Vec<Value> = command_boundaries(source)
        .iter()
        .map(|&off| pos(li, off))
        .collect();

    json!({
        "scriptComplete": script_is_complete(source),
        "commandBoundaries": boundaries,
        "brackets": {
            "unterminated": brackets.unterminated_count(),
            "structuralEvents": brackets.events().len(),
            "inertSpans": inert_spans(li, source, brackets.inert_spans()),
        },
        "braces": {
            "unterminated": braces.unterminated_count(),
            "structuralEvents": braces.events().len(),
            "inertSpans": inert_spans(li, source, braces.inert_spans()),
        },
    })
}

/// Serialise the Rust-native `sourceMap` view: the `LineIndex` span model
/// that powers O(1) offset ↔ line:col resolution (`tcl_lexer::SourceMap` /
/// `LineIndex`). Surfaces the line-start table the analyses resolve every
/// span through — the reference for debugging range/offset bugs.
#[must_use]
pub fn serialise_source_map(source: &str, li: &LineIndex) -> Value {
    let byte_length = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let line_count = u32::try_from(li.line_count()).unwrap_or(u32::MAX);
    let lines: Vec<Value> = (0..line_count)
        .map(|line| {
            let start = li.line_start(line);
            let end = if line + 1 < line_count {
                li.line_start(line + 1)
            } else {
                byte_length
            };
            let text = source
                .get(start as usize..end as usize)
                .unwrap_or("")
                .trim_end_matches('\n');
            json!({
                "line": line,
                "start": start,
                "end": end,
                "length": end - start,
                "text": text,
            })
        })
        .collect();
    json!({
        "byteLength": byte_length,
        "lineCount": line_count,
        "lines": lines,
    })
}

/// Serialise the `stats` summary.
///
/// `deadStores` counts the optimiser's **O109** dead stores (there is no
/// standalone liveness pass) and the warning counts come from the Rust
/// analyses. `dataflow*` counts are omitted — `dataflow` is not implemented,
/// and such counts only apply when a dataflow graph is present.
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
        + find_thunking_warnings_for_cu(&result.unit).len()
        + find_byte_array_warnings_for_cu(&result.unit, registry).len();
    let mut gvn = find_redundancies_for_cu(&result.unit, registry, dialect);
    gvn.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    gvn.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));
    let taint = find_taint_warnings_for_cu(&result.unit, registry, dialect).len();
    let rewrites = optimise(&result.source, registry).len();
    // Dead stores are O109 optimiser findings (no standalone liveness pass).
    let dead_stores = find_dead_stores(&result.unit, registry, dialect).len();

    json!({
        "procedures": result.unit.ir_module.procedures.len(),
        "functions": result.snapshots().len(),
        "blocks": result.total_blocks(),
        "deadStores": dead_stores,
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
/// If/For/Switch bodies only.
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
/// Collects all sources (GUI default), then emits per-annotation entries
/// grouped by line.
///
/// Aggregates the optimiser / shimmer / gvn / taint sources. Dead-store
/// callouts come from the optimiser's **O109** findings (there is no
/// standalone liveness pass); constant-branch + unreachable-block callouts
/// come from `sccp`.
// One flat pass aggregating every optimiser/shimmer/gvn/taint callout by line;
// splitting the per-source arms would scatter one contract across helpers.
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

    // Dead stores (O109 optimiser findings — Rust has no standalone liveness
    // pass), constant branches, and unreachable blocks, per function.
    let dead_stores = find_dead_stores(&result.unit, registry, dialect);
    for snap in result.snapshots() {
        for dead in dead_stores.iter().filter(|d| d.function == snap.name) {
            let Some(block) = snap.unit.cfg.block_by_name(&dead.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(dead.statement_index) else {
                continue;
            };
            if let Some(stmt) = block.statements.get(idx) {
                anns.push(Ann {
                    span: stmt.span(),
                    label: format!(
                        "{}: dead store {}#{}",
                        snap.name, dead.variable, dead.version
                    ),
                    kind: "deadStore",
                    severity: "warning",
                    priority: 1,
                });
            }
        }
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
        let mut unreachable: Vec<tcl_compiler::cfg::BlockId> = snap
            .unit
            .cfg
            .blocks
            .keys()
            .copied()
            .filter(|b| !snap.unit.sccp.executable_blocks.contains(b))
            .collect();
        unreachable.sort_unstable();
        for bn in unreachable {
            let block = &snap.unit.cfg.blocks[&bn];
            let bn_name = snap.unit.cfg.block_name(bn);
            let span = block
                .statements
                .first()
                .map(Statement::span)
                .or_else(|| block.terminator.as_ref().and_then(Terminator::span));
            if let Some(span) = span {
                anns.push(Ann {
                    span,
                    label: format!("{}: unreachable block {bn_name}", snap.name),
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
        let sev = shimmer_severity(w.code.as_str());
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "shimmer",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    for w in find_thunking_warnings_for_cu(&result.unit) {
        let sev = shimmer_severity(w.code.as_str());
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "thunking",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    for w in find_byte_array_warnings_for_cu(&result.unit, registry) {
        let sev = shimmer_severity(w.code.as_str());
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "shimmer",
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
            severity: taint_severity(w.code.as_str()),
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
// Assembles the whole explorer JSON contract field-by-field in one place;
// each stage adds one top-level key, so the length is inherent to the schema.
#[must_use]
#[allow(clippy::too_many_lines)]
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
    // Honour the document's dialect so the CST and segment views tokenise
    // `{*}` / iRules braces the same way the rest of the pipeline does.
    let lexer_config = LexerConfig::for_dialect(&result.dialect);
    out.insert(
        "cst".to_owned(),
        crate::cst::serialise_cst(&result.source, lexer_config),
    );
    out.insert(
        "segments".to_owned(),
        serialise_segments(&result.source, lexer_config),
    );
    // Rust-native: the lexer structural pre-scan.
    out.insert(
        "structuralIndex".to_owned(),
        serialise_structural_index(&result.source, &li),
    );
    // Rust-native: the LineIndex span model.
    out.insert(
        "sourceMap".to_owned(),
        serialise_source_map(&result.source, &li),
    );
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
    // Rust-native: the optimiser pass pipeline.
    out.insert(
        "optimiserPasses".to_owned(),
        serialise_optimiser_passes(result, &li, &result.source),
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

    // WASM views: drive the eval-fallback WASM emitter (the same one `tcl
    // compwasm` uses) and surface its WAT plus the rich per-instruction
    // explorer shape (resolved `call`/branch targets, per-instruction ranges)
    // alongside per-function headers, which the text/`wasm` view renders.
    out.insert(
        "wasm".to_owned(),
        serialise_wasm(&result.unit.ir_module, &result.source),
    );
    out.insert(
        "wasmOptimised".to_owned(),
        opt.as_ref()
            .map_or(Value::Null, |(r, s)| serialise_wasm(&r.unit.ir_module, s)),
    );

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
        // 16 dialects: the prior 15 + `tcl9.1` (the Tcl 9.1 sync).
        assert_eq!(meta["dialects"].as_array().unwrap().len(), 16);
        // 26 views: the base 24 minus the dropped `greentree` tab (Rust has a
        // single red-green CST) plus the Rust-native `structuralIndex`,
        // `sourceMap`, and `optimiserPasses` views.
        assert_eq!(meta["views"].as_array().unwrap().len(), 26);
        assert_eq!(meta["severities"], json!(["error", "warning", "info"]));
        // The parse-tree tab is the CST; there is no `greentree` entry.
        assert_eq!(
            meta["views"][0],
            json!({ "id": "cst", "label": "CST", "group": "compiler" })
        );
        assert!(
            !meta["views"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["id"] == "greentree"),
            "greentree tab must be dropped"
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
        // SCCP proves `x` is `3` and its only use is propagated, so the
        // computed def couple-removes — the result is
        // `puts 3`, not `set x 3`.
        let s = opt.as_str().expect("optimised source string");
        assert!(s.contains("puts 3"), "{s:?}");
        assert!(!s.contains("set x"), "{s:?}");
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
    fn dead_stores_surface_from_o109() {
        // `set x 1` is overwritten by `set x 2` before any read — the
        // optimiser's O109 finding surfaces in cfgPostSsa.analysis, stats,
        // and as a deadStore callout (Rust has no standalone liveness pass).
        let result = run_pipeline("proc f {} { set x 1; set x 2; return $x }", "tcl8.6");
        let value = serialise_result(&result);

        let dead: Vec<&Value> = value["cfgPostSsa"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|fn_| fn_["analysis"]["deadStores"].as_array().unwrap())
            .collect();
        assert_eq!(dead.len(), 1, "one dead store");
        assert_eq!(dead[0]["variable"], "x");
        assert_eq!(dead[0]["version"], 1);

        assert_eq!(value["stats"]["deadStores"], 1);
        assert!(
            value["annotations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["kind"] == "deadStore"),
            "a deadStore callout is emitted"
        );
    }

    #[test]
    fn no_dead_stores_when_value_is_read() {
        let result = run_pipeline("proc f {} { set x 1; return $x }", "tcl8.6");
        let value = serialise_result(&result);
        assert_eq!(value["stats"]["deadStores"], 0);
    }

    #[test]
    fn structural_index_reports_boundaries_and_balance() {
        // Two complete commands, balanced brackets/braces.
        let result = run_pipeline("set x {a b c}\nputs [llength $x]", "tcl8.6");
        let si = serialise_result(&result)["structuralIndex"].clone();
        assert_eq!(si["scriptComplete"], true);
        assert_eq!(si["commandBoundaries"].as_array().unwrap().len(), 2);
        assert_eq!(si["brackets"]["unterminated"], 0);
        assert_eq!(si["braces"]["unterminated"], 0);
    }

    #[test]
    fn structural_index_flags_incomplete_script() {
        // An unclosed brace leaves the script incomplete with an unterminated
        // inert span.
        let result = run_pipeline("proc f {} {\n  set x 1", "tcl8.6");
        let si = serialise_result(&result)["structuralIndex"].clone();
        assert_eq!(si["scriptComplete"], false);
        assert!(si["braces"]["unterminated"].as_i64().unwrap() >= 1);
    }

    #[test]
    fn source_map_maps_lines_to_byte_ranges() {
        let result = run_pipeline("set x 1\nputs $x\n", "tcl8.6");
        let sm = serialise_result(&result)["sourceMap"].clone();
        assert_eq!(sm["lineCount"], 3); // two lines + the trailing empty line
        let lines = sm["lines"].as_array().unwrap();
        assert_eq!(lines[0]["start"], 0);
        assert_eq!(lines[0]["text"], "set x 1");
        // Line 1 starts right after the first newline.
        assert_eq!(lines[1]["start"], 8);
        assert_eq!(lines[1]["text"], "puts $x");
        // Byte length covers the whole source.
        assert_eq!(sm["byteLength"], 16);
    }

    #[test]
    fn optimiser_passes_lists_every_pass_in_order() {
        // The Rust-native pass-pipeline view: all 9 passes in execution
        // order, each with the optimisations it produced.
        let result = run_pipeline("set x 1\nset y [expr {1 + 2}]\nputs $x$y", "tcl8.6");
        let passes = serialise_result(&result)["optimiserPasses"].clone();
        let arr = passes.as_array().unwrap();
        assert_eq!(arr.len(), 9, "all nine passes are listed");
        // Execution order is fixed (propagation first).
        assert_eq!(arr[0]["id"], "propagation");
        assert_eq!(arr[1]["id"], "branch_folding");
        // Each entry carries a label, count, and the optimisations array.
        for p in arr {
            assert!(p["label"].is_string());
            let opts = p["optimisations"].as_array().unwrap();
            assert_eq!(p["count"].as_u64().unwrap(), opts.len() as u64);
        }
        // Propagation folds the constant var-ref / interpolation here.
        let prop = &arr[0];
        assert!(prop["count"].as_u64().unwrap() >= 1);
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
        // RULE_INIT and HTTP_RESPONSE. Verified byte-for-byte for this snippet.
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
        // `0 < 10` is statically true on entry, so analysis rotates the loop
        // (FP-RBS-18): the entry guard `for_header` moves out of the natural
        // loop and the back-edge target — the natural-loop header — becomes the
        // `for_body` block, with the `for_step` re-check as the latch.
        let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
        let loops = serialise_result(&result)["loops"].clone();
        let top = &loops.as_array().unwrap()[0];
        let lps = top["loops"].as_array().unwrap();
        assert_eq!(lps.len(), 1, "one natural loop for a single for-loop");
        let lp = &lps[0];
        assert_eq!(lp["depth"], 1);
        let header = lp["header"].as_str().unwrap();
        assert!(
            header.contains("for_body") || header.contains("header"),
            "rotated for-loop header is the body block, got {header}"
        );
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
        // A by-name read (`[set x]`) keeps the def alive, so the O101
        // constant-fold survives as its own reported rewrite rather than
        // being superseded by the dead-store coupling.
        let result = run_pipeline("set x [expr {1 + 2}]\nputs [set x]", "tcl8.6");
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
        let segs = serialise_segments("string length \"hi\"", LexerConfig::default());
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
        let segs = serialise_segments("if {$x} { puts hi }", LexerConfig::default());
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
