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

//! Decoders for raw JSON-RPC LSP responses used by the e2e suite.
//!
//! Native port of `tests/lsp_e2e/_lsp_helpers.py`. The harness speaks raw JSON,
//! so every response is a `serde_json::Value`. These helpers normalise the
//! handful of shapes the protocol allows (`Location` vs `LocationLink`, `Hover`
//! content variants, hierarchical document symbols, the semantic-tokens delta
//! encoding) so the feature tests can assert on stable, decoded values.

#![allow(dead_code)]

use serde_json::Value;

/// Flatten a `Hover` result's `contents` to plain text.
pub fn hover_text(hover: &Value) -> String {
    if hover.is_null() {
        return String::new();
    }
    let contents = hover.get("contents").unwrap_or(&Value::Null);
    match contents {
        Value::Object(o) => o
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Object(o) => o
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// A normalised location: `uri` + `range`.
#[derive(Debug, Clone)]
pub struct Loc {
    pub uri: String,
    pub range: Value,
}

/// Normalise a definition/references result to a list of locations, collapsing
/// the `LocationLink` form onto the plain `Location` shape.
pub fn locations(result: &Value) -> Vec<Loc> {
    let items = as_items(result);
    items
        .iter()
        .map(|item| {
            if let Some(target) = item.get("targetUri").and_then(Value::as_str) {
                Loc {
                    uri: target.to_owned(),
                    range: item
                        .get("targetSelectionRange")
                        .or_else(|| item.get("targetRange"))
                        .cloned()
                        .unwrap_or(Value::Null),
                }
            } else {
                Loc {
                    uri: item
                        .get("uri")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                    range: item.get("range").cloned().unwrap_or(Value::Null),
                }
            }
        })
        .collect()
}

/// The `(line, character)` start of every location/highlight in a result.
pub fn starts(result: &Value) -> std::collections::BTreeSet<(i64, i64)> {
    let mut out = std::collections::BTreeSet::new();
    for item in as_items(result) {
        let rng = item
            .get("range")
            .or_else(|| item.get("targetSelectionRange"))
            .or_else(|| item.get("targetRange"));
        if let Some(rng) = rng
            && let Some(s) = rng.get("start")
        {
            let line = s.get("line").and_then(Value::as_i64).unwrap_or(0);
            let ch = s.get("character").and_then(Value::as_i64).unwrap_or(0);
            out.insert((line, ch));
        }
    }
    out
}

/// The set of start lines of every location/highlight in a result.
pub fn start_lines(result: &Value) -> std::collections::BTreeSet<i64> {
    starts(result).into_iter().map(|(l, _)| l).collect()
}

/// The `items` array of a completion result (`CompletionList` or bare array).
pub fn completion_items(result: &Value) -> Vec<Value> {
    match result {
        Value::Object(o) => o
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    }
}

/// The `label` of every completion item.
pub fn completion_labels(result: &Value) -> Vec<String> {
    completion_items(result)
        .iter()
        .map(|item| {
            item.get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

/// Depth-first flatten of a hierarchical `DocumentSymbol` tree.
pub fn flatten_symbols(symbols: &Value) -> Vec<Value> {
    fn walk(items: &Value, out: &mut Vec<Value>) {
        if let Some(arr) = items.as_array() {
            for sym in arr {
                out.push(sym.clone());
                if let Some(children) = sym.get("children") {
                    walk(children, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(symbols, &mut out);
    out
}

/// The set of all symbol names in a (possibly hierarchical) symbol result.
pub fn symbol_names(symbols: &Value) -> std::collections::BTreeSet<String> {
    flatten_symbols(symbols)
        .iter()
        .map(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

/// Return `{uri: [{range, newText}, ...]}` from a `WorkspaceEdit` result.
pub fn rename_edits(result: &Value) -> std::collections::BTreeMap<String, Vec<Value>> {
    let mut out: std::collections::BTreeMap<String, Vec<Value>> = std::collections::BTreeMap::new();
    if result.is_null() {
        return out;
    }
    if let Some(changes) = result.get("changes").and_then(Value::as_object) {
        for (uri, edits) in changes {
            out.insert(uri.clone(), edits.as_array().cloned().unwrap_or_default());
        }
        return out;
    }
    if let Some(doc_changes) = result.get("documentChanges").and_then(Value::as_array) {
        for change in doc_changes {
            if let Some(uri) = change
                .get("textDocument")
                .and_then(|d| d.get("uri"))
                .and_then(Value::as_str)
            {
                let edits = change
                    .get("edits")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                out.entry(uri.to_owned()).or_default().extend(edits);
            }
        }
    }
    out
}

/// One decoded semantic token.
#[derive(Debug, Clone, Copy)]
pub struct SemToken {
    pub line: i64,
    pub char: i64,
    pub length: i64,
    pub ttype: i64,
    pub modifiers: i64,
}

/// Decode the LSP semantic-tokens delta encoding into absolute tokens.
pub fn decode_semantic_tokens(result: &Value) -> Vec<SemToken> {
    let data: Vec<i64> = result
        .get("data")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let mut out = Vec::new();
    let mut line = 0i64;
    let mut char = 0i64;
    for group in data.chunks(5) {
        if group.len() < 5 {
            break;
        }
        let (d_line, d_char, length, ttype, tmods) =
            (group[0], group[1], group[2], group[3], group[4]);
        if d_line != 0 {
            line += d_line;
            char = d_char;
        } else {
            char += d_char;
        }
        out.push(SemToken {
            line,
            char,
            length,
            ttype,
            modifiers: tmods,
        });
    }
    out
}

/// True for an LSP `Range` dict: `{start:{line,character}, end:{...}}`.
fn looks_like_range(obj: &Value) -> bool {
    let (Some(start), Some(end)) = (obj.get("start"), obj.get("end")) else {
        return false;
    };
    start.get("line").is_some()
        && start.get("character").is_some()
        && end.get("line").is_some()
        && end.get("character").is_some()
}

/// Recursively collect every LSP `Range` nested anywhere in `obj`.
pub fn iter_ranges(obj: &Value) -> Vec<Value> {
    fn walk(obj: &Value, out: &mut Vec<Value>) {
        if looks_like_range(obj) {
            out.push(obj.clone());
            return;
        }
        match obj {
            Value::Object(o) => o.values().for_each(|v| walk(v, out)),
            Value::Array(a) => a.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(obj, &mut out);
    out
}

/// Length of `s` in UTF-16 code units — the unit LSP positions are measured in.
fn utf16_len(s: &str) -> i64 {
    s.chars()
        .map(|c| i64::try_from(c.len_utf16()).unwrap())
        .sum()
}

/// Per-line content length in UTF-16 units (trailing `\r` excluded).
fn doc_lines_utf16(text: &str) -> Vec<i64> {
    text.split('\n')
        .map(|line| utf16_len(line.strip_suffix('\r').unwrap_or(line)))
        .collect()
}

/// Return well-formedness violations for every `Range` in `result` (empty ==
/// ok). See the pytest docstring: non-negative coords, `start <= end`, real
/// lines, `start` column within its line (UTF-16). `strict_end_column` also
/// requires `end` within its line.
pub fn range_violations(
    result: &Value,
    text: &str,
    label: &str,
    strict_end_column: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    let line_u16 = doc_lines_utf16(text);
    let n_lines = i64::try_from(line_u16.len()).unwrap();
    let pre = if label.is_empty() {
        String::new()
    } else {
        format!("{label}: ")
    };
    for rng in iter_ranges(result) {
        let s = &rng["start"];
        let e = &rng["end"];
        let (sl, sc) = (
            s["line"].as_i64().unwrap_or(0),
            s["character"].as_i64().unwrap_or(0),
        );
        let (el, ec) = (
            e["line"].as_i64().unwrap_or(0),
            e["character"].as_i64().unwrap_or(0),
        );
        if sl.min(sc).min(el).min(ec) < 0 {
            out.push(format!("{pre}negative coordinate in range {rng}"));
            continue;
        }
        if (el, ec) < (sl, sc) {
            out.push(format!("{pre}end before start in range {rng}"));
        }
        if sl >= n_lines {
            out.push(format!(
                "{pre}start line {sl} past end of document ({n_lines} lines)"
            ));
            continue;
        }
        if el > n_lines {
            out.push(format!(
                "{pre}end line {el} past end of document ({n_lines} lines)"
            ));
        }
        if sc > line_u16[usize::try_from(sl).unwrap()] {
            out.push(format!(
                "{pre}start col {sc} past line {sl} length {} (UTF-16)",
                line_u16[usize::try_from(sl).unwrap()]
            ));
        }
        if strict_end_column && el < n_lines && ec > line_u16[usize::try_from(el).unwrap()] {
            out.push(format!(
                "{pre}end col {ec} past line {el} length {} (UTF-16)",
                line_u16[usize::try_from(el).unwrap()]
            ));
        }
    }
    out
}

/// A rename edit reduced to `((start_line, start_col), (end_line, end_col), new_text)`.
type EditSpan = ((i64, i64), (i64, i64), String);

/// Return overlap/duplicate violations in a `WorkspaceEdit` (empty == ok).
pub fn workspace_edit_violations(result: &Value, label: &str) -> Vec<String> {
    let mut out = Vec::new();
    let pre = if label.is_empty() {
        String::new()
    } else {
        format!("{label}: ")
    };
    for (uri, edits) in rename_edits(result) {
        let mut spans: Vec<EditSpan> = edits
            .iter()
            .map(|ed| {
                let rng = ed.get("range").cloned().unwrap_or(Value::Null);
                let s = rng.get("start").cloned().unwrap_or(Value::Null);
                let e = rng.get("end").cloned().unwrap_or(Value::Null);
                (
                    (
                        s.get("line").and_then(Value::as_i64).unwrap_or(0),
                        s.get("character").and_then(Value::as_i64).unwrap_or(0),
                    ),
                    (
                        e.get("line").and_then(Value::as_i64).unwrap_or(0),
                        e.get("character").and_then(Value::as_i64).unwrap_or(0),
                    ),
                    ed.get("newText")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                )
            })
            .collect();
        spans.sort();
        for i in 1..spans.len() {
            let (prev_s, prev_e, prev_t) = &spans[i - 1];
            let (cur_s, cur_e, cur_t) = &spans[i];
            if cur_s == prev_s && cur_e == prev_e && cur_t == prev_t {
                out.push(format!(
                    "{pre}{uri}: duplicate edit at {cur_s:?}->{cur_e:?}"
                ));
            } else if cur_s < prev_e {
                out.push(format!(
                    "{pre}{uri}: overlapping edits {prev_s:?}->{prev_e:?} and {cur_s:?}->{cur_e:?}"
                ));
            }
        }
    }
    out
}

/// Return semantic-token invariant violations (empty == valid). See the pytest
/// docstring for the full property list.
pub fn semantic_token_violations(
    result: &Value,
    text: &str,
    token_types: &[&str],
    token_modifiers: &[&str],
) -> Vec<String> {
    let mut out = Vec::new();
    let Some(data) = result.get("data").and_then(Value::as_array) else {
        return vec!["semanticTokens result has no `data` array".to_owned()];
    };
    if data.len() % 5 != 0 {
        return vec![format!(
            "token data length {} is not a multiple of 5",
            data.len()
        )];
    }
    let n_types = i64::try_from(token_types.len()).unwrap();
    let n_mods = i64::try_from(token_modifiers.len()).unwrap();
    let ints: Vec<i64> = data.iter().filter_map(Value::as_i64).collect();
    for (i, group) in ints.chunks(5).enumerate() {
        let (d_line, d_char, length, ttype, tmods) =
            (group[0], group[1], group[2], group[3], group[4]);
        if d_line < 0 || d_char < 0 {
            out.push(format!(
                "token {i}: negative delta (dline={d_line}, dchar={d_char})"
            ));
        }
        if length <= 0 {
            out.push(format!("token {i}: non-positive length {length}"));
        }
        if !(0..n_types).contains(&ttype) {
            out.push(format!(
                "token {i}: type index {ttype} outside legend (0..{})",
                n_types - 1
            ));
        }
        if tmods < 0 || (n_mods < 32 && (tmods >> n_mods) != 0) {
            out.push(format!(
                "token {i}: modifier bits {tmods:#b} outside legend ({n_mods} mods)"
            ));
        }
    }

    let line_u16 = doc_lines_utf16(text);
    let n_lines = i64::try_from(line_u16.len()).unwrap();
    let mut prev: Option<SemToken> = None;
    for (idx, tok) in decode_semantic_tokens(result).into_iter().enumerate() {
        if tok.line >= n_lines {
            out.push(format!(
                "token {idx}: line {} past end of document ({n_lines} lines)",
                tok.line
            ));
        } else if tok.char + tok.length > line_u16[usize::try_from(tok.line).unwrap()] {
            out.push(format!(
                "token {idx}: ends at utf16 col {} but line {} is {} units long (UTF-16 drift?)",
                tok.char + tok.length,
                tok.line,
                line_u16[usize::try_from(tok.line).unwrap()]
            ));
        }
        if let Some(p) = prev {
            if (tok.line, tok.char) < (p.line, p.char) {
                out.push(format!(
                    "token {idx}: starts before previous token (not ascending)"
                ));
            } else if tok.line == p.line && tok.char < p.char + p.length {
                out.push(format!(
                    "token {idx}: overlaps previous token on line {} (starts {}, previous ends {})",
                    tok.line,
                    tok.char,
                    p.char + p.length
                ));
            }
        }
        prev = Some(tok);
    }
    out
}

/// The `(range, newText)` of every workspace edit carried by code actions
/// whose `title` satisfies `pred`, in response order. Handles both
/// `WorkspaceEdit` encodings (`changes` and `documentChanges`), mirroring the
/// per-file new-text collectors in the code-action suites.
pub fn action_edits(actions: &Value, pred: impl Fn(&str) -> bool) -> Vec<(Value, String)> {
    fn push_edit(edit: &Value, out: &mut Vec<(Value, String)>) {
        if let (Some(rng), Some(text)) = (
            edit.get("range"),
            edit.get("newText").and_then(Value::as_str),
        ) {
            out.push((rng.clone(), text.to_owned()));
        }
    }
    let mut out = Vec::new();
    let empty = Vec::new();
    for action in actions.as_array().unwrap_or(&empty) {
        let title = action.get("title").and_then(Value::as_str).unwrap_or("");
        if !pred(title) {
            continue;
        }
        let edit = action.get("edit").cloned().unwrap_or(Value::Null);
        if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
            for edits in changes.values() {
                for e in edits.as_array().unwrap_or(&empty) {
                    push_edit(e, &mut out);
                }
            }
        }
        if let Some(doc_changes) = edit.get("documentChanges").and_then(Value::as_array) {
            for change in doc_changes {
                for e in change
                    .get("edits")
                    .and_then(Value::as_array)
                    .unwrap_or(&empty)
                {
                    push_edit(e, &mut out);
                }
            }
        }
    }
    out
}

/// Assert that at least one code action titled exactly `title` carries an
/// edit that, applied to `source`, produces `expected` — pinning the fix's
/// range and `newText` jointly by their end-to-end effect. Tolerant of
/// same-titled actions from other providers: any matching edit satisfies.
pub fn assert_fix_applies(actions: &Value, source: &str, title: &str, expected: &str) {
    let edits = action_edits(actions, |t| t == title);
    assert!(
        edits
            .iter()
            .any(|(rng, new_text)| apply_edit(source, rng, new_text) == expected),
        "expected a {title:?} fix producing {expected:?}; offered actions: {actions:?}"
    );
}

/// Apply a single `(range, newText)` text edit to `source`, returning the
/// edited document — the strongest form of "the fix produces the right
/// change". Columns are treated as byte offsets, which matches LSP's UTF-16
/// units for the ASCII-only fixtures the code-action suites use.
pub fn apply_edit(source: &str, rng: &Value, new_text: &str) -> String {
    fn offset(source: &str, pos: &Value) -> usize {
        let line = usize::try_from(pos["line"].as_u64().unwrap()).unwrap();
        let character = usize::try_from(pos["character"].as_u64().unwrap()).unwrap();
        let line_start: usize = source.split_inclusive('\n').take(line).map(str::len).sum();
        line_start + character
    }
    let start = offset(source, &rng["start"]);
    let end = offset(source, &rng["end"]);
    format!("{}{}{}", &source[..start], new_text, &source[end..])
}

/// A result normalised to a list of items (bare value → single-element list;
/// null → empty).
fn as_items(result: &Value) -> Vec<Value> {
    match result {
        Value::Null => Vec::new(),
        Value::Array(a) => a.clone(),
        other => vec![other.clone()],
    }
}
