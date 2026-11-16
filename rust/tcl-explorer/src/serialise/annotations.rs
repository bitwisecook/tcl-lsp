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

//! The `annotations` source-callout views, with the compiler barriers they mark.

use super::retained_explorer_source;
use super::warnings::{shimmer_severity, taint_severity};
use crate::formatters::{preview, range_dict};
use crate::{ExplorerResult, FunctionSnapshotKind};
use serde_json::{Map, Value, json};
use tcl_compiler::cfg::Terminator;
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::ir::{Script, Statement};
use tcl_compiler::optimiser::{find_dead_stores, optimise_unit};
use tcl_compiler::shimmer::{
    find_byte_array_warnings_for_cu, find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu,
};
use tcl_compiler::taint::taint_warning_report_for_cu;
use tcl_lexer::{LineIndex, Span};

/// One source-callout annotation, before serialisation.
struct Ann {
    pub(super) span: Span,
    pub(super) label: String,
    pub(super) kind: &'static str,
    severity: &'static str,
    pub(super) priority: i32,
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
pub(super) fn serialise_annotations(
    result: &ExplorerResult,
    li: &LineIndex,
    source: &str,
) -> (Value, Value) {
    let Some(retained) = retained_explorer_source(result, source) else {
        return (Value::Null, Value::Null);
    };
    let registry = retained.registry;
    let dialect = retained.dialect;
    let mut anns: Vec<Ann> = Vec::new();

    // Barriers (IR walk).
    walk_barriers(&result.unit.ir_module.top_level, "::top", &mut anns);
    for procedure in result.unit.ir_module.procedures.values() {
        walk_barriers(&procedure.body, &procedure.qualified_name, &mut anns);
    }

    // Dead stores (O109 optimiser findings — Rust has no standalone liveness
    // pass), constant branches, and unreachable blocks, per function.
    let dead_stores = find_dead_stores(&result.unit, registry, dialect);
    for snap in result.all_snapshots() {
        for dead in dead_stores.iter().filter(|dead| match dead.body_id {
            Some(id) => snap.unit.body_id == Some(id),
            None => dead.is_top_level && snap.kind == FunctionSnapshotKind::TopLevel,
        }) {
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
                let label = if branch.kind == tcl_compiler::sccp::BranchFactKind::Selected {
                    format!(
                        "{}: arm '{}' is never selected",
                        snap.name, branch.condition
                    )
                } else {
                    format!(
                        "{}: branch is always {dir}; takes {}",
                        snap.name, branch.taken_target
                    )
                };
                anns.push(Ann {
                    span,
                    label,
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
    for o in optimise_unit(&result.unit, registry, dialect) {
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
    for w in find_thunking_warnings_for_cu(&result.unit, registry) {
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
    for w in taint_warning_report_for_cu(&result.unit, registry, dialect).warnings {
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
