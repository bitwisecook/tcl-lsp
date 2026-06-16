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
use serde_json::{Value, json};
use tcl_cli_support::{
    OutputTarget, combine_sources, ensure_ascii, read_input_documents, write_text_output,
};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ProcDef, Scope, ScopeKind, VarDef};
use tcl_lexer::{LineIndex, Span};

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
        let line_suffix = entry
            .line
            .map_or_else(String::new, |l| format!(" (line {l})"));
        if entry.kind == "function" {
            let params = entry
                .params
                .as_ref()
                .map(|p| p.join(", "))
                .unwrap_or_default();
            lines.push(format!(
                "{indent}function {}({params}){line_suffix}",
                entry.name
            ));
        } else {
            lines.push(format!(
                "{indent}{} {}{line_suffix}",
                entry.kind, entry.name
            ));
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}

// ---------------------------------------------------------------------------
// symbolgraph
// ---------------------------------------------------------------------------

/// 0-based source line of a span-start offset (Python `range.start.line`).
fn line0(line_index: &LineIndex, offset: u32) -> u32 {
    line_index.position_at(offset).line
}

/// `{"line": L, "character": C}` for a span-start offset, 0-based and counted
/// in UTF-16 code units (mirrors `_pos_dict` over the analyser `Range`).
fn pos_value(line_index: &LineIndex, source: &str, offset: u32) -> Value {
    let pos = line_index.position_at_utf16(offset, source);
    json!({ "line": pos.line, "character": pos.character })
}

/// Call sites of a proc, deduplicated by span (port of `find_proc_call_sites`).
fn find_proc_call_sites(name: &str, qualified_name: &str, analysis: &AnalysisResult) -> Vec<Span> {
    let no_prefix = qualified_name.strip_prefix("::").unwrap_or(qualified_name);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for inv in &analysis.command_invocations {
        let matches = match &inv.resolved_qualified_name {
            Some(resolved) => resolved == qualified_name,
            None => inv.name == name || inv.name == qualified_name || inv.name == no_prefix,
        };
        if matches && seen.insert((inv.range.start(), inv.range.end())) {
            out.push(inv.range);
        }
    }
    out
}

/// Serialise one scope (global / namespace) to the wire dict.
///
/// Port of `_scope_to_dict`: procs first (source order), then this scope's
/// variables, then children — namespace children recurse with the full shape,
/// proc children contribute a `{kind, name, variables, children:[]}` node only
/// when they carry recorded variables. `HashMap` iteration is made
/// deterministic by sorting on the defining token's offset.
fn scope_to_value(
    scope: &Scope,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    source: &str,
) -> Value {
    let mut procs: Vec<&ProcDef> = scope.procs.values().collect();
    procs.sort_by_key(|p| p.name_span.start());
    let proc_values: Vec<Value> = procs
        .iter()
        .map(|proc| {
            let ref_count = analysis
                .command_invocations
                .iter()
                .filter(|inv| {
                    inv.resolved_qualified_name.as_deref() == Some(proc.qualified_name.as_str())
                        || inv.name == proc.name
                })
                .count();
            json!({
                "name": proc.name,
                "qualified_name": proc.qualified_name,
                "params": proc.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
                "line": line0(line_index, proc.name_span.start()),
                "ref_count": ref_count,
            })
        })
        .collect();

    // Namespace/global scopes carry no parameters, so an empty param order is
    // correct here.
    let variable_values = collect_var_values(scope, &[], line_index, source);

    let mut children = Vec::new();
    for child in &scope.children {
        match child.kind {
            ScopeKind::Namespace => {
                children.push(scope_to_value(child, analysis, line_index, source));
            }
            ScopeKind::Proc => {
                // The proc's parameters all share the proc-name token's span,
                // so the scope alone can't order them; recover declaration
                // order from the owning `ProcDef` (matched by body span).
                let params: Vec<String> = scope
                    .procs
                    .values()
                    .find(|p| Some(p.body_span) == child.body_span)
                    .map(|p| p.params.iter().map(|x| x.name.clone()).collect())
                    .unwrap_or_default();
                let proc_vars = collect_var_values(child, &params, line_index, source);
                if !proc_vars.is_empty() {
                    children.push(json!({
                        "kind": "proc",
                        "name": child.name,
                        "variables": proc_vars,
                        "children": [],
                    }));
                }
            }
            _ => {}
        }
    }

    let mut map = serde_json::Map::new();
    map.insert("kind".to_string(), json!(scope.kind.as_str()));
    map.insert("name".to_string(), json!(scope.name));
    if !proc_values.is_empty() {
        map.insert("procs".to_string(), json!(proc_values));
    }
    if !variable_values.is_empty() {
        map.insert("variables".to_string(), json!(variable_values));
    }
    if !children.is_empty() {
        map.insert("children".to_string(), json!(children));
    }
    Value::Object(map)
}

/// `[{name, line, references:[…]}]` for a scope's variables in Python's
/// insertion order: parameters first (declaration order, since they share the
/// proc-name span and can't be span-ordered), then body locals by definition
/// offset. `params` is empty for namespace/global scopes.
fn collect_var_values(
    scope: &Scope,
    params: &[String],
    line_index: &LineIndex,
    source: &str,
) -> Vec<Value> {
    let param_pos: std::collections::HashMap<&str, usize> = params
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut vars: Vec<&VarDef> = scope.variables.values().collect();
    vars.sort_by(|a, b| {
        match (
            param_pos.get(a.name.as_str()),
            param_pos.get(b.name.as_str()),
        ) {
            (Some(ia), Some(ib)) => ia.cmp(ib),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.definition_span.start().cmp(&b.definition_span.start()),
        }
    });
    vars.iter()
        .map(|var| {
            let refs: Vec<Value> = var
                .references
                .iter()
                .map(|r| pos_value(line_index, source, r.start()))
                .collect();
            json!({
                "name": var.name,
                "line": line0(line_index, var.definition_span.start()),
                "references": refs,
            })
        })
        .collect()
}

/// Count variables with a definition site across the whole scope tree.
fn count_variables(scope: &Scope) -> usize {
    scope.variables.len() + scope.children.iter().map(count_variables).sum::<usize>()
}

/// Count namespace scopes across the whole scope tree.
fn count_namespaces(scope: &Scope) -> usize {
    scope
        .children
        .iter()
        .map(|c| usize::from(c.kind == ScopeKind::Namespace) + count_namespaces(c))
        .sum()
}

/// Render the text form of a serialised scope (port of
/// `_append_symbolgraph_scope`).
fn append_symbolgraph_scope(lines: &mut Vec<String>, scope: &Value, depth: usize) {
    let indent = "  ".repeat(depth);
    let kind = scope.get("kind").and_then(Value::as_str).unwrap_or("?");
    let name = scope.get("name").and_then(Value::as_str).unwrap_or("?");
    lines.push(format!("{indent}{kind} {name}"));

    if let Some(procs) = scope.get("procs").and_then(Value::as_array) {
        for proc in procs {
            let params = proc
                .get("params")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let line_suffix = proc
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(String::new, |l| format!(" (line {})", l + 1));
            let refs = proc.get("ref_count").and_then(Value::as_i64).unwrap_or(0);
            let pname = proc.get("name").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!(
                "{indent}  proc {pname}({params}){line_suffix} [{refs} refs]"
            ));
        }
    }

    if let Some(variables) = scope.get("variables").and_then(Value::as_array) {
        for var in variables {
            let line_suffix = var
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(String::new, |l| format!(" (line {})", l + 1));
            let refs = var
                .get("references")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let vname = var.get("name").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!("{indent}  var {vname}{line_suffix} [{refs} refs]"));
        }
    }

    if let Some(children) = scope.get("children").and_then(Value::as_array) {
        for child in children {
            append_symbolgraph_scope(lines, child, depth + 1);
        }
    }
}

/// `tcl symbolgraph` — scope hierarchy with proc/variable references.
pub fn run_symbolgraph(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);
    let result = Analyser::new().analyse(&source, &input.dialect);
    let line_index = LineIndex::new(&source);

    let scopes = vec![scope_to_value(
        &result.global_scope,
        &result,
        &line_index,
        &source,
    )];

    // proc_references: every proc's deduped call sites, source-ordered keys.
    let mut all_procs: Vec<&ProcDef> = result.all_procs.values().collect();
    all_procs.sort_by_key(|p| p.name_span.start());
    let mut proc_references = serde_json::Map::new();
    for proc in all_procs {
        let sites = find_proc_call_sites(&proc.name, &proc.qualified_name, &result);
        if !sites.is_empty() {
            let positions: Vec<Value> = sites
                .iter()
                .map(|s| pos_value(&line_index, &source, s.start()))
                .collect();
            proc_references.insert(proc.qualified_name.clone(), json!(positions));
        }
    }

    let package_requires: Vec<Value> = result
        .package_requires
        .iter()
        .map(|pr| {
            let mut map = serde_json::Map::new();
            map.insert("name".to_string(), json!(pr.name));
            map.insert(
                "line".to_string(),
                json!(line0(&line_index, pr.range.start())),
            );
            if let Some(version) = &pr.version {
                map.insert("version".to_string(), json!(version));
            }
            Value::Object(map)
        })
        .collect();

    let total_procs = result.all_procs.len();
    let total_variables = count_variables(&result.global_scope);
    let total_namespaces = count_namespaces(&result.global_scope);

    let data = json!({
        "scopes": scopes,
        "proc_references": Value::Object(proc_references),
        "package_requires": package_requires,
        "summary": {
            "total_procs": total_procs,
            "total_variables": total_variables,
            "total_namespaces": total_namespaces,
        },
    });

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let mut lines = vec![format!(
        "symbol graph: procs={total_procs} variables={total_variables} namespaces={total_namespaces}"
    )];
    if let Some(scope_list) = data.get("scopes").and_then(Value::as_array)
        && !scope_list.is_empty()
    {
        lines.push("scopes:".to_string());
        for scope in scope_list {
            append_symbolgraph_scope(&mut lines, scope, 1);
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
