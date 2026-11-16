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

//! The optimiser's views: the rewrites found, each pass's output and the redundancy hints.

use super::retained_explorer_source;
use crate::ExplorerResult;
use crate::formatters::range_dict;
use serde_json::{Value, json};
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::optimiser::{optimise_by_pass, optimise_unit};
use tcl_lexer::LineIndex;

/// Serialise the `gvn` view: redundant-computation hints (GVN/CSE + PRE +
/// LICM). — `{code, message, expression, range,
/// firstRange, severity: info}`. Composes the three `*_for_cu`
/// finders and de-duplicates on `(code, span, first_span)`. Optimiser-
/// derived, pinned by a Rust unit test.
#[must_use]
pub fn serialise_gvn(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let registry = retained.registry;
    let dialect = retained.dialect;
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

/// Serialise the `optimisations` view: the optimiser rewrites found for
/// the source. — `{code, message,
/// range, replacement}` per rewrite. Uses the actual retained compilation
/// unit and command store; unavailable source ownership returns null.
#[must_use]
pub fn serialise_optimisations(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let opts: Vec<Value> = optimise_unit(&result.unit, retained.registry, retained.dialect)
        .iter()
        .map(|o| {
            json!({
                "code": o.code.as_str(),
                "message": o.message,
                "range": range_dict(o.span, li, source),
                "replacement": o.replacement,
                // Informational rather than actionable, and its span is the
                // whole consuming statement rather than a sub-word. Without
                // this flag a hint-only finding like O102 is indistinguishable
                // from an applicable rewrite and can be read as a one-click
                // fix, so the LSP sends it unconditionally.
                "hintOnly": o.hint_only,
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
    let Some(retained) = retained_explorer_source(result, source) else {
        return Value::Null;
    };
    let passes: Vec<Value> = optimise_by_pass(&result.unit, retained.registry, retained.dialect)
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
                        "hintOnly": o.hint_only,
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
