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

//! The views that list warnings: shimmer and thunking, taint sinks with their replays, and interval bounds.

use super::{retained_explorer_source, snapshot_body_key};
use crate::ExplorerResult;
use crate::formatters::range_dict;
use serde_json::{Value, json};
use tcl_compiler::interval_bounds::{
    BoundsRelease, BoundsSemantics, find_divide_by_zero_with_entered_operands,
    find_interval_bounds_in_release,
};
use tcl_compiler::shimmer::{
    find_byte_array_warnings_for_cu, find_sharing_warnings_for_cu, find_shimmer_warnings_for_cu,
    find_thunking_warnings_for_cu, type_name,
};
use tcl_compiler::taint::{TaintReplayAnalysis, taint_warning_report_for_cu};
use tcl_lexer::LineIndex;

/// Renderer severity for a shimmer/thunking code. `S102` is the sole
/// danger-level code (rendered as `error`); all others are `warning`.
pub(super) fn shimmer_severity(code: &str) -> &'static str {
    if code == "S102" { "error" } else { "warning" }
}

/// Serialise the `shimmer` view: intrep-shimmer (S100/S101), loop-thunking
/// (S102), and shared-value copy-on-write (S103) warnings, combining
/// the warning kinds into one list. This view is strictly gated by the
/// differential harness.
#[must_use]
pub fn serialise_shimmer(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let registry = retained.registry;
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
    for w in find_thunking_warnings_for_cu(&result.unit, registry) {
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

/// Renderer severity for a taint code.
/// (`T1*` prefix or `T3001`-`T3004` → error, else warning).
pub(super) fn taint_severity(code: &str) -> &'static str {
    if code.starts_with("T1") || matches!(code, "T3001" | "T3002" | "T3003" | "T3004") {
        "error"
    } else {
        "warning"
    }
}

/// Serialise proven information-flow sink warnings from the shared report.
#[must_use]
pub fn serialise_taint(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let registry = retained.registry;
    let dialect = retained.dialect;
    let report = taint_warning_report_for_cu(&result.unit, registry, dialect);
    let out: Vec<Value> = report
        .warnings
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

pub(super) fn taint_replay_status(replays: &TaintReplayAnalysis) -> &'static str {
    match replays {
        TaintReplayAnalysis::Analysed { .. } => "analysed",
        TaintReplayAnalysis::Unavailable { .. } => "unavailable",
    }
}

pub(super) fn serialise_taint_replays(
    result: &ExplorerResult,
    li: &LineIndex,
    source: &str,
) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let report = taint_warning_report_for_cu(&result.unit, retained.registry, retained.dialect);
    let unavailable = report
        .replays
        .unavailable_replays()
        .iter()
        .map(|replay| {
            let current = replay.source.as_ref().is_some_and(|snapshot| {
                snapshot.matches_module(&result.unit.ir_module, retained.registry)
            });
            let range = if current {
                range_dict(replay.span, li, source)
            } else {
                Value::Null
            };
            json!({
                "sourceSpan": current.then_some([replay.span.start(), replay.span.end()]),
                "replaySpan": [replay.span.start(), replay.span.end()],
                "range": range,
                "reason": replay.reason.as_str(),
                "sourceCurrent": current,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "scope": "command-heads-and-stored-callbacks",
        "status": taint_replay_status(&report.replays),
        "complete": report.replays.is_complete(),
        "unavailableReplays": unavailable,
    })
}

/// Serialise the `bounds` view: interval-driven out-of-range findings per
/// function.
///
/// Both interval-driven passes share the same SCCP-executable-block filter:
/// `find_interval_bounds_with` (W230/W231/W232 out-of-range index access) and
/// `find_divide_by_zero_with` (W233 provably-`[0,0]` divisor).
#[must_use]
pub fn serialise_bounds(result: &ExplorerResult) -> Value {
    let Some(retained) = retained_explorer_source(result, &result.source) else {
        return Value::Null;
    };
    let funcs: Option<Vec<Value>> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let metadata = snap.unit.invocation_metadata_context_for_module(
                retained.registry,
                &result.unit.ir_module,
            )?;
            let input = snap.unit.source_metadata_input()?;
            let profile = input.unit_profile();
            let numbers = tcl_dialect::NumberSyntax::of_profile(Some(profile));
            let semantics = BoundsSemantics {
                registry: retained.registry,
                context: Some(metadata),
                grammar: snap.unit.source_lexer_config().grammar_over(profile.grammar),
            };
            let findings: Vec<Value> = find_interval_bounds_in_release(
                &snap.unit.cfg,
                &snap.unit.ssa,
                (&snap.unit.sccp.values, (&snap.unit.sccp, &snap.unit.types)),
                &snap.unit.sccp.executable_blocks,
                BoundsRelease {
                    characters: profile.character_model(),
                    numbers,
                    lset_appends_at_length: tcl_dialect::TclVersion::lset_appends_at_length_for(
                        profile.const_fold_version(),
                    ),
                },
                semantics,
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
            let divzero: Vec<Value> = find_divide_by_zero_with_entered_operands(
                &snap.unit.cfg,
                &snap.unit.ssa,
                (&snap.unit.sccp.values, (&snap.unit.sccp, &snap.unit.types)),
                &snap.unit.sccp.executable_blocks,
                numbers,
                semantics,
            )
            .iter()
            .map(|d| json!({ "code": "W233", "op": d.op }))
            .collect();
            Some(json!({ "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap), "kind": snap.kind.as_str(), "findings": findings, "divzero": divzero }))
        })
        .collect();
    funcs.map_or(Value::Null, Value::Array)
}
