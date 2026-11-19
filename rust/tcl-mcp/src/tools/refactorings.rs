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

//! The cursor-addressed refactorings and the `refactor` tool that names one.

use super::{analyse, arg_str, arg_u32, cursor, resolve_dialect};
use serde_json::{Value, json};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Utf16Col};

fn refactoring_json(source: &str, r: &tcl_lsp_core::refactor::Refactoring) -> Value {
    let mut result = json!({
        "title": r.title,
        "rewritten": r.apply(source),
        "edit_count": r.edits.len(),
    });
    if let Some(reason) = &r.disabled {
        result["disabled"] = json!(reason);
    }
    result
}

/// Run a cursor-addressed refactoring, returning its result dict or `null`.
fn refactor_at(
    args: &Value,
    f: impl FnOnce(
        &str,
        u32,
        &AnalysisResult,
        &LineIndex,
    ) -> Option<tcl_lsp_core::refactor::Refactoring>,
) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = analyse(source, &dialect);
    match f(source, off, &analysis, &idx) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

pub(super) fn inline_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = analyse(source, &dialect);
    let Some(context) = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry)
    else {
        return Value::Null;
    };
    match tcl_lsp_core::refactor::inline_variable(
        source,
        off,
        &analysis,
        context.commands().as_ref(),
        &idx,
    ) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

pub(super) fn if_to_switch(args: &Value) -> Value {
    refactor_at(args, tcl_lsp_core::refactor::if_to_switch)
}

pub(super) fn switch_to_dict(args: &Value) -> Value {
    refactor_at(args, tcl_lsp_core::refactor::switch_to_dict)
}

pub(super) fn brace_expr(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (_idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = analyse(source, &dialect);
    match tcl_lsp_core::refactor::brace_expr(source, off, &analysis) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

// ── Refactor tools ────────────────────────────────────────────────────

pub(super) fn extract_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let line_index = LineIndex::new(source);
    let start_off = line_index.offset_at_utf16(
        arg_u32(args, "start_line"),
        Utf16Col::new(arg_u32(args, "start_character")),
        source,
    );
    let end_off = line_index.offset_at_utf16(
        arg_u32(args, "end_line"),
        Utf16Col::new(arg_u32(args, "end_character")),
        source,
    );
    let var_name = match arg_str(args, "var_name") {
        "" => "result",
        v => v,
    };
    match tcl_lsp_core::refactor::extract_variable(
        source,
        (start_off, end_off),
        var_name,
        &analysis,
        &line_index,
    ) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

/// Both tool surfaces keep the analysis owner through conditional source edits.
pub(super) fn datagroup_advice(
    source: &str,
    cursor: u32,
    name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<tcl_lsp_core::refactor::Refactoring> {
    tcl_lsp_core::refactor::extract_to_datagroup_with_analysis(
        source, cursor, name, analysis, line_index,
    )
}

pub(super) fn extract_datagroup(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let (line_index, cursor) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let dg_name = arg_str(args, "dg_name");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let Some(r) = datagroup_advice(source, cursor, dg_name, &analysis, &line_index) else {
        return Value::Null;
    };
    let Some(dg) = r.data_group.as_ref() else {
        return Value::Null;
    };
    let records: Vec<Value> = dg
        .records
        .iter()
        .map(|(k, v)| json!({ "key": k, "value": v }))
        .collect();
    json!({
        "title": r.title,
        "rewritten": r.apply(source),
        "data_group_definition": tcl_lsp_core::refactor::data_group_tcl(dg),
        "data_group": {
            "name": dg.name,
            "value_type": dg.value_type,
            "record_count": dg.records.len(),
            "records": records,
        },
        "edit_count": r.edits.len(),
    })
}

pub(super) fn refactor(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let line_index = LineIndex::new(source);
    let start_off = line_index.offset_at_utf16(
        arg_u32(args, "start_line"),
        Utf16Col::new(arg_u32(args, "start_character")),
        source,
    );
    let end_off = line_index.offset_at_utf16(
        arg_u32(args, "end_line"),
        Utf16Col::new(arg_u32(args, "end_character")),
        source,
    );
    let analysis = analyse(source, &dialect);
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands());
    let mut available: Vec<Value> = Vec::new();
    let mut push = |tool: &str, r: Option<tcl_lsp_core::refactor::Refactoring>| {
        if let Some(r) = r {
            let mut candidate = json!({ "tool": tool, "title": r.title });
            if let Some(reason) = r.disabled {
                candidate["disabled"] = json!(reason);
            }
            available.push(candidate);
        }
    };
    // Extract-variable only applies to a non-empty selection.
    if start_off != end_off {
        push(
            "extract_variable",
            tcl_lsp_core::refactor::extract_variable(
                source,
                (start_off, end_off),
                "result",
                &analysis,
                &line_index,
            ),
        );
    }
    if let Some(commands) = commands {
        push(
            "inline_variable",
            tcl_lsp_core::refactor::inline_variable(
                source,
                start_off,
                &analysis,
                commands.as_ref(),
                &line_index,
            ),
        );
    }
    push(
        "if_to_switch",
        tcl_lsp_core::refactor::if_to_switch(source, start_off, &analysis, &line_index),
    );
    push(
        "switch_to_dict",
        tcl_lsp_core::refactor::switch_to_dict(source, start_off, &analysis, &line_index),
    );
    push(
        "brace_expr",
        tcl_lsp_core::refactor::brace_expr(source, start_off, &analysis),
    );
    push(
        "parenthesise_expression",
        tcl_lsp_core::refactor::parenthesise_expression(source, start_off, &analysis),
    );
    push(
        "remove_redundant_parens",
        tcl_lsp_core::refactor::remove_redundant_parens(source, start_off, &analysis),
    );
    push(
        "extract_datagroup",
        datagroup_advice(source, start_off, "", &analysis, &line_index),
    );
    json!({ "total": available.len(), "available": available })
}
