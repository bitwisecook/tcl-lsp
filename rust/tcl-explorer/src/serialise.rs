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
use tcl_compiler::interprocedural::InterproceduralAnalysis;
use tcl_compiler::intervals::compute_intervals;
use tcl_compiler::ir::{Module, Script, Statement};
use tcl_compiler::segmenter::segment_commands;
use tcl_lexer::{LineIndex, Span, TokenType};
use tcl_registry::available_dialects;
use tcl_syntax::expr::ast::render_expr;

use crate::ExplorerResult;
use crate::formatters::{
    format_return_shape, format_type, preview, range_dict, stmt_color_class, stmt_kind,
    stmt_summary, type_kind_name,
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
    if let Some(interproc) = &result.unit.interproc {
        out.insert("interprocedural".to_owned(), serialise_interproc(interproc));
    }
    out.insert("types".to_owned(), serialise_types(result));
    out.insert("segments".to_owned(), serialise_segments(&result.source));
    out.insert("intervals".to_owned(), serialise_intervals(result));
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
