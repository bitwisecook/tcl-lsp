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

//! The views of the front end: segments, the structural index, the source map and the IR tree.

use super::{range_or_null, snapshot_body_key};
use crate::ExplorerResult;
use crate::formatters::{preview, range_dict, stmt_color_class, stmt_kind, stmt_summary};
use serde_json::{Map, Value, json};
use tcl_compiler::ir::{Module, Script, Statement};
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, LineIndex, TokenType};
use tcl_syntax::expr::ast::render_expr;

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

/// Serialise the `ir` view.:
/// Procedure rows retain module body IDs independently of their display names.
#[must_use]
pub fn serialise_ir(module: &Module, li: &LineIndex, source: &str) -> Value {
    let mut procedures = Map::new();
    for (id, proc) in module.procedures.iter() {
        procedures.insert(
            format!("body:{}", id.ordinal()),
            json!({
                "bodyId": id.ordinal(), "bodyKey": id.reporting_key(),
                "name": proc.qualified_name,
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

/// Serialise the Rust-native `structuralIndex` view: the lexer's structural
/// pre-scan (`tcl_lexer::structural_index`) — where commands begin, the
/// bracket/brace balance, and the inert spans where `[`/`]`/`{`/`}` are
/// *literal* (brace words, comments, `${…}`, escapes). This acceleration
/// layer drives incremental reparse.
///
/// `commandBoundaries` here is the **reparse split points**, not the
/// segmentation view: `command_boundaries` is registered in
/// `shared-utility-contracts-rust.md` as a dialect-blind byte scan of
/// stock 9.x structure, so on an F5 or 8.x document it can differ from
/// the dialect-resolved answer — which is what the `segments` and `cst`
/// views beside it show.
#[must_use]
pub fn serialise_structural_index(
    source: &str,
    li: &LineIndex,
    original: &tcl_lexer::SourceImage,
    config: LexerConfig,
) -> Value {
    use tcl_lexer::{BraceIndex, BracketIndex, command_boundaries, script_image_is_complete};

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

    let brackets = BracketIndex::build_with_config(source, config);
    let braces = BraceIndex::build_with_config(source, config);
    let boundaries: Vec<Value> = command_boundaries(source)
        .iter()
        .map(|&off| pos(li, off))
        .collect();

    json!({
        "scriptComplete": (original.bytes() == source.as_bytes())
            .then(|| script_image_is_complete(original, config).ok()).flatten(),
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

pub(super) fn serialise_source_map_units(result: &ExplorerResult) -> Value {
    result
        .all_snapshots()
        .iter()
        .map(|snap| {
            json!({
                "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap),
                "kind": snap.kind.as_str(),
                "baseOffset": snap.unit.base_offset,
            })
        })
        .collect()
}
