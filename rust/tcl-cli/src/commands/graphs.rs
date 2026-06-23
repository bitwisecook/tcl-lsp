//! Analysis/graph verbs: `symbols`, `callgraph`, `symbolgraph`, `dataflow`,
//! and `diagram`.
//!
//! These verbs combine every resolved input into one source
//! (`combine_sources`), analyse it, and
//! emit a JSON-serialisable graph/symbol shape (or a plain-text summary).
//!
//! Ordering note: the Python source iterates `Scope.procs` / `Scope.variables`
//! (insertion-ordered dicts), so symbols come out in source-definition order.
//! The Rust analyser stores them in `HashMap`s, so we sort by the defining
//! token's source offset to recover that deterministic ordering.

use serde::Serialize;
use serde_json::{Map, Value, json};
use tcl_cli_support::{
    OutputTarget, combine_sources, ensure_ascii, read_input_documents, registry_for_dialect,
    write_text_output,
};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ProcDef, Scope, ScopeKind, VarDef};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::ir::Module as IrModule;
use tcl_compiler::path_concat::find_path_concat_warnings;
use tcl_compiler::side_effects::EffectRegion;
use tcl_compiler::taint::{
    find_destructive_file_warnings, find_setter_constraint_warnings, find_taint_warnings,
    is_irules_dialect,
};
use tcl_compiler::uri_split::find_uri_split_suggestions;
use tcl_lexer::{LineIndex, Span};

use crate::cli::InputArgs;

/// One symbol entry in the `symbols` payload.
///
/// Fields are emitted in order (`kind`, `name`, `line`, `depth`, then the
/// function-only `params`). `params` is emitted only for functions (the key
/// is omitted entirely for other kinds), and `line` may be `null` for a proc
/// with no name token.
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

// symbolgraph

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

/// Call sites of a proc, deduplicated by span.
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
/// Procs first (source order), then this scope's
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

/// Render the text form of a serialised scope.
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

// callgraph

/// The synthetic caller node for calls made outside any proc body.
const TOP_LEVEL: &str = "<top-level>";

/// Human-readable string for an [`EffectRegion`].
/// Empty → `"NONE"`; otherwise the contained
/// single-bit member names joined with `|` in definition order (which, for
/// a single member, yields just that name — matching Python's `IntFlag`
/// `.name`).
fn effect_region_str(er: EffectRegion) -> String {
    const MEMBERS: &[(EffectRegion, &str)] = &[
        (EffectRegion::HTTP_STATE, "HTTP_STATE"),
        (EffectRegion::RESPONSE_LIFECYCLE, "RESPONSE_LIFECYCLE"),
        (EffectRegion::GLOBAL_STATE, "GLOBAL_STATE"),
        (EffectRegion::UNKNOWN_STATE, "UNKNOWN_STATE"),
    ];
    if er.is_empty() {
        return "NONE".to_owned();
    }
    MEMBERS
        .iter()
        .filter(|(flag, _)| er.contains(*flag))
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join("|")
}

/// 0-based start/end line pair for a proc's definition span.
fn proc_line_span(ir_module: &IrModule, qname: &str, line_index: &LineIndex) -> Option<(u32, u32)> {
    ir_module.procedures.get(qname).map(|p| {
        (
            line0(line_index, p.span.start()),
            line0(line_index, p.span.end()),
        )
    })
}

/// Whether a span lies wholly within some proc's definition.
fn is_inside_proc(range: Span, ir_module: &IrModule, line_index: &LineIndex) -> bool {
    let start = line0(line_index, range.start());
    let end = line0(line_index, range.end());
    ir_module.procedures.values().any(|p| {
        let ps = line0(line_index, p.span.start());
        let pe = line0(line_index, p.span.end());
        start >= ps && end <= pe
    })
}

/// Call-site positions of `callee_qname` within `caller_qname`'s body.
#[allow(clippy::similar_names)] // caller_qname / callee_qname mirror the domain
fn find_call_sites_in_scope(
    analysis: &AnalysisResult,
    callee_qname: &str,
    caller_qname: &str,
    ir_module: &IrModule,
    line_index: &LineIndex,
    source: &str,
) -> Vec<Value> {
    let Some((proc_start, proc_end)) = proc_line_span(ir_module, caller_qname, line_index) else {
        return Vec::new();
    };
    let short = callee_qname.trim_start_matches(':');
    let mut callee_forms: std::collections::HashSet<&str> =
        [callee_qname, short].into_iter().collect();
    if let Some(stripped) = callee_qname.strip_prefix("::") {
        callee_forms.insert(stripped);
    }

    let mut sites = Vec::new();
    for inv in &analysis.command_invocations {
        if line0(line_index, inv.range.start()) < proc_start {
            continue;
        }
        if line0(line_index, inv.range.end()) > proc_end {
            continue;
        }
        match &inv.resolved_qualified_name {
            Some(resolved) => {
                if resolved != callee_qname {
                    continue;
                }
            }
            None => {
                if !callee_forms.contains(inv.name.as_str()) {
                    continue;
                }
            }
        }
        sites.push(pos_value(line_index, source, inv.range.start()));
    }
    sites
}

/// Resolve a top-level invocation to a known proc qname. Proc names are
/// iterated in sorted order
/// for a deterministic pick on the (rare) ambiguous short-name fallback.
fn resolve_invocation_target(
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    proc_names_sorted: &[String],
) -> Option<String> {
    if let Some(resolved) = &inv.resolved_qualified_name
        && proc_names_sorted.iter().any(|n| n == resolved)
    {
        return Some(resolved.clone());
    }
    for qname in proc_names_sorted {
        let short = qname.strip_prefix("::").unwrap_or(qname);
        if inv.name == *qname || inv.name == short || inv.name == qname.trim_start_matches(':') {
            return Some(qname.clone());
        }
    }
    None
}

/// The `nodes` list — one entry per proc (sorted by qname), carrying
/// params, 0-based definition line, purity, and the effect-region string.
fn build_nodes(
    proc_names: &[String],
    interproc: &tcl_compiler::interprocedural::InterproceduralAnalysis,
    ir_module: &IrModule,
    line_index: &LineIndex,
) -> Vec<Value> {
    proc_names
        .iter()
        .map(|qname| {
            let summary = &interproc.procedures[qname];
            let line = ir_module
                .procedures
                .get(qname)
                .map(|p| line0(line_index, p.span.start()));
            let effects = effect_region_str(summary.effect_reads | summary.effect_writes);
            json!({
                "name": qname,
                "params": summary.params,
                "line": line,
                "pure": summary.pure,
                "effects": effects,
            })
        })
        .collect()
}

/// Build the full call-graph payload.
fn build_call_graph(source: &str, dialect: &str) -> Value {
    let registry = registry_for_dialect(dialect);
    // Build the full compilation unit (matching Python's
    // `ensure_compilation_unit`) so the interprocedural pass sees the same
    // lowered IR — raw `lower_to_ir` alone does not surface nested `[cmd …]`
    // call sites to the call scanner.
    let cu = CompilationUnit::build_for(source, registry, false)
        .with_interprocedural(registry, Some(dialect));
    let ir_module = &cu.ir_module;
    let interproc = cu
        .interproc
        .as_ref()
        .expect("with_interprocedural populates the summary");
    let analysis = Analyser::new().analyse(source, dialect);
    let line_index = LineIndex::new(source);

    // Proc qnames in sorted order (Python iterates the summary dict, which is
    // keyed and surfaced in sorted qualified-name order).
    let mut proc_names: Vec<String> = interproc.procedures.keys().cloned().collect();
    proc_names.sort();
    let proc_set: std::collections::HashSet<&str> = proc_names.iter().map(String::as_str).collect();

    let nodes = build_nodes(&proc_names, interproc, ir_module, &line_index);

    // Edges + bookkeeping.
    let mut edges: Vec<Value> = Vec::new();
    let mut called_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut has_outgoing: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for qname in &proc_names {
        let summary = &interproc.procedures[qname];
        for callee in &summary.direct_calls {
            if !proc_set.contains(callee.as_str()) {
                continue;
            }
            has_outgoing.insert(qname.as_str());
            called_set.insert(callee.clone());
            let sites =
                find_call_sites_in_scope(&analysis, callee, qname, ir_module, &line_index, source);
            edges.push(json!({
                "caller": qname,
                "callee": callee,
                "call_sites": sites,
            }));
        }
    }

    // Top-level calls (outside any proc body), in first-seen order.
    let mut top_level: Map<String, Value> = Map::new();
    for inv in &analysis.command_invocations {
        if is_inside_proc(inv.range, ir_module, &line_index) {
            continue;
        }
        if let Some(target) = resolve_invocation_target(inv, &proc_names) {
            let pos = pos_value(&line_index, source, inv.range.start());
            top_level
                .entry(target)
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .expect("call-site list is an array")
                .push(pos);
        }
    }
    let has_top_level_calls = !top_level.is_empty();
    for (callee, sites) in &top_level {
        called_set.insert(callee.clone());
        edges.push(json!({
            "caller": TOP_LEVEL,
            "callee": callee,
            "call_sites": sites,
        }));
    }

    // Roots = uncalled procs, sorted; `<top-level>` prepended when any
    // invocation sits outside a proc body.
    let mut roots: Vec<String> = proc_names
        .iter()
        .filter(|qn| !called_set.contains(*qn))
        .cloned()
        .collect();
    roots.sort();
    let any_top_level_inv = analysis
        .command_invocations
        .iter()
        .any(|inv| !is_inside_proc(inv.range, ir_module, &line_index));
    if has_top_level_calls || any_top_level_inv {
        roots.insert(0, TOP_LEVEL.to_owned());
    }

    // Leaves = procs with no outgoing proc calls, sorted.
    let mut leaf_procs: Vec<String> = proc_names
        .iter()
        .filter(|qn| !has_outgoing.contains(qn.as_str()))
        .cloned()
        .collect();
    leaf_procs.sort();

    json!({
        "nodes": nodes,
        "edges": edges,
        "roots": roots,
        "leaf_procs": leaf_procs,
    })
}

/// `tcl callgraph` — caller→callee graph across every proc in the input.
#[allow(clippy::similar_names)] // caller / callee mirror the domain
pub fn run_callgraph(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);
    let data = build_call_graph(&source, &input.dialect);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let empty: Vec<Value> = Vec::new();
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let edges = data
        .get("edges")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let roots = data
        .get("roots")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let leaves = data
        .get("leaf_procs")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    let mut lines = vec![format!(
        "call graph: procs={} edges={}",
        nodes.len(),
        edges.len()
    )];
    if !nodes.is_empty() {
        lines.push("procs:".to_owned());
        for node in nodes {
            let name = node.get("name").and_then(Value::as_str).unwrap_or("?");
            let params = node
                .get("params")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let line_suffix = node
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(String::new, |l| format!(" (line {})", l + 1));
            let pure_suffix = if node.get("pure").and_then(Value::as_bool) == Some(true) {
                " [pure]"
            } else {
                ""
            };
            lines.push(format!("  {name}({params}){line_suffix}{pure_suffix}"));
        }
    }
    if !edges.is_empty() {
        lines.push("edges:".to_owned());
        for edge in edges {
            let caller = edge.get("caller").and_then(Value::as_str).unwrap_or("?");
            let callee = edge.get("callee").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!("  {caller} -> {callee}"));
        }
    }
    if !roots.is_empty() {
        let names: Vec<&str> = roots.iter().filter_map(Value::as_str).collect();
        lines.push(format!("roots: {}", names.join(", ")));
    }
    if !leaves.is_empty() {
        let names: Vec<&str> = leaves.iter().filter_map(Value::as_str).collect();
        lines.push(format!("leaves: {}", names.join(", ")));
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}

// dataflow

/// Aggregate every taint warning kind for one function unit into the
/// dataflow JSON shape. Runs five families in a
/// fixed order: sink injection (`_find_taint_sinks`), setter-constraint,
/// uri-split, path-concat, then destructive-file — the same order
/// and dialect gating as `compiler_checks::run_all_checks`. Every kind
/// yields a `variable` + `sink_command` (Python's path-concat warning is a
/// `TaintWarning` with `sink_command="set"`; the Rust `PathConcatWarning`
/// carries no command field, so we supply the same constant).
fn collect_taint_warnings(
    fu: &FunctionUnit,
    taints: &std::collections::HashMap<
        tcl_compiler::ssa::ValueKey,
        tcl_compiler::taint::TaintLattice,
    >,
    registry: &tcl_registry::CommandRegistry,
    dialect: &str,
    line_index: &LineIndex,
    out: &mut Vec<Value>,
) {
    let mut push = |code: &str, span: Span, message: &str, variable: &str, sink: &str| {
        out.push(json!({
            "code": code,
            "line": line0(line_index, span.start()),
            "message": message,
            "variable": variable,
            "sink_command": sink,
        }));
    };

    // 1. Sink injection (T100 / T101 / T102 families).
    for w in find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        taints,
        &fu.sccp.executable_blocks,
        registry,
        Some(dialect),
    ) {
        push(&w.code, w.span, &w.message, &w.variable, &w.sink_command);
    }

    // 2 + 3. Setter-constraint + uri-split are iRules-only. The helpers
    // gate internally too; skipping the walk under a non-iRules dialect
    // matches `compiler_checks` and avoids needless work.
    if is_irules_dialect(Some(dialect)) {
        for w in find_setter_constraint_warnings(
            registry,
            &fu.cfg,
            &fu.ssa,
            taints,
            &fu.sccp.executable_blocks,
            Some(dialect),
        ) {
            push(&w.code, w.span, &w.message, &w.variable, &w.sink_command);
        }
        for w in find_uri_split_suggestions(
            &fu.cfg,
            &fu.ssa,
            Some(&fu.sccp.values),
            &fu.sccp.executable_blocks,
            registry,
            Some(dialect),
        ) {
            push(&w.code, w.span, &w.message, &w.variable, &w.sink_command);
        }
    }

    // 4. Path-concat (`W201`). Python reports `sink_command="set"`.
    for w in find_path_concat_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.rendered_props,
        taints,
        &fu.sccp.executable_blocks,
    ) {
        push(&w.code, w.span, &w.message, &w.variable, "set");
    }

    // 5. Destructive-file (`W313`).
    for w in find_destructive_file_warnings(
        &fu.cfg,
        &fu.ssa,
        taints,
        &fu.sccp.executable_blocks,
        registry,
    ) {
        push(&w.code, w.span, &w.message, &w.variable, &w.sink_command);
    }
}

/// Tainted variable names in `fu` (deduplicated, sorted for a
/// deterministic order). Python iterates the per-unit taint lattice map
/// (`analysis.taints`) in SSA insertion order; the Rust taint map is a
/// `HashMap`, so the order is recovered by sorting.
///
/// Version-0 entries are skipped: a `(name, 0)` slot is the enclosing-
/// scope / pre-existing value, and the only way it becomes tainted is the
/// conservative cross-procedure global-write seeding in `propagate_taints`
/// (e.g. `::store` in `proc save {v} { set ::store $v }`). Python's
/// per-unit `analysis.taints` does no such seeding, so it never surfaces a
/// version-0-tainted variable — filtering them here matches Python's
/// `tainted_variables` output without disturbing the seeding the sink
/// warnings rely on.
fn tainted_var_names(fu: &FunctionUnit) -> Vec<&str> {
    // Definition-site offset of each `(name, version)` SSA value, so
    // tainted variables order by where they are defined — recovering
    // Python's SSA/source iteration order over `analysis.taints` rather
    // than an alphabetical sort. Each SSA version is defined once, so
    // `or_insert` records that single site.
    let mut def_offset: std::collections::HashMap<(&str, u32), u32> =
        std::collections::HashMap::new();
    for block in fu.ssa.blocks.values() {
        for st in &block.statements {
            let off = st.statement.span().start();
            for (name, &ver) in &st.defs {
                def_offset.entry((name.as_str(), ver)).or_insert(off);
            }
        }
    }

    let mut entries: Vec<(&str, u32)> = fu
        .taints
        .iter()
        .filter(|((_, version), lattice)| *version != 0 && lattice.is_tainted())
        .map(|((name, version), _)| (name.as_str(), *version))
        .collect();
    // (def offset, version, name) keeps the order deterministic and
    // source-ordered; values defined outside this unit (no def site) sort
    // last.
    entries.sort_by_key(|(name, ver)| {
        (
            def_offset.get(&(*name, *ver)).copied().unwrap_or(u32::MAX),
            *ver,
            *name,
        )
    });
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    entries
        .into_iter()
        .filter_map(|(name, _)| seen.insert(name).then_some(name))
        .collect()
}

/// Build the dataflow / taint graph payload.
fn build_dataflow_graph(source: &str, dialect: &str) -> Value {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(source, registry, false)
        .with_interprocedural(registry, Some(dialect));
    let line_index = LineIndex::new(source);

    let mut proc_names: Vec<&String> = cu.procedures.keys().collect();
    proc_names.sort();

    // Interprocedural taint solve: colour-aware return summaries + parameter
    // entry taints. Its `top_taints` / `proc_taints` feed the warning
    // families (cross-proc entry-taint), mirroring Python's
    // `find_taint_warnings`. The `tainted_variables` listing below keeps the
    // per-unit `fu.taints` lattice, matching Python's `analysis.taints`.
    let solved =
        tcl_compiler::taint_interproc::solve_interprocedural_taints(&cu, registry, Some(dialect));

    // Taint warnings: top level first, then each procedure.
    let mut taint_warnings: Vec<Value> = Vec::new();
    collect_taint_warnings(
        &cu.top_level,
        solved.taints_for(&cu.top_level.name, &cu.top_level.taints),
        registry,
        dialect,
        &line_index,
        &mut taint_warnings,
    );
    for qname in &proc_names {
        let fu = &cu.procedures[*qname];
        collect_taint_warnings(
            fu,
            solved.taints_for(&fu.name, &fu.taints),
            registry,
            dialect,
            &line_index,
            &mut taint_warnings,
        );
    }

    // Tainted variables per scope: top level first, then each procedure.
    let mut tainted_variables: Vec<Value> = Vec::new();
    for name in tainted_var_names(&cu.top_level) {
        tainted_variables.push(json!({ "scope": "<top-level>", "variable": name }));
    }
    for qname in &proc_names {
        for name in tainted_var_names(&cu.procedures[*qname]) {
            tainted_variables.push(json!({ "scope": qname, "variable": name }));
        }
    }

    // Per-proc side-effect classification (interproc summaries, by qname).
    let interproc = cu
        .interproc
        .as_ref()
        .expect("with_interprocedural populates the summary");
    let mut effect_names: Vec<&String> = interproc.procedures.keys().collect();
    effect_names.sort();
    let mut proc_effects: Vec<Value> = Vec::new();
    let mut pure_count = 0i64;
    let mut impure_count = 0i64;
    for qname in &effect_names {
        let summary = &interproc.procedures[*qname];
        if summary.pure {
            pure_count += 1;
        } else {
            impure_count += 1;
        }
        proc_effects.push(json!({
            "name": qname,
            "pure": summary.pure,
            "reads": effect_region_str(summary.effect_reads),
            "writes": effect_region_str(summary.effect_writes),
            "has_barrier": summary.has_barrier,
        }));
    }

    json!({
        "taint_warnings": taint_warnings,
        "tainted_variables": tainted_variables,
        "proc_effects": proc_effects,
        "summary": {
            "total_taint_warnings": taint_warnings.len(),
            "tainted_variable_count": tainted_variables.len(),
            "pure_proc_count": pure_count,
            "impure_proc_count": impure_count,
        },
    })
}

/// `tcl dataflow` — taint warnings, tainted variables, and per-proc
/// side-effect classification.
pub fn run_dataflow(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);
    let data = build_dataflow_graph(&source, &input.dialect);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let summary = data.get("summary").cloned().unwrap_or_else(|| json!({}));
    let get = |key: &str| summary.get(key).and_then(Value::as_i64).unwrap_or(0);
    let mut lines = vec![format!(
        "dataflow: taintWarnings={} taintedVars={} pure={} impure={}",
        get("total_taint_warnings"),
        get("tainted_variable_count"),
        get("pure_proc_count"),
        get("impure_proc_count"),
    )];

    let empty: Vec<Value> = Vec::new();
    let warnings = data
        .get("taint_warnings")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    if !warnings.is_empty() {
        lines.push("taint warnings:".to_owned());
        for warning in warnings {
            let line_no = warning
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(|| "?".to_owned(), |l| (l + 1).to_string());
            let code = warning.get("code").and_then(Value::as_str).unwrap_or("");
            let message = warning.get("message").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("  {code} line {line_no}: {message}"));
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
