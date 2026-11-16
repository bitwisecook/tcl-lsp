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

//! The docstring tools: generate one, read a procedure's documentation and update a file's docstrings.

use super::{analyse, arg_bool, arg_str, resolve_dialect};
use serde_json::{Map, Value, json};
use tcl_lexer::LineIndex;

/// Split `s` into lines keeping each trailing `\n` (like
/// `splitlines(keepends=True)` over `\n`); re-joining with `concat()` is
/// loss-free. Mirrors the `insert_docstring_stubs` facade helper.
fn split_keep_ends(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        if b == b'\n' {
            out.push(s[start..=i].to_owned());
            start = i + 1;
        }
    }
    if start < s.len() {
        out.push(s[start..].to_owned());
    }
    out
}

// ── Docstring tools ───────────────────────────────────────────────────

pub(super) fn generate_docstring(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let proc_name = arg_str(args, "proc_name");
    let style = match arg_str(args, "style") {
        "" => "doxygen",
        s => s,
    };
    let decoration = arg_bool(args, "decoration");
    let analysis = analyse(source, &dialect);
    let Some(declarations) =
        tcl_lsp_core::procedure_symbol::for_literal_name(source, &analysis, proc_name)
    else {
        return json!({ "error": "The current original procedure inventory is unavailable" });
    };
    match declarations.as_slice() {
        [proc] => {
            let tag = tcl_lsp_core::formatting::resolve_tag_style(style);
            let docstring = tcl_lsp_core::formatting::generate_stub_for_proc(
                proc, tag, decoration, '.', 70, "",
            );
            json!({ "proc": proc_name, "docstring": docstring })
        }
        [] => json!({
            "error": format!("Proc '{proc_name}' not found at global scope; use its qualified name for a namespace declaration"),
        }),
        _ => json!({
            "error": format!("Proc '{proc_name}' has multiple source declarations"),
            "declarations": declarations.iter().map(|proc| json!({
                "name_start": proc.name_span.start(),
                "name_end": proc.name_span.end(),
            })).collect::<Vec<_>>(),
        }),
    }
}

pub(super) fn read_proc_docs(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let Some(procs) = tcl_lsp_core::procedure_symbol::declarations(source, &analysis) else {
        return json!({ "error": "The current original procedure inventory is unavailable" });
    };
    let entries: Vec<Value> = procs
        .into_iter()
        .map(|proc| {
            let params: Vec<Value> = proc
                .params
                .iter()
                .map(|p| {
                    if p.has_default {
                        json!({ "name": p.name, "default": p.default_value })
                    } else {
                        json!({ "name": p.name })
                    }
                })
                .collect();
            let mut entry =
                json!({ "name": proc.name, "qualified_name": proc.qualified_name, "params": params });
            let m = entry.as_object_mut().expect("json object");
            if let Some(original) = analysis
                .original_procedure_declarations()
                .find(|row| row.metadata().name_span == proc.name_span)
            {
                m.insert(
                    "name_source".to_owned(),
                    json!(source.get(original.name_input().span().as_range())),
                );
            }
            m.insert("name_start".to_owned(), json!(proc.name_span.start()));
            m.insert("name_end".to_owned(), json!(proc.name_span.end()));
            if proc.doc.is_empty() {
                m.insert("doc".to_owned(), Value::Null);
            } else {
                m.insert("doc_raw".to_owned(), json!(proc.doc));
                m.insert(
                    "doc".to_owned(),
                    tcl_lsp_core::formatting::parse_docstring(&proc.doc).to_json(),
                );
            }
            if !proc.param_traits.is_empty() {
                let mut traits = Map::new();
                for (name, set) in &proc.param_traits {
                    let mut names: Vec<&str> = set.iter().map(|t| t.as_str()).collect();
                    names.sort_unstable();
                    traits.insert(name.clone(), json!(names));
                }
                m.insert("param_traits".to_owned(), Value::Object(traits));
            }
            entry
        })
        .collect();
    json!({ "procs": entries })
}

pub(super) fn update_docstrings(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let style = match arg_str(args, "style") {
        "" => "doxygen",
        s => s,
    };
    let decoration = arg_bool(args, "decoration");
    let analysis = analyse(source, &dialect);
    let line_index = LineIndex::new(source);
    let tag = tcl_lsp_core::formatting::resolve_tag_style(style);
    let Some(procs) = tcl_lsp_core::procedure_symbol::declarations(source, &analysis) else {
        return json!({ "error": "The current original procedure inventory is unavailable" });
    };
    let total_procs = procs.len();

    // Insert from the last original declaration so earlier source positions
    // stay valid while each independent declaration receives its own comment.
    let mut targets: Vec<_> = procs
        .into_iter()
        .filter(|proc| proc.doc.is_empty())
        .collect();
    targets.sort_by_key(|p| std::cmp::Reverse(line_index.line_at(p.name_span.start())));

    let mut lines: Vec<String> = split_keep_ends(source);
    for proc in &targets {
        let stub =
            tcl_lsp_core::formatting::generate_stub_for_proc(proc, tag, decoration, '.', 70, "");
        let at = (line_index.line_at(proc.name_span.start()) as usize).min(lines.len());
        lines.insert(at, format!("{stub}\n"));
    }
    json!({
        "source": lines.concat(),
        "procs_documented": targets.len(),
        "total_procs": total_procs,
    })
}
