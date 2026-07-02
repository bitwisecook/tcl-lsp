//! MCP tool registry + handlers — each calls the Rust analysis crates directly.
//!
//! Handlers take the JSON `arguments` object and return a JSON result `Value`;
//! [`call`] renders it into the MCP `content[].text` wire shape (a JSON string),
//! matching what the Python server emitted so existing clients are unaffected.

use serde_json::{Map, Value, json};
use tcl_lexer::{LineIndex, Utf16Col};
use tcl_registry::{CommandRegistry, registry_for_dialect};

const DEFAULT_DIALECT: &str = "tcl9.0";

/// The dialect to use: an explicit non-empty `dialect` argument, else detected
/// from the source content, else the default.
fn resolve_dialect(args: &Value, source: &str) -> String {
    match args.get("dialect").and_then(Value::as_str) {
        Some(d) if !d.is_empty() => d.to_owned(),
        _ => tcl_registry::detect_dialect(source, None, DEFAULT_DIALECT).to_owned(),
    }
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

fn registry(dialect: &str) -> &'static CommandRegistry {
    registry_for_dialect(dialect)
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

// ── Individual tool handlers ──────────────────────────────────────────

fn call_graph(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_lsp_core::graphs::call_graph(source, registry(&dialect), &dialect)
}

fn symbol_graph(args: &Value) -> Value {
    let source = arg_str(args, "source");
    tcl_lsp_core::graphs::symbol_graph(source, &resolve_dialect(args, source))
}

fn dataflow_graph(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_lsp_core::graphs::dataflow_graph(source, registry(&dialect), &dialect)
}

fn def_use_chains(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let mut graph = tcl_lsp_core::graphs::def_use_graph(source, registry(&dialect), &dialect);
    // Optional variable filter (mirrors the Python tool).
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
    tcl_lsp_core::graphs::memory_alias_graph(source, registry(&dialect), &dialect)
}

fn diagram(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_lsp_core::diagram::diagram_data(source, registry(&dialect))
}

fn detect_dialect(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let filename = args.get("filename").and_then(Value::as_str);
    json!(tcl_registry::detect_dialect(source, filename, DEFAULT_DIALECT))
}

fn event_order(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let events = tcl_registry::events::EventRegistry::build();
    let ordered: Vec<Value> = events
        .order_events_for_file(source)
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let multiplicity = events.event_multiplicity(&name);
            json!({ "index": i + 1, "name": name, "multiplicity": multiplicity })
        })
        .collect();
    json!({ "events": ordered, "total": ordered.len() })
}

fn format_source(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let mut config = tcl_lsp_core::formatting::FormatterConfig::default();
    if let Some(n) = args.get("indent_size").and_then(Value::as_u64) {
        config.indent_size = usize::try_from(n).unwrap_or(config.indent_size);
    }
    if let Some(n) = args.get("max_line_length").and_then(Value::as_u64) {
        config.max_line_length = usize::try_from(n).unwrap_or(config.max_line_length);
    }
    if arg_str(args, "indent_style").eq_ignore_ascii_case("tabs") {
        config.indent_style = tcl_lsp_core::formatting::IndentStyle::Tabs;
    }
    let formatted = tcl_lsp_core::formatting::engine::format_tcl(source, &config, registry(&dialect));
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
    let disabled: std::collections::HashSet<String> =
        profile_to_disabled(profile).into_iter().map(str::to_owned).collect();
    let (optimised, opts, iterations) = optimise_source_multipass_filtered(
        source,
        registry(&dialect),
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
    let ir = tcl_compiler::lowering::lower_to_ir(source, registry(&dialect));
    let mut wasm = tcl_compiler::codegen::wasm::wasm_codegen_module(&ir, source);
    let function_count = wasm.functions.len();
    let bytes = wasm.to_bytes();
    let wat = wasm.to_wat();
    json!({ "wat": wat, "byte_length": bytes.len(), "function_count": function_count })
}

/// Run a cursor-addressed refactoring, returning its result dict or `null`.
fn refactor_at(
    args: &Value,
    f: impl FnOnce(&str, u32, &CommandRegistry, &LineIndex) -> Option<tcl_lsp_core::refactor::Refactoring>,
) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    match f(source, off, registry(&dialect), &idx) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

fn inline_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, &dialect);
    match tcl_lsp_core::refactor::inline_variable(source, off, &analysis, registry(&dialect), &idx) {
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
    match tcl_lsp_core::refactor::brace_expr(source, off, registry(&dialect)) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
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

const SRC: (&str, &str, &str) = ("source", "string", "Tcl or iRules source code");
const DIALECT: (&str, &str, &str) = ("dialect", "string", "Language dialect; auto-detected if empty");
const LINE: (&str, &str, &str) = ("line", "integer", "0-based line of the cursor");
const CHAR: (&str, &str, &str) = ("character", "integer", "0-based character of the cursor");

const TOOLS: &[ToolDef] = &[
    ToolDef { name: "call_graph", description: "Proc caller→callee graph with call sites, roots, and leaf procs.", params: &[SRC, DIALECT], required: &["source"], handler: call_graph },
    ToolDef { name: "symbol_graph", description: "Scope hierarchy with proc/variable definitions, references, and package requires.", params: &[SRC, DIALECT], required: &["source"], handler: symbol_graph },
    ToolDef { name: "dataflow_graph", description: "Taint warnings, tainted variables, and per-proc side-effect classification.", params: &[SRC, DIALECT], required: &["source"], handler: dataflow_graph },
    ToolDef { name: "def_use_chains", description: "SSA def-use chains + memory-SSA aliases; optional variable filter.", params: &[SRC, DIALECT, ("variable", "string", "Filter to a specific variable name (optional)")], required: &["source"], handler: def_use_chains },
    ToolDef { name: "memory_aliases", description: "Memory-SSA alias sets (upvar/global/variable) with reasons and locations.", params: &[SRC, DIALECT], required: &["source"], handler: memory_aliases },
    ToolDef { name: "diagram", description: "Control-flow diagram data ({events, procedures}) from the IR.", params: &[SRC, DIALECT], required: &["source"], handler: diagram },
    ToolDef { name: "detect_dialect", description: "Detect the Tcl dialect from source (+ optional filename).", params: &[SRC, ("filename", "string", "File name for extension-based detection (optional)")], required: &["source"], handler: detect_dialect },
    ToolDef { name: "event_order", description: "iRule events in canonical firing order with multiplicity.", params: &[SRC], required: &["source"], handler: event_order },
    ToolDef { name: "format_source", description: "Format Tcl/iRules source.", params: &[SRC, DIALECT, ("indent_size", "integer", "Spaces per indent (default 4)"), ("indent_style", "string", "'spaces' or 'tabs'"), ("max_line_length", "integer", "Max line length (default 120)")], required: &["source"], handler: format_source },
    ToolDef { name: "optimize", description: "Find optimisation opportunities and produce rewritten source.", params: &[SRC, DIALECT, ("profile", "string", "off | readability | standard | full | aggressive")], required: &["source"], handler: optimize },
    ToolDef { name: "compile_wasm", description: "Compile to a WebAssembly module (eval-fallback tier); returns WAT + counts.", params: &[SRC, DIALECT], required: &["source"], handler: compile_wasm },
    ToolDef { name: "inline_variable", description: "Inline a single-use variable at the cursor.", params: &[SRC, LINE, CHAR, DIALECT], required: &["source", "line", "character"], handler: inline_variable },
    ToolDef { name: "if_to_switch", description: "Convert an if/elseif chain testing one variable to a switch.", params: &[SRC, LINE, CHAR, DIALECT], required: &["source", "line", "character"], handler: if_to_switch },
    ToolDef { name: "switch_to_dict", description: "Convert a switch whose arms set one variable to a dict lookup.", params: &[SRC, LINE, CHAR, DIALECT], required: &["source", "line", "character"], handler: switch_to_dict },
    ToolDef { name: "brace_expr", description: "Brace an unbraced expr argument for safety/performance.", params: &[SRC, LINE, CHAR, DIALECT], required: &["source", "line", "character"], handler: brace_expr },
];

/// The JSON-Schema input-schema object (`{type, properties, required}`) for a
/// tool's parameters.
fn input_schema(t: &ToolDef) -> Value {
    let mut properties = Map::new();
    for (name, ty, desc) in t.params {
        properties.insert((*name).to_owned(), json!({ "type": ty, "description": desc }));
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
    TOOLS.iter().find(|t| t.name == name).map(|t| (t.handler)(args))
}
