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

//! MCP tool registry + handlers — each calls the Rust analysis crates directly.
//!
//! Handlers take the JSON `arguments` object and return a JSON result `Value`;
//! [`call`] renders it into the MCP `content[].text` wire shape (a JSON string),
//! kept stable so existing clients are unaffected.

use serde_json::{Map, Value, json};
use tcl_compiler::analyser::{Analyser, AnalysisResult, Diagnostic};
use tcl_dialect::KNOWN_DIALECTS;
use tcl_lexer::{LineIndex, SourceMap, Span, Utf16Col};
use tcl_lsp_core::definition::LspRange;
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;

const IRULES_DIALECT: &str = "f5-irules";

const DEFAULT_DIALECT: &str = "tcl9.0";

/// Process-wide session dialect — the detection default set by `set_dialect`,
/// held in a `_session_dialect` module global.
fn session_dialect() -> &'static std::sync::Mutex<String> {
    static SESSION: std::sync::OnceLock<std::sync::Mutex<String>> = std::sync::OnceLock::new();
    SESSION.get_or_init(|| std::sync::Mutex::new(DEFAULT_DIALECT.to_owned()))
}

/// The dialect to use: an explicit non-empty `dialect` argument, else detected
/// from the source content with the session dialect as the fallback default.
fn resolve_dialect(args: &Value, source: &str) -> String {
    match args.get("dialect").and_then(Value::as_str) {
        Some(d) if !d.is_empty() => d.to_owned(),
        _ => {
            let session = session_dialect()
                .lock()
                .expect("session dialect lock")
                .clone();
            // `detect_dialect`'s default must be `&'static`; map the session
            // name onto a known dialect, else fall back to the built-in default.
            let default = KNOWN_DIALECTS
                .iter()
                .copied()
                .find(|&d| d == session)
                .unwrap_or(DEFAULT_DIALECT);
            tcl_registry::detect_dialect(source, None, default).to_owned()
        }
    }
}

/// The dialect for a tool whose `source` is **not** Tcl code to detect from —
/// a `.tclspec` pack, say: an explicit non-empty `dialect` argument, else the
/// session dialect.
///
/// Deliberately skips [`tcl_registry::detect_dialect`]. A spec pack's text
/// *describes* commands rather than calling them, so detecting a dialect from
/// its content would answer a different question than "which registry should
/// this pack be checked against".
pub(crate) fn declared_dialect(args: &Value) -> String {
    match args.get("dialect").and_then(Value::as_str) {
        Some(d) if !d.is_empty() => d.to_owned(),
        _ => session_dialect()
            .lock()
            .expect("session dialect lock")
            .clone(),
    }
}

fn set_dialect(args: &Value) -> Value {
    let requested = arg_str(args, "dialect").to_owned();
    let mut guard = session_dialect().lock().expect("session dialect lock");
    let previous = std::mem::replace(&mut *guard, requested.clone());
    let changed = previous != requested;
    json!({ "dialect": requested, "previous": previous, "changed": changed })
}

fn arg_str<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or("")
}

fn arg_u32(args: &Value, key: &str) -> u32 {
    args.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0)
}

/// The registry every dialect-taking tool answers from — the shared
/// per-dialect cache, **plus the shipped `.tclspec` loadables**, because the
/// EDA vendor libraries live in those now (`docs/design/spec-packs.md`) and an
/// MCP client asking about `synth_design` would otherwise be told it does not
/// exist.
fn registry(dialect: &str) -> std::sync::Arc<CommandRegistry> {
    tcl_spectcl::bundled::registry_for_dialect(dialect)
}

/// The byte offset of `(line, character)` (UTF-16 column) in `source`.
fn cursor(source: &str, line: u32, character: u32) -> (LineIndex, u32) {
    let idx = LineIndex::new(source);
    let off = idx.offset_at_utf16(line, Utf16Col::new(character), source);
    (idx, off)
}

fn refactoring_json(source: &str, r: &tcl_lsp_core::refactor::Refactoring) -> Value {
    json!({
        "title": r.title,
        "rewritten": r.apply(source),
        "edit_count": r.edits.len(),
    })
}

/// Analyse `source` under `dialect` (fresh analyser per call, like the facades).
///
/// [`registry`] first, then the overlay key it built: the analyser resolves its
/// own registry from the dialect profile, so without the key it would miss the
/// bundled `.tclspec` loadables and call every EDA vendor command unknown.
fn analyse(source: &str, dialect: &str) -> AnalysisResult {
    let _ = registry(dialect);
    Analyser::new()
        .with_pack_overlay(tcl_spectcl::bundled::packs().key)
        .analyse(source, dialect)
}

/// `"true"`/`"1"`/`"yes"` (case-insensitive) or a JSON `true` — else `false`.
fn arg_bool(args: &Value, key: &str) -> bool {
    match args.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => {
            matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")
        }
        _ => false,
    }
}

/// `{line, character}` for a byte offset, using byte columns (matching the
/// facade's `SourceMap`-based diagnostic ranges).
fn byte_pos(sm: &SourceMap<'_>, offset: u32) -> Value {
    let p = sm.position_at(offset);
    json!({ "line": p.line, "character": p.character.get() })
}

fn byte_range(sm: &SourceMap<'_>, span: Span) -> Value {
    json!({ "start": byte_pos(sm, span.start()), "end": byte_pos(sm, span.end()) })
}

/// The nested LSP range shape `{start:{line,character}, end:{…}}` (UTF-16).
fn lsp_range_json(r: &LspRange) -> Value {
    json!({
        "start": { "line": r.start_line, "character": r.start_character },
        "end": { "line": r.end_line, "character": r.end_character },
    })
}

/// Serialise one analyser diagnostic to `{code, severity, message, range,
/// category, fixes?}` (the `_facade_diagnostic_to_dict` wire shape).
fn diag_to_json(d: &Diagnostic, sm: &SourceMap<'_>) -> Value {
    let code = d.code.as_str();
    let mut obj = json!({
        "code": code,
        "severity": d.severity.as_str(),
        "message": d.message,
        "range": byte_range(sm, d.span),
        "category": crate::diag_meta::meta().categorise(code),
    });
    if !d.fixes.is_empty() {
        let fixes: Vec<Value> = d
            .fixes
            .iter()
            .map(|f| {
                json!({
                    "range": byte_range(sm, f.span),
                    "new_text": f.new_text,
                    "description": f.description,
                })
            })
            .collect();
        obj.as_object_mut()
            .expect("json object")
            .insert("fixes".to_owned(), Value::Array(fixes));
    }
    obj
}

/// Serialise a control-flow document symbol tree (nested-range shape).
fn doc_symbol_to_json(sym: &tcl_lsp_core::document_symbols::DocumentSymbol) -> Value {
    let range = |r: &tcl_lsp_core::document_symbols::LineRange| {
        json!({
            "start": { "line": r.start_line, "character": r.start_character },
            "end": { "line": r.end_line, "character": r.end_character },
        })
    };
    let mut node = json!({
        "name": sym.name,
        "kind": sym.kind.as_str(),
        "range": range(&sym.range),
        "selection_range": range(&sym.selection_range),
        "children": sym.children.iter().map(doc_symbol_to_json).collect::<Vec<_>>(),
    });
    if let Some(detail) = &sym.detail {
        node.as_object_mut()
            .expect("json object")
            .insert("detail".to_owned(), json!(detail));
    }
    node
}

/// iRule events in canonical firing order as `{index, name, multiplicity}`
/// (1-based index) — the `ordered_events` shape.
fn event_order_list(source: &str) -> Vec<Value> {
    let events = EventRegistry::build();
    events
        .order_events_for_file(source)
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let multiplicity = events.event_multiplicity(&name);
            json!({ "index": i + 1, "name": name, "multiplicity": multiplicity })
        })
        .collect()
}

/// iRule `when EVENT` handlers as `{name, line}` (0-based line), first
/// appearance only — mirrors the `_detect_events` regex
/// `^\s*when\s+([A-Z][A-Z0-9_]{2,})\b`.
fn detect_events(source: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (line_no, line) in source.lines().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("when") else {
            continue;
        };
        // `when` must be followed by at least one whitespace character.
        let after = rest.trim_start();
        if after.len() == rest.len() {
            continue;
        }
        // Capture the maximal `[A-Z0-9_]` run; `\b` then holds automatically.
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if name.len() < 3 || !name.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        if seen.insert(name.clone()) {
            out.push(json!({ "name": name, "line": line_no }));
        }
    }
    out
}

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

// ── Individual tool handlers ──────────────────────────────────────────

fn call_graph(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_lsp_core::graphs::call_graph(source, &registry(&dialect), &dialect)
}

fn symbol_graph(args: &Value) -> Value {
    let source = arg_str(args, "source");
    tcl_lsp_core::graphs::symbol_graph(source, &resolve_dialect(args, source))
}

fn dataflow_graph(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_lsp_core::graphs::dataflow_graph(source, &registry(&dialect), &dialect)
}

fn def_use_chains(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let mut graph = tcl_lsp_core::graphs::def_use_graph(source, &registry(&dialect), &dialect);
    // Optional variable filter.
    let variable = arg_str(args, "variable");
    if !variable.is_empty()
        && let Some(functions) = graph.get_mut("functions").and_then(Value::as_array_mut)
    {
        for func in functions {
            if let Some(nodes) = func.get_mut("nodes").and_then(Value::as_array_mut) {
                nodes.retain(|n| n.get("name").and_then(Value::as_str) == Some(variable));
            }
            if let Some(edges) = func.get_mut("edges").and_then(Value::as_array_mut) {
                edges.retain(|e| {
                    e.get("fromName").and_then(Value::as_str) == Some(variable)
                        || e.get("toName").and_then(Value::as_str) == Some(variable)
                });
            }
        }
    }
    graph
}

fn memory_aliases(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_lsp_core::graphs::memory_alias_graph(source, &registry(&dialect), &dialect)
}

fn diagram(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_diagram::diagram_data_for_dialect(source, &registry(&dialect), &dialect)
}

fn detect_dialect(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let filename = args.get("filename").and_then(Value::as_str);
    json!(tcl_registry::detect_dialect(
        source,
        filename,
        DEFAULT_DIALECT
    ))
}

fn event_order(args: &Value) -> Value {
    let ordered = event_order_list(arg_str(args, "source"));
    json!({ "events": ordered, "total": ordered.len() })
}

fn format_source(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    // The tool's resolved dialect drives the formatter as one fact (issue
    // #1465): lexer grammar, rewrite-candidate release, and forward range.
    // The default config formatted every dialect as modern Tcl.
    let mut config = tcl_lsp_core::formatting::FormatterConfig::for_dialect(&dialect);
    if let Some(n) = args.get("indent_size").and_then(Value::as_u64) {
        config.indent_size = usize::try_from(n).unwrap_or(config.indent_size);
    }
    if let Some(n) = args.get("max_line_length").and_then(Value::as_u64) {
        config.max_line_length = usize::try_from(n).unwrap_or(config.max_line_length);
    }
    if arg_str(args, "indent_style").eq_ignore_ascii_case("tabs") {
        config.indent_style = tcl_lsp_core::formatting::IndentStyle::Tabs;
    }
    let formatted =
        tcl_lsp_core::formatting::engine::format_tcl(source, &config, &registry(&dialect));
    json!({ "formatted": formatted, "changed": formatted != source })
}

fn optimize(args: &Value) -> Value {
    use tcl_compiler::optimiser::optimise_source_multipass_filtered;
    use tcl_compiler::optimiser::profiles::{OptimisationProfile, profile_to_disabled};

    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let profile = OptimisationProfile::parse({
        let p = arg_str(args, "profile");
        if p.is_empty() { "full" } else { p }
    });
    let disabled: std::collections::HashSet<String> = profile_to_disabled(profile)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let (optimised, opts, iterations) = optimise_source_multipass_filtered(
        source,
        &registry(&dialect),
        Some(dialect.as_str()),
        profile.max_iterations(),
        &disabled,
    );

    let line_index = LineIndex::new(source);
    let pos = |offset: u32| {
        let p = line_index.position_at_utf16(offset, source);
        json!({ "line": p.line, "character": p.character.get() })
    };
    let optimizations: Vec<Value> = opts
        .iter()
        .map(|o| {
            let mut item = json!({
                "code": o.code.to_string(),
                "message": o.message,
                "range": { "start": pos(o.span.start()), "end": pos(o.span.end()) },
                "replacement": o.replacement,
            });
            let map = item.as_object_mut().expect("json object");
            if let Some(g) = o.group {
                map.insert("group".to_owned(), json!(g));
            }
            if o.hint_only {
                map.insert("hintOnly".to_owned(), json!(true));
            }
            item
        })
        .collect();

    json!({
        "optimizations": optimizations,
        "total": optimizations.len(),
        "optimized_source": optimised,
        "changed": optimised != source,
        "profile": profile.name(),
        "iterations": iterations,
        "multi_pass": profile.is_multi_pass(),
    })
}

fn compile_wasm(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let registry = registry(&dialect);
    let unit = tcl_compiler::compilation_unit::CompilationUnit::build_for_dialect(
        source, &registry, false, &dialect,
    );
    let mut wasm = tcl_compiler::codegen::wasm::compile_wasm(
        &unit,
        &registry,
        tcl_compiler::codegen::wasm::WasmCompileOptions::hosted(),
    );
    let codegen_plan = wasm.plan.as_str();
    let semantic_decline = wasm
        .plan
        .semantic_decline()
        .map(tcl_compiler::codegen::wasm::WasmSemanticDecline::as_str);
    let semantic_decline_detail = wasm
        .plan
        .semantic_decline()
        .map(tcl_compiler::codegen::wasm::WasmSemanticDecline::detail_kind);
    let function_count = wasm.functions.len();
    let bytes = wasm.to_bytes();
    let wat = wasm.to_wat();
    json!({
        "pipeline": "canonical",
        "codegen_plan": codegen_plan,
        "semantic_decline": semantic_decline,
        "semantic_decline_detail": semantic_decline_detail,
        "wat": wat,
        "byte_length": bytes.len(),
        "function_count": function_count,
    })
}

/// Run a cursor-addressed refactoring, returning its result dict or `null`.
fn refactor_at(
    args: &Value,
    f: impl FnOnce(
        &str,
        u32,
        &CommandRegistry,
        &LineIndex,
    ) -> Option<tcl_lsp_core::refactor::Refactoring>,
) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    match f(source, off, &registry(&dialect), &idx) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

fn inline_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = analyse(source, &dialect);
    match tcl_lsp_core::refactor::inline_variable(source, off, &analysis, &registry(&dialect), &idx)
    {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

fn if_to_switch(args: &Value) -> Value {
    refactor_at(args, tcl_lsp_core::refactor::if_to_switch)
}

fn switch_to_dict(args: &Value) -> Value {
    refactor_at(args, tcl_lsp_core::refactor::switch_to_dict)
}

fn brace_expr(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (_idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    match tcl_lsp_core::refactor::brace_expr(source, off, &registry(&dialect)) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

// ── Diagnostics tools ─────────────────────────────────────────────────

fn analyze(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let sm = SourceMap::new(source);
    let diagnostics: Vec<Value> = analysis
        .diagnostics
        .iter()
        .map(|d| diag_to_json(d, &sm))
        .collect();
    let symbols: Vec<Value> = tcl_lsp_core::document_symbols::document_symbols(source, &dialect)
        .iter()
        .map(doc_symbol_to_json)
        .collect();
    json!({
        "diagnostics": diagnostics,
        "diagnostic_count": analysis.diagnostics.len(),
        "symbols": symbols,
        "events": detect_events(source),
        "event_order": event_order_list(source),
    })
}

fn validate(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let sm = SourceMap::new(source);
    let meta = crate::diag_meta::meta();
    let mut categories = Map::new();
    for (key, label) in &meta.category_order {
        let items: Vec<Value> = analysis
            .diagnostics
            .iter()
            .filter(|d| meta.categorise(d.code.as_str()) == key)
            .map(|d| diag_to_json(d, &sm))
            .collect();
        if !items.is_empty() {
            categories.insert(key.clone(), json!({ "label": label, "items": items }));
        }
    }
    json!({ "categories": Value::Object(categories), "total": analysis.diagnostics.len() })
}

fn review(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let sm = SourceMap::new(source);
    let meta = crate::diag_meta::meta();
    let filt = |set: &std::collections::HashSet<String>| -> Vec<Value> {
        analysis
            .diagnostics
            .iter()
            .filter(|d| set.contains(d.code.as_str()))
            .map(|d| diag_to_json(d, &sm))
            .collect()
    };
    let security = filt(&meta.security_codes);
    let taint = filt(&meta.taint_codes);
    let thread = filt(&meta.thread_codes);
    let total = security.len() + taint.len() + thread.len();
    json!({ "security": security, "taint": taint, "thread_safety": thread, "total": total })
}

fn find_legacy(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let sm = SourceMap::new(source);
    // Shared with `tcl-cli`'s `find-legacy` verb (`tcl_cli::CONVERTIBLE_CODES`/
    // `conversion_for`) rather than a second hand-duplicated copy of the same
    // 6-code table.
    let patterns: Vec<Value> = analysis
        .diagnostics
        .iter()
        .filter(|d| tcl_cli::CONVERTIBLE_CODES.contains(&d.code.as_str()))
        .map(|d| {
            let mut obj = diag_to_json(d, &sm);
            let conversion = tcl_cli::conversion_for(d.code.as_str());
            obj.as_object_mut()
                .expect("json object")
                .insert("conversion".to_owned(), json!(conversion));
            obj
        })
        .collect();
    json!({ "total": patterns.len(), "patterns": patterns })
}

#[cfg(test)]
mod format_dialect_tests {
    use super::*;

    /// TMM accepts the `}{` ghost separator (`if {expr}{body}`); stock Tcl
    /// does not, so the two dialects format these bytes differently.
    const GHOST_SEPARATOR_IRULE: &str =
        "when HTTP_REQUEST {\n    if { 1 }{\n        pool p\n    }\n}\n";

    #[test]
    fn the_format_tool_resolves_the_requested_dialect() {
        // Issue #1465: `format_source` started from `FormatterConfig::default()`
        // and never projected the tool's resolved dialect onto it, so an iRule
        // was tokenised with the Tcl 9 lexer.
        for dialect in ["f5-irules", "irules"] {
            let result = format_source(&json!({
                "source": GHOST_SEPARATOR_IRULE,
                "dialect": dialect,
            }));
            let formatted = result["formatted"].as_str().expect("formatted string");
            assert!(formatted.contains("} {"), "{dialect}: {formatted}");
            assert!(!formatted.contains("}{"), "{dialect}: {formatted}");
        }
    }

    #[test]
    fn the_format_tool_leaves_the_ghost_separator_alone_under_core_tcl() {
        let result = format_source(&json!({
            "source": GHOST_SEPARATOR_IRULE,
            "dialect": "tcl9.0",
        }));
        let formatted = result["formatted"].as_str().expect("formatted string");
        assert!(formatted.contains("}{"), "{formatted}");
    }
}

#[cfg(test)]
mod find_legacy_tests {
    use super::*;

    /// The convertible-code/conversion-hint table the MCP `find-legacy` tool
    /// reads comes straight from `tcl_cli`, so this also guards against the
    /// shared table silently losing the conversion hint.
    #[test]
    fn reports_unbraced_expr_with_its_conversion_hint() {
        let source = "proc check {a b} {\n    set total [expr $a + $b]\n    return $total\n}\n";
        let result = find_legacy(&json!({ "source": source }));
        let total = result["total"].as_u64().expect("total is a number");
        assert!(total >= 1, "{result}");
        let patterns = result["patterns"].as_array().expect("patterns array");
        let w100 = patterns
            .iter()
            .find(|p| p["code"] == "W100")
            .unwrap_or_else(|| panic!("no W100 pattern in {result}"));
        assert_eq!(w100["conversion"], "Unbraced expr -> braced expr");
    }
}

#[cfg(test)]
mod wasm_pipeline_tests {
    use super::*;

    #[test]
    fn compile_tool_uses_canonical_semantic_plan_without_backend_parameter() {
        let schema = tool_schemas()
            .into_iter()
            .find(|(name, _, _)| *name == "compile_wasm")
            .expect("compile_wasm schema")
            .2;
        assert!(schema["properties"].get("backend").is_none());

        let result = dispatch(
            "compile_wasm",
            &json!({ "source": "string length hello", "dialect": "tcl9.0" }),
        )
        .expect("compile_wasm tool");
        assert_eq!(result["pipeline"], "canonical");
        assert_eq!(result["codegen_plan"], "generic-invoke");
        assert!(result["semantic_decline"].is_null());
        assert!(result["wat"].as_str().unwrap().contains("tcl_invoke_argv"));
    }
}

// ── Registry info tools ───────────────────────────────────────────────

fn event_info(args: &Value) -> Value {
    let event = arg_str(args, "event_name");
    let reg = registry(IRULES_DIALECT);
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let info = reg.event_info(event, &events, &profiles, None);
    let valid_command_count = info.valid_commands.len();
    json!({
        "event": info.event,
        "known": info.known,
        // The three lifecycle releases, same names and null semantics as the
        // CLI/query/snapshot surfaces; `retired_version` is exclusive.
        "introduced_version": info.lifecycle.introduced,
        "deprecated_version": info.lifecycle.deprecated,
        "retired_version": info.lifecycle.retired,
        "lifecycle_state": info.lifecycle_state.as_str(),
        "multiplicity": info.multiplicity,
        "description": info.description,
        "side": info.side,
        "transport": info.transport,
        "implied_profiles": info.implied_profiles,
        "valid_commands": info.valid_commands,
        "valid_command_count": valid_command_count,
    })
}

fn command_info(args: &Value) -> Value {
    let command = arg_str(args, "command_name").trim();
    let reg = registry(IRULES_DIALECT);
    let Some(spec) = ({
        use tcl_registry::ProfileQueries;
        tcl_dialect::DialectProfile::irules().resolve_command(&reg, command)
    }) else {
        return json!({ "command": command, "found": false });
    };
    // Optional BIG-IP target release: report the declared version range
    // (explicit introduction data, or the 15.0 axis baseline) and whether
    // the command exists at the target.
    let bigip_version = args
        .get("bigip_version")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let keyed_range = {
        use tcl_registry::ProfileQueries;
        tcl_dialect::DialectProfile::irules().keyed_version_range(spec)
    };
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let mut obj = json!({ "command": command, "found": true });
    let m = obj.as_object_mut().expect("json object");
    if let Some(h) = spec.hover {
        if !h.summary.is_empty() {
            m.insert("summary".to_owned(), json!(h.summary));
        }
        if !h.synopsis.is_empty() {
            m.insert("synopsis".to_owned(), json!(h.synopsis));
        }
    }
    let switches = {
        use tcl_registry::ProfileQueries;
        tcl_dialect::DialectProfile::irules().available_option_names(spec)
    };
    if !switches.is_empty() {
        m.insert("switches".to_owned(), json!(switches));
    }
    let valid_events = reg.irules_events_for_command(command, &events, &profiles);
    if !valid_events.is_empty() {
        m.insert("valid_events".to_owned(), json!(valid_events));
    }
    if let Some((min, max)) = keyed_range {
        m.insert("bigip_min_version".to_owned(), json!(min));
        m.insert("bigip_max_version".to_owned(), json!(max));
        if let Some(target) = bigip_version {
            m.insert(
                "available_at_bigip_version".to_owned(),
                json!(tcl_registry::version::within_range(target, min, max)),
            );
        }
    }
    obj
}

// ── LSP-feature tools ─────────────────────────────────────────────────

const SOURCE_URI: &str = "file:///source.tcl";

fn symbols(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let syms: Vec<Value> = tcl_lsp_core::document_symbols::document_symbols(source, &dialect)
        .iter()
        .map(doc_symbol_to_json)
        .collect();
    json!({ "symbols": syms })
}

fn hover(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    match tcl_lsp_core::hover::hover(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        Some(&registry(&dialect)),
    ) {
        Some(h) => json!({ "contents": h.value }),
        None => Value::Null,
    }
}

fn complete(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let items = tcl_lsp_core::completion::completions(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        Some(&registry(&dialect)),
        None,
        &dialect,
    );
    let items_json: Vec<Value> = items
        .iter()
        .map(|i| {
            let mut obj = json!({ "label": i.label, "kind": completion_kind_str(i.kind), "insert_text": i.insert_text });
            let m = obj.as_object_mut().expect("json object");
            if let Some(d) = &i.detail {
                m.insert("detail".to_owned(), json!(d));
            }
            if let Some(d) = &i.documentation {
                m.insert("documentation".to_owned(), json!(d));
            }
            if let Some(s) = &i.sort_text {
                m.insert("sort_text".to_owned(), json!(s));
            }
            obj
        })
        .collect();
    json!({ "items": items_json, "total": items_json.len() })
}

fn completion_kind_str(k: tcl_lsp_core::completion::CompletionKind) -> &'static str {
    use tcl_lsp_core::completion::CompletionKind::{EnumValue, Function, Snippet, Variable};
    match k {
        Variable => "variable",
        Function => "function",
        EnumValue => "enum_member",
        Snippet => "snippet",
    }
}

fn goto_definition(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let ranges = tcl_lsp_core::definition::definition(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
    );
    let locations: Vec<Value> = ranges
        .iter()
        .map(|r| json!({ "uri": SOURCE_URI, "range": lsp_range_json(r) }))
        .collect();
    json!({ "locations": locations })
}

fn find_references(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let ranges = tcl_lsp_core::references::references(
        source,
        &dialect,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        true,
    );
    let refs: Vec<Value> = ranges
        .iter()
        .map(|r| json!({ "uri": SOURCE_URI, "range": lsp_range_json(r) }))
        .collect();
    json!({ "references": refs, "total": refs.len() })
}

fn rename(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let edits = tcl_lsp_core::rename::rename(
        source,
        &dialect,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        arg_str(args, "new_name"),
        &analysis,
        Some(&registry(&dialect)),
    );
    if edits.is_empty() {
        return Value::Null;
    }
    let edits_json: Vec<Value> = edits
        .iter()
        .map(|e| json!({ "range": lsp_range_json(&e.range), "new_text": e.new_text }))
        .collect();
    json!({ "edits": edits_json, "total": edits.len() })
}

fn code_actions(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let range = LspRange {
        start_line: arg_u32(args, "start_line"),
        start_character: arg_u32(args, "start_character"),
        end_line: arg_u32(args, "end_line"),
        end_character: arg_u32(args, "end_character"),
    };
    // The MCP tool analyses one standalone source string with no workspace
    // behind it, so the analyser's own diagnostics *are* the published set.
    let actions: Vec<Value> = tcl_lsp_core::code_actions::code_actions(
        source,
        range,
        Some(&analysis),
        &analysis.diagnostics,
    )
    .iter()
        .map(|a| {
            let mut obj = json!({ "title": a.title, "kind": a.kind.as_str() });
            let m = obj.as_object_mut().expect("json object");
            if !a.edits.is_empty() {
                let edits: Vec<Value> = a
                    .edits
                    .iter()
                    .map(|e| json!({ "uri": SOURCE_URI, "range": lsp_range_json(&e.range), "new_text": e.new_text }))
                    .collect();
                m.insert("edits".to_owned(), Value::Array(edits));
            }
            if let Some(cmd) = &a.command {
                m.insert(
                    "command".to_owned(),
                    json!({ "command": cmd.command, "args": cmd.args, "string_args": cmd.string_args }),
                );
            }
            if let Some(dg) = &a.data_group_definition {
                m.insert("data_group_definition".to_owned(), json!(dg));
            }
            obj
        })
        .collect();
    json!({ "actions": actions })
}

// ── Refactor tools ────────────────────────────────────────────────────

fn extract_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
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
        start_off,
        end_off,
        var_name,
        &line_index,
    ) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

fn extract_datagroup(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let (line_index, cursor) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let dg_name = arg_str(args, "dg_name");
    let reg = registry(IRULES_DIALECT);
    let Some(r) =
        tcl_lsp_core::refactor::extract_to_datagroup(source, cursor, dg_name, &reg, &line_index)
    else {
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

fn refactor(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let reg = registry(&dialect);
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
    let mut available: Vec<Value> = Vec::new();
    let mut push = |tool: &str, r: Option<tcl_lsp_core::refactor::Refactoring>| {
        if let Some(r) = r {
            available.push(json!({ "tool": tool, "title": r.title }));
        }
    };
    // Extract-variable only applies to a non-empty selection.
    if start_off != end_off {
        push(
            "extract_variable",
            tcl_lsp_core::refactor::extract_variable(
                source,
                start_off,
                end_off,
                "result",
                &line_index,
            ),
        );
    }
    push(
        "inline_variable",
        tcl_lsp_core::refactor::inline_variable(source, start_off, &analysis, &reg, &line_index),
    );
    push(
        "if_to_switch",
        tcl_lsp_core::refactor::if_to_switch(source, start_off, &reg, &line_index),
    );
    push(
        "switch_to_dict",
        tcl_lsp_core::refactor::switch_to_dict(source, start_off, &reg, &line_index),
    );
    push(
        "brace_expr",
        tcl_lsp_core::refactor::brace_expr(source, start_off, &reg),
    );
    push(
        "extract_datagroup",
        tcl_lsp_core::refactor::extract_to_datagroup(source, start_off, "", &reg, &line_index),
    );
    json!({ "total": available.len(), "available": available })
}

// ── Docstring tools ───────────────────────────────────────────────────

fn generate_docstring(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let proc_name = arg_str(args, "proc_name");
    let style = match arg_str(args, "style") {
        "" => "doxygen",
        s => s,
    };
    let decoration = arg_bool(args, "decoration");
    let analysis = analyse(source, &dialect);
    // Resolve `proc_name` to its `ProcDef`: the qualified spelling (`::name`,
    // or the exact name as given) first, then — for a bare name that names a
    // proc in some namespace — the lexicographically smallest qualified name
    // among the same-named procs.  The tool has no call-site namespace to
    // resolve against, so an ambiguous bare name resolves *deterministically*
    // rather than in `HashMap` iteration order (which picked an arbitrary
    // same-named proc across namespaces run to run).
    let qualified = format!("::{proc_name}");
    let proc = analysis
        .all_procs
        .values()
        .find(|p| p.qualified_name == qualified || p.qualified_name == proc_name)
        .or_else(|| {
            // drift-ok: a user-typed bare name with no cursor context — the
            // deterministic (lexicographically-least) simple-name fallback,
            // the same discipline as definition.rs::fallback_proc_by_simple_name.
            analysis
                .all_procs
                .values()
                .filter(|p| p.name == proc_name)
                .min_by(|a, b| a.qualified_name.cmp(&b.qualified_name))
        });
    match proc {
        Some(proc) => {
            let tag = tcl_lsp_core::formatting::resolve_tag_style(style);
            let docstring = tcl_lsp_core::formatting::generate_stub_for_proc(
                proc, tag, decoration, '.', 70, "",
            );
            json!({ "proc": proc_name, "docstring": docstring })
        }
        None => json!({ "error": format!("Proc '{proc_name}' not found") }),
    }
}

fn read_proc_docs(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let mut procs: Vec<_> = analysis.all_procs.values().collect();
    procs.sort_by_key(|p| p.name_span.start());
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

fn update_docstrings(args: &Value) -> Value {
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
    let total_procs = analysis.all_procs.len();

    // Undocumented procs, bottom-up so earlier insertions don't shift later
    // line numbers.
    let mut targets: Vec<_> = analysis
        .all_procs
        .values()
        .filter(|p| p.doc.is_empty())
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

// ── Minification tool ─────────────────────────────────────────────────

fn unminify_error(args: &Value) -> Value {
    let error_message = arg_str(args, "error_message");
    let symbol_map = arg_str(args, "symbol_map");
    let minified = arg_str(args, "minified_source");
    let original = arg_str(args, "original_source");
    let map = tcl_lsp_core::minify::SymbolMap::parse(symbol_map);
    let mut translated = error_message.to_owned();
    if !minified.is_empty() && !original.is_empty() {
        translated = tcl_lsp_core::minify::remap_line_references(&translated, minified, original);
    }
    let translated = tcl_lsp_core::minify::unminify_error(&translated, &map);
    json!({
        "original_error": error_message,
        "translated_error": translated,
        "changed": translated != error_message,
    })
}

// ── Help tool ─────────────────────────────────────────────────────────

fn help(args: &Value) -> Value {
    let topic = arg_str(args, "topic");
    let dialect = match args.get("dialect").and_then(Value::as_str) {
        Some(d) if !d.is_empty() => d,
        _ => "",
    };
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(20);
    tcl_cli::help_json(topic, dialect, limit)
}

// ── Registry table ────────────────────────────────────────────────────

type Handler = fn(&Value) -> Value;

struct ToolDef {
    name: &'static str,
    description: &'static str,
    /// `(property_name, json_type, description)` for the input schema.
    params: &'static [(&'static str, &'static str, &'static str)],
    required: &'static [&'static str],
    handler: Handler,
}

type Param = (&'static str, &'static str, &'static str);

const SRC: Param = ("source", "string", "Tcl or iRules source code");
const DIALECT: Param = (
    "dialect",
    "string",
    "Language dialect; auto-detected if empty",
);
const LINE: Param = ("line", "integer", "0-based line of the cursor");
const CHAR: Param = ("character", "integer", "0-based character of the cursor");
const START_LINE: Param = (
    "start_line",
    "integer",
    "0-based start line of the selection",
);
const START_CHAR: Param = (
    "start_character",
    "integer",
    "0-based start character of the selection",
);
const END_LINE: Param = ("end_line", "integer", "0-based end line of the selection");
const END_CHAR: Param = (
    "end_character",
    "integer",
    "0-based end character of the selection",
);
const STYLE: Param = (
    "style",
    "string",
    "Docstring style: 'doxygen' (default) or 'plain'",
);
const DECORATION: Param = (
    "decoration",
    "boolean",
    "Add '# ....' rule lines above/below the stub",
);

const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "call_graph",
        description: "Proc caller→callee graph with call sites, roots, and leaf procs.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: call_graph,
    },
    ToolDef {
        name: "symbol_graph",
        description: "Scope hierarchy with proc/variable definitions, references, and package requires.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: symbol_graph,
    },
    ToolDef {
        name: "dataflow_graph",
        description: "Taint warnings, tainted variables, and per-proc side-effect classification.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: dataflow_graph,
    },
    ToolDef {
        name: "def_use_chains",
        description: "SSA def-use chains + memory-SSA aliases; optional variable filter.",
        params: &[
            SRC,
            DIALECT,
            (
                "variable",
                "string",
                "Filter to a specific variable name (optional)",
            ),
        ],
        required: &["source"],
        handler: def_use_chains,
    },
    ToolDef {
        name: "memory_aliases",
        description: "Memory-SSA alias sets (upvar/global/variable) with reasons and locations.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: memory_aliases,
    },
    ToolDef {
        name: "diagram",
        description: "Control-flow diagram data ({events, procedures}) from the IR.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: diagram,
    },
    ToolDef {
        name: "detect_dialect",
        description: "Detect the Tcl dialect from source (+ optional filename).",
        params: &[
            SRC,
            (
                "filename",
                "string",
                "File name for extension-based detection (optional)",
            ),
        ],
        required: &["source"],
        handler: detect_dialect,
    },
    ToolDef {
        name: "event_order",
        description: "iRule events in canonical firing order with multiplicity.",
        params: &[SRC],
        required: &["source"],
        handler: event_order,
    },
    ToolDef {
        name: "format_source",
        description: "Format Tcl/iRules source.",
        params: &[
            SRC,
            DIALECT,
            ("indent_size", "integer", "Spaces per indent (default 4)"),
            ("indent_style", "string", "'spaces' or 'tabs'"),
            (
                "max_line_length",
                "integer",
                "Max line length (default 120)",
            ),
        ],
        required: &["source"],
        handler: format_source,
    },
    ToolDef {
        name: "optimize",
        description: "Find optimisation opportunities and produce rewritten source.",
        params: &[
            SRC,
            DIALECT,
            (
                "profile",
                "string",
                "off | readability | standard | full | aggressive",
            ),
        ],
        required: &["source"],
        handler: optimize,
    },
    ToolDef {
        name: "compile_wasm",
        description: "Compile through the canonical analysis-aware WebAssembly pipeline and \
                      return the per-script WAT module and counts.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: compile_wasm,
    },
    ToolDef {
        name: "inline_variable",
        description: "Inline a single-use variable at the cursor.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: inline_variable,
    },
    ToolDef {
        name: "if_to_switch",
        description: "Convert an if/elseif chain testing one variable to a switch.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: if_to_switch,
    },
    ToolDef {
        name: "switch_to_dict",
        description: "Convert a switch whose arms set one variable to a dict lookup.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: switch_to_dict,
    },
    ToolDef {
        name: "brace_expr",
        description: "Brace an unbraced expr argument for safety/performance.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: brace_expr,
    },
    ToolDef {
        name: "analyze",
        description: "Full analysis: diagnostics (+category), document symbols, detected events, and event firing order.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: analyze,
    },
    ToolDef {
        name: "validate",
        description: "Diagnostics grouped by category (security, taint, thread-safety, control-flow, performance, style, …).",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: validate,
    },
    ToolDef {
        name: "review",
        description: "Security, taint, and thread-safety diagnostics for a focused review.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: review,
    },
    ToolDef {
        name: "find-legacy",
        description: "Auto-convertible legacy patterns with a modernisation hint per finding.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: find_legacy,
    },
    ToolDef {
        name: "event_info",
        description: "iRules event metadata: multiplicity, side, transport, implied profiles, valid commands.",
        params: &[(
            "event_name",
            "string",
            "iRules event name, e.g. HTTP_REQUEST",
        )],
        required: &["event_name"],
        handler: event_info,
    },
    ToolDef {
        name: "command_info",
        description: "Registry metadata for a command: summary, synopsis, switches, valid events.",
        params: &[
            ("command_name", "string", "Command name, e.g. HTTP::uri"),
            (
                "bigip_version",
                "string",
                "Optional target BIG-IP release; reports whether the command exists there",
            ),
        ],
        required: &["command_name"],
        handler: command_info,
    },
    ToolDef {
        name: "symbols",
        description: "Document symbol tree (procs, classes, methods, variables) with ranges.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: symbols,
    },
    ToolDef {
        name: "hover",
        description: "Hover documentation (markdown) for the symbol at the cursor.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: hover,
    },
    ToolDef {
        name: "complete",
        description: "Completion items at the cursor (variables, procs, switches, subcommands, snippets).",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: complete,
    },
    ToolDef {
        name: "goto_definition",
        description: "Definition location(s) for the symbol at the cursor.",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: goto_definition,
    },
    ToolDef {
        name: "find_references",
        description: "All references to the symbol at the cursor (including its declaration).",
        params: &[SRC, LINE, CHAR, DIALECT],
        required: &["source", "line", "character"],
        handler: find_references,
    },
    ToolDef {
        name: "rename",
        description: "Text edits to rename the symbol at the cursor; null when not renameable.",
        params: &[
            SRC,
            LINE,
            CHAR,
            ("new_name", "string", "New symbol name"),
            DIALECT,
        ],
        required: &["source", "line", "character", "new_name"],
        handler: rename,
    },
    ToolDef {
        name: "code_actions",
        description: "Code actions (quick-fixes + refactors) for a selection range.",
        params: &[SRC, START_LINE, START_CHAR, END_LINE, END_CHAR, DIALECT],
        required: &[
            "source",
            "start_line",
            "start_character",
            "end_line",
            "end_character",
        ],
        handler: code_actions,
    },
    ToolDef {
        name: "extract_variable",
        description: "Extract a selected expression into a new `set var …` binding.",
        params: &[
            SRC,
            START_LINE,
            START_CHAR,
            END_LINE,
            END_CHAR,
            ("var_name", "string", "New variable name (default 'result')"),
        ],
        required: &[
            "source",
            "start_line",
            "start_character",
            "end_line",
            "end_character",
        ],
        handler: extract_variable,
    },
    ToolDef {
        name: "extract_datagroup",
        description: "Extract an if/switch over literals into an iRules data-group + lookup.",
        params: &[
            SRC,
            LINE,
            CHAR,
            (
                "dg_name",
                "string",
                "Data-group name; auto-generated if empty",
            ),
        ],
        required: &["source", "line", "character"],
        handler: extract_datagroup,
    },
    ToolDef {
        name: "refactor",
        description: "List the refactorings available at a selection (probe of all refactors).",
        params: &[SRC, START_LINE, START_CHAR, END_LINE, END_CHAR, DIALECT],
        required: &[
            "source",
            "start_line",
            "start_character",
            "end_line",
            "end_character",
        ],
        handler: refactor,
    },
    ToolDef {
        name: "generate_docstring",
        description: "Generate a docstring stub for a named proc.",
        params: &[
            SRC,
            ("proc_name", "string", "Proc to document"),
            STYLE,
            DECORATION,
            DIALECT,
        ],
        required: &["source", "proc_name"],
        handler: generate_docstring,
    },
    ToolDef {
        name: "read_proc_docs",
        description: "Structured docs for every proc: params, parsed docstring, inferred param traits.",
        params: &[SRC, DIALECT],
        required: &["source"],
        handler: read_proc_docs,
    },
    ToolDef {
        name: "update_docstrings",
        description: "Insert docstring stubs above every undocumented proc; returns rewritten source.",
        params: &[SRC, STYLE, DECORATION, DIALECT],
        required: &["source"],
        handler: update_docstrings,
    },
    ToolDef {
        name: "unminify_error",
        description: "Translate a minified error back to original symbol names / line numbers.",
        params: &[
            ("error_message", "string", "The error text to translate"),
            ("symbol_map", "string", "Minified→original symbol map"),
            (
                "minified_source",
                "string",
                "Minified source (optional, for line remapping)",
            ),
            (
                "original_source",
                "string",
                "Original source (optional, for line remapping)",
            ),
        ],
        required: &["error_message", "symbol_map"],
        handler: unminify_error,
    },
    ToolDef {
        name: "set_dialect",
        description: "Set the session default dialect used when a tool's dialect is auto-detected.",
        params: &[(
            "dialect",
            "string",
            "Dialect to make the session default (e.g. f5-irules, tcl9.0)",
        )],
        required: &["dialect"],
        handler: set_dialect,
    },
    ToolDef {
        name: "fakecmp_which_tmm",
        description: "Which TMM a connection 4-tuple hashes to under the fakeCMP disaggregator.",
        params: &[
            ("tmm_count", "integer", "Number of TMMs (>= 2)"),
            ("src_addr", "string", "Source IPv4"),
            ("src_port", "integer", "Source port (0-65535)"),
            ("dst_addr", "string", "Destination IPv4"),
            ("dst_port", "integer", "Destination port (0-65535)"),
        ],
        required: &["tmm_count", "src_addr", "src_port", "dst_addr", "dst_port"],
        handler: crate::fakecmp::which_tmm,
    },
    ToolDef {
        name: "fakecmp_suggest_sources",
        description: "Source tuples that spread test traffic across every TMM.",
        params: &[
            ("tmm_count", "integer", "Number of TMMs (>= 2)"),
            ("count", "integer", "Tuples per TMM (default 1)"),
            (
                "dst_addr",
                "string",
                "Destination IPv4 (default 192.168.1.100)",
            ),
            ("dst_port", "integer", "Destination port (default 443)"),
        ],
        required: &["tmm_count"],
        handler: crate::fakecmp::suggest_sources,
    },
    ToolDef {
        name: "irule_with_context",
        description: "Bundle each iRule in a BIG-IP config with the objects it references (pools, data-groups, profiles, monitors, …).",
        params: &[
            (
                "config_text",
                "string",
                "BIG-IP config text (bigip.conf/scf)",
            ),
            (
                "config_paths",
                "array",
                "Paths to config files to merge (optional)",
            ),
            (
                "rule_path",
                "string",
                "Full path of a single rule to bundle (optional)",
            ),
            ("format", "string", "'json' (default) or 'text'"),
            (
                "transitive",
                "boolean",
                "Follow transitive references (default true)",
            ),
        ],
        required: &[],
        handler: crate::bigip::irule_with_context,
    },
    ToolDef {
        name: "help",
        description: "Search the KCS feature knowledge base (empty topic → full catalogue).",
        params: &[
            ("topic", "string", "Search query; empty lists all features"),
            ("dialect", "string", "Filter features by dialect (optional)"),
            ("limit", "integer", "Max matches (default 20)"),
        ],
        required: &[],
        handler: help,
    },
    ToolDef {
        name: "xc_translate",
        description: "Translate an iRule to F5 Distributed Cloud (XC) config — terraform HCL + ves.io JSON, with per-command coverage.",
        params: &[
            SRC,
            (
                "output_format",
                "string",
                "'terraform', 'json', or 'both' (default)",
            ),
        ],
        required: &["source"],
        handler: crate::xc::xc_translate,
    },
    ToolDef {
        name: "explain_flow",
        description: "Narrate a captured session (pcap) against a BIG-IP config: matched VS, profiles, HTTP/TLS, ordered iRule events, pool decision, optional simulation.",
        params: &[
            ("pcap_path", "string", "Path to the pcap/pcapng capture"),
            ("config_text", "string", "Inline BIG-IP config (optional)"),
            (
                "config_paths",
                "array",
                "Config file paths to merge (optional)",
            ),
            (
                "use_tshark",
                "boolean",
                "Use the tshark overlay for L7/TLS decode",
            ),
            (
                "keylog_path",
                "string",
                "TLS keylog file for decryption (optional)",
            ),
            (
                "tshark_filter",
                "string",
                "Extra tshark display filter (optional)",
            ),
            (
                "max_event_body_lines",
                "integer",
                "Max iRule body lines per event (default 8)",
            ),
            ("simulate", "boolean", "Run the iRule simulation pass"),
        ],
        required: &["pcap_path"],
        handler: crate::bigip::explain_flow,
    },
    ToolDef {
        name: "tk_layout",
        description: "Extract the Tk widget tree (types, geometry managers, visual options) + pack/grid conflicts for preview.",
        params: &[SRC],
        required: &["source"],
        handler: crate::tk::tk_layout,
    },
    ToolDef {
        name: "irule_cfg_paths",
        description: "Enumerate CFG paths through an iRule to terminal actions (pool/reject/redirect/…), each annotated with priority, relevant taint warnings, and test-generation questions.",
        params: &[SRC],
        required: &["source"],
        handler: crate::irule_test::irule_cfg_paths,
    },
    ToolDef {
        name: "suggest_datagroup_extractions",
        description: "Scan iRules source for if/switch patterns extractable to data-groups; returns per-candidate value type, CIDR flag, body-shape, confidence, and whether the static extractor applies.",
        params: &[SRC],
        required: &["source"],
        handler: crate::datagroup::suggest_datagroup_extractions,
    },
    ToolDef {
        name: "generate_irule_test",
        description: "Generate a runnable iRule test scaffold (per-event + per-CFG-path cases, multi-TMM scenario) with extracted events/profiles/commands/pools/datagroups.",
        params: &[SRC],
        required: &["source"],
        handler: crate::irule_gen::generate_irule_test,
    },
    ToolDef {
        name: "spectcl_check",
        description: "Validate a SpecTcl (.tclspec) spec pack: commands parsed with the draft fields each sets, loader notices (dropped/unknown words), declared hooks with their family and shape-cacheability, hooks whose body reads past its own `-inputs` declaration (the one way a pack miscompiles silently), and collisions with the shipped registry for the target dialect.",
        params: &[
            (
                "source",
                "string",
                "SpecTcl pack source text (the contents of a .tclspec file)",
            ),
            (
                "dialect",
                "string",
                "Dialect whose shipped registry to check names against; defaults to the session dialect",
            ),
        ],
        required: &["source"],
        handler: crate::spectcl::spectcl_check,
    },
];

/// The JSON-Schema input-schema object (`{type, properties, required}`) for a
/// tool's parameters.
fn input_schema(t: &ToolDef) -> Value {
    let mut properties = Map::new();
    for (name, ty, desc) in t.params {
        properties.insert(
            (*name).to_owned(),
            json!({ "type": ty, "description": desc }),
        );
    }
    json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": t.required,
    })
}

/// Every tool as `(name, description, input_schema)` — the data the rmcp
/// `list_tools` handler turns into `rmcp::model::Tool`s.
pub fn tool_schemas() -> Vec<(&'static str, &'static str, Value)> {
    TOOLS
        .iter()
        .map(|t| (t.name, t.description, input_schema(t)))
        .collect()
}

/// Run a tool by name against its JSON `arguments`, returning the raw JSON
/// result — or `None` when the tool is unknown.
pub fn dispatch(name: &str, args: &Value) -> Option<Value> {
    TOOLS
        .iter()
        .find(|t| t.name == name)
        .map(|t| (t.handler)(args))
}

#[cfg(test)]
mod source_integrity_tests {
    use super::*;

    fn diagnostic_codes(result: &Value) -> Vec<&str> {
        result["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d["code"].as_str())
            .collect()
    }

    #[test]
    fn analyse_reports_bidi_controls_as_security_without_w108_duplication() {
        let source = "puts \"a\u{202e}b\"\n";
        let result = dispatch("analyze", &json!({ "source": source, "dialect": "tcl9.0" }))
            .expect("analyze tool");
        let diagnostics = result["diagnostics"].as_array().expect("diagnostics");
        let w305: Vec<_> = diagnostics.iter().filter(|d| d["code"] == "W305").collect();
        assert_eq!(w305.len(), 1, "{result}");
        assert_eq!(w305[0]["category"], "security");
        assert_eq!(w305[0]["severity"], "error");
        assert_eq!(w305[0].pointer("/range/start/character"), Some(&json!(7)));
        assert!(!diagnostic_codes(&result).contains(&"W108"), "{result}");
    }

    #[test]
    fn validate_and_review_include_w305_in_the_security_group() {
        let args = json!({ "source": "# \u{202e}hidden\n", "dialect": "tcl9.0" });
        let validate = dispatch("validate", &args).expect("validate tool");
        assert_eq!(
            validate.pointer("/categories/security/items/0/code"),
            Some(&json!("W305")),
            "{validate}"
        );
        let review = dispatch("review", &args).expect("review tool");
        assert_eq!(review.pointer("/security/0/code"), Some(&json!("W305")));
    }

    #[test]
    fn ordinary_right_to_left_text_is_not_a_w305_finding() {
        let result = dispatch(
            "analyze",
            &json!({ "source": "puts \"مرحبا بالعالم\"\n", "dialect": "tcl9.0" }),
        )
        .expect("analyze tool");
        assert!(!diagnostic_codes(&result).contains(&"W305"), "{result}");
    }
}

#[cfg(test)]
mod docstring_tests {
    use super::*;

    /// Two namespaces defining a `dup` proc with distinct parameter names, so a
    /// docstring reveals which one was resolved.
    const TWO_NS_DUP: &str = "namespace eval a {\n    proc dup {alphaparam} { return $alphaparam }\n}\nnamespace eval z {\n    proc dup {omegaparam} { return $omegaparam }\n}\n";

    fn docstring_for(proc_name: &str) -> String {
        let args = json!({
            "source": TWO_NS_DUP,
            "proc_name": proc_name,
            "style": "doxygen",
        });
        let res = generate_docstring(&args);
        res["docstring"].as_str().unwrap_or_default().to_owned()
    }

    #[test]
    fn qualified_name_resolves_to_that_namespace() {
        // `z::dup` names `::z::dup` exactly — its `omegaparam` shows, never
        // `::a::dup`'s `alphaparam`.
        let ds = docstring_for("z::dup");
        assert!(ds.contains("omegaparam"), "{ds}");
        assert!(!ds.contains("alphaparam"), "{ds}");
    }

    #[test]
    fn ambiguous_bare_name_is_deterministic() {
        // A bare `dup` names a proc in two namespaces; with no call-site
        // namespace to resolve against, the smallest qualified name
        // (`::a::dup`) wins — and it must win on every run, never in `HashMap`
        // iteration order.
        for _ in 0..32 {
            let ds = docstring_for("dup");
            assert!(ds.contains("alphaparam"), "{ds}");
            assert!(!ds.contains("omegaparam"), "{ds}");
        }
    }
}
