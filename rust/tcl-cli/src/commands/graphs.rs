//! Analysis/graph verbs: `symbols` (and, as the analyser engine converges,
//! `callgraph` / `symbolgraph` / `dataflow` / `diagram`).
//!
//! Ports the handlers in `tooling/tcl/verbs/graphs.py`. These verbs combine
//! every resolved input into one source (`combine_sources`), analyse it, and
//! emit a JSON-serialisable graph/symbol shape (or a plain-text summary).
//!
//! Ordering note: the Python source iterates `Scope.procs` / `Scope.variables`
//! (insertion-ordered dicts), so symbols come out in source-definition order.
//! The Rust analyser stores them in `HashMap`s, so we sort by the defining
//! token's source offset to recover that deterministic ordering.

use serde::Serialize;
use tcl_cli_support::{
    OutputTarget, combine_sources, ensure_ascii, read_input_documents, write_text_output,
};
use tcl_compiler::analyser::{Analyser, ProcDef, Scope, ScopeKind, VarDef};
use tcl_lexer::LineIndex;

use crate::cli::InputArgs;

/// One symbol entry in the `symbols` payload.
///
/// Field order mirrors the Python dict (`kind`, `name`, `line`, `depth`,
/// then the function-only `params`). `params` is emitted only for functions
/// (Python omits the key entirely for other kinds), and `line` may be `null`
/// for a proc with no name token.
#[derive(Serialize)]
struct SymbolEntry {
    kind: &'static str,
    name: String,
    line: Option<u32>,
    depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<String>>,
}

/// `symbols --json` payload.
#[derive(Serialize)]
struct SymbolsPayload {
    count: usize,
    dialect: String,
    inputs: Vec<String>,
    symbols: Vec<SymbolEntry>,
}

/// 1-based source line of a span's start offset.
fn line_of(line_index: &LineIndex, offset: u32) -> u32 {
    line_index.position_at(offset).line + 1
}

/// Detect `when EVENT` iRules entries via the Python regex
/// `\bwhen\s+([A-Z_][A-Z0-9_]*)`, deduplicated, in first-seen order.
///
/// Mirrors `_detect_event_entries`: runs unconditionally (every dialect),
/// each entry at depth 0 with its 1-based line.
fn detect_event_entries(source: &str, line_index: &LineIndex) -> Vec<SymbolEntry> {
    fn is_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }
    fn is_upper_start(b: u8) -> bool {
        b.is_ascii_uppercase() || b == b'_'
    }
    fn is_upper_rest(b: u8) -> bool {
        b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'
    }

    let bytes = source.as_bytes();
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        // `\bwhen`: the four chars "when" with a word boundary before.
        if &bytes[i..i + 4] == b"when" && (i == 0 || !is_word(bytes[i - 1])) {
            let mut j = i + 4;
            // `\s+`
            let ws_start = j;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j > ws_start && j < bytes.len() && is_upper_start(bytes[j]) {
                let name_start = j;
                j += 1;
                while j < bytes.len() && is_upper_rest(bytes[j]) {
                    j += 1;
                }
                let name = &source[name_start..j];
                if seen.insert(name.to_string()) {
                    entries.push(SymbolEntry {
                        kind: "event",
                        name: name.to_string(),
                        line: Some(line_of(line_index, u32::try_from(i).unwrap_or(u32::MAX))),
                        depth: 0,
                        params: None,
                    });
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    entries
}

/// Recursively collect proc/variable/namespace symbol entries from a scope.
///
/// Mirrors `_collect_scope_symbol_entries`: procs first (source order), then
/// global/namespace-scope variables, then children — namespace children get
/// an entry and recurse, proc children only recurse (to surface nested procs).
fn collect_scope_entries(
    scope: &Scope,
    depth: usize,
    line_index: &LineIndex,
    out: &mut Vec<SymbolEntry>,
) {
    let mut procs: Vec<&ProcDef> = scope.procs.values().collect();
    procs.sort_by_key(|p| p.name_span.start());
    for proc in procs {
        let params = proc.params.iter().map(|p| p.name.clone()).collect();
        out.push(SymbolEntry {
            kind: "function",
            name: proc.name.clone(),
            line: Some(line_of(line_index, proc.name_span.start())),
            depth,
            params: Some(params),
        });
    }

    if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace) {
        let mut vars: Vec<&VarDef> = scope.variables.values().collect();
        vars.sort_by_key(|v| v.definition_span.start());
        for var in vars {
            out.push(SymbolEntry {
                kind: "variable",
                name: var.name.clone(),
                line: Some(line_of(line_index, var.definition_span.start())),
                depth,
                params: None,
            });
        }
    }

    for child in &scope.children {
        match child.kind {
            ScopeKind::Namespace => {
                if let Some(body) = child.body_span {
                    out.push(SymbolEntry {
                        kind: "namespace",
                        name: child.name.clone(),
                        line: Some(line_of(line_index, body.start())),
                        depth,
                        params: None,
                    });
                    collect_scope_entries(child, depth + 1, line_index, out);
                }
            }
            ScopeKind::Proc => {
                collect_scope_entries(child, depth + 1, line_index, out);
            }
            _ => {}
        }
    }
}

/// `tcl symbols` — list every declared symbol (procs, namespaces, variables,
/// iRules `when` events) across the combined input.
pub fn run_symbols(input: &InputArgs, json: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);
    let result = Analyser::new().analyse(&source, &input.dialect);
    let line_index = LineIndex::new(&source);

    let mut entries = detect_event_entries(&source, &line_index);
    collect_scope_entries(&result.global_scope, 0, &line_index, &mut entries);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json {
        let payload = SymbolsPayload {
            count: entries.len(),
            dialect: input.dialect.clone(),
            inputs: documents.iter().map(|d| d.label.clone()).collect(),
            symbols: entries,
        };
        let text = ensure_ascii(&serde_json::to_string_pretty(&payload)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    if entries.is_empty() {
        write_text_output(&target, "no symbols")?;
        return Ok(0);
    }

    let mut lines = vec![format!("symbols: {}", entries.len())];
    for entry in &entries {
        let indent = "  ".repeat(entry.depth);
        let line_suffix = entry.line.map_or_else(String::new, |l| format!(" (line {l})"));
        if entry.kind == "function" {
            let params = entry
                .params
                .as_ref()
                .map(|p| p.join(", "))
                .unwrap_or_default();
            lines.push(format!("{indent}function {}({params}){line_suffix}", entry.name));
        } else {
            lines.push(format!("{indent}{} {}{line_suffix}", entry.kind, entry.name));
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
