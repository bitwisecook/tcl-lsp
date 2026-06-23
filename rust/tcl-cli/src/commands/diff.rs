//! Diff verb: compare two sources at the AST / IR / CFG layers.
//!
//! Compares two sources across three layers, byte-parity with the Python CLI
//! (modulo a residual CFG/SSA construction gap on complex scripts):
//!
//! - **AST** — segments each side (`tcl-compiler` segmenter) and serialises
//!   the command list to canonical JSON (`sort_keys`).
//! - **IR** — reproduces the `serialise_result(compiled)["ir"]` view via the
//!   `tcl-explorer` `serialise::serialise_ir`, so the diff calls it directly
//!   rather than carrying a second IR serialiser.
//! - **CFG** — `{preSsa, postSsa}` from the explorer's `serialise_result`
//!   `cfgPreSsa`/`cfgPostSsa` views.
//!
//! Every layer renders through the shared `value_to_json` →
//! `snapshot::Json::dumps_indent2` adapter and a
//! `difflib.unified_diff`-faithful diff (`tcl_cli_support::difflib`).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value, json};
use tcl_cli_support::{
    OutputTarget, combine_sources, difflib, ensure_ascii, read_input_documents,
    registry_for_dialect, write_text_output,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::segmenter::{SegmentedCommand, UnclosedDelimiter, segment_commands};
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;
use tcl_registry::snapshot::Json;

/// Unified-diff context lines. The Python verb exposes `--context` (default
/// 3); the Rust CLI surface does not, so the default is used.
const CONTEXT: usize = 3;

/// Expand / validate the `--show` layer list.
/// All three layers (`ast`, `ir`, `cfg`) are implemented, so `all` expands to
/// the full set. Duplicates are dropped, preserving order.
fn parse_layers(show: &[String]) -> anyhow::Result<Vec<&'static str>> {
    const KNOWN: [&str; 3] = ["ast", "ir", "cfg"];
    // All three layers are implemented (`cfg` rides on the engine-parity
    // CFG/SSA serialisers).
    const ALL_IMPLEMENTED: [&str; 3] = ["ast", "ir", "cfg"];
    let mut selected: Vec<&'static str> = Vec::new();
    for raw in show {
        for token in raw.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let names: &[&str] = match token {
                "all" => &ALL_IMPLEMENTED,
                "ast" => &["ast"],
                "ir" => &["ir"],
                "cfg" => &["cfg"],
                other => anyhow::bail!("unknown diff layer: {other} (expected ast,ir,cfg,all)"),
            };
            for name in names {
                let canonical = KNOWN.iter().find(|n| *n == name).copied().expect("known");
                if !selected.contains(&canonical) {
                    selected.push(canonical);
                }
            }
        }
    }
    if selected.is_empty() {
        anyhow::bail!("no diff layers selected; pass --show ast,ir,cfg or --show all");
    }
    Ok(selected)
}

/// `range_dict` over a command span: inclusive→exclusive end (`+1`), which is
/// exactly the segmenter's exclusive `span.end()`. The whole-command range is
/// never a single braced word, so `widen_for_highlight` is a no-op here.
fn range_json(span: tcl_lexer::Span, line_index: &LineIndex, source: &str) -> Json {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    let mut m = BTreeMap::new();
    m.insert("startLine".to_owned(), Json::Int(i64::from(start.line)));
    m.insert("startCol".to_owned(), Json::Int(i64::from(start.character)));
    m.insert("startOffset".to_owned(), Json::Int(i64::from(span.start())));
    m.insert("endLine".to_owned(), Json::Int(i64::from(end.line)));
    m.insert("endCol".to_owned(), Json::Int(i64::from(end.character)));
    m.insert("endOffset".to_owned(), Json::Int(i64::from(span.end())));
    Json::Object(m)
}

/// `partial_delimiter.name.lower()` or `""`.
fn partial_delimiter_str(cmd: &SegmentedCommand) -> &'static str {
    match cmd.partial_delimiter {
        Some(UnclosedDelimiter::Brace) => "brace",
        Some(UnclosedDelimiter::Bracket) => "bracket",
        Some(UnclosedDelimiter::Quote) => "quote",
        None => "",
    }
}

/// Resolve the command's subcommand name:
/// when the head has registered subcommands and the first arg names one.
fn resolve_subcommand<'a>(cmd: &'a SegmentedCommand, registry: &CommandRegistry) -> &'a str {
    if cmd.texts.len() < 2 {
        return "";
    }
    let Some(spec) = registry.get(cmd.name()) else {
        return "";
    };
    let candidate = cmd.texts[1].as_str();
    if spec.subcommands.iter().any(|s| s.name == candidate) {
        candidate
    } else {
        ""
    }
}

/// The `ast` layer payload (`{"commands": [...]}`), as canonical
/// `json.dumps(indent=2, sort_keys=True)` text.
fn serialise_command_ast(
    source: &str,
    registry: &CommandRegistry,
    line_index: &LineIndex,
) -> String {
    let commands: Vec<Json> = segment_commands(source)
        .iter()
        .map(|cmd| {
            let mut m = BTreeMap::new();
            m.insert("name".to_owned(), Json::s(cmd.name()));
            m.insert(
                "subcommand".to_owned(),
                Json::s(resolve_subcommand(cmd, registry)),
            );
            m.insert(
                "args".to_owned(),
                Json::Array(cmd.args().iter().map(|a| Json::s(a.clone())).collect()),
            );
            m.insert("isPartial".to_owned(), Json::Bool(cmd.is_partial));
            m.insert(
                "partialDelimiter".to_owned(),
                Json::s(partial_delimiter_str(cmd)),
            );
            m.insert(
                "precedingComment".to_owned(),
                Json::s(cmd.preceding_comment.clone().unwrap_or_default()),
            );
            m.insert(
                "expandWord".to_owned(),
                Json::Array(
                    cmd.expand_word
                        .as_ref()
                        .map(|v| v.iter().map(|&b| Json::Bool(b)).collect())
                        .unwrap_or_default(),
                ),
            );
            m.insert("range".to_owned(), range_json(cmd.span, line_index, source));
            Json::Object(m)
        })
        .collect();
    let mut root = BTreeMap::new();
    root.insert("commands".to_owned(), Json::Array(commands));
    Json::Object(root).dumps_indent2()
}

/// Compute one layer's diff: equal flag +
/// the raw `unified_diff` line list (each line keeps its terminator).
fn compute_layer_diff(
    layer: &str,
    left_text: &str,
    right_text: &str,
    left_name: &str,
    right_name: &str,
) -> (bool, Vec<String>) {
    if left_text == right_text {
        return (true, Vec::new());
    }
    let left_owned = format!("{left_text}\n");
    let right_owned = format!("{right_text}\n");
    let la = difflib::splitlines_keepends(&left_owned);
    let lb = difflib::splitlines_keepends(&right_owned);
    let from = format!("{layer}:{left_name}");
    let to = format!("{layer}:{right_name}");
    let lines = difflib::unified_diff(&la, &lb, &from, &to, CONTEXT);
    (false, lines)
}

/// Build one layer's canonical-JSON payload text for `src`.
fn layer_payload(
    layer: &str,
    src: &str,
    dialect: &str,
    registry: &CommandRegistry,
    line_index: &LineIndex,
) -> anyhow::Result<String> {
    match layer {
        "ast" => Ok(serialise_command_ast(src, registry, line_index)),
        "ir" => {
            // Python's diff reads `serialise_result(compiled)["ir"]`; the
            // native `tcl-explorer` serialiser reproduces that view, so reuse
            // it (through the shared `value_to_json` adapter) rather than
            // carrying a second IR serialiser.
            let cu = CompilationUnit::build_for(src, registry, false);
            let ir = tcl_explorer::serialise::serialise_ir(&cu.ir_module, line_index, src);
            Ok(value_to_json(&ir).dumps_indent2())
        }
        "cfg" => Ok(cfg_layer_payload(src, dialect)),
        other => anyhow::bail!("unknown diff layer: {other} (expected ast,ir,cfg)"),
    }
}

/// Build the `cfg` diff layer payload: `{preSsa, postSsa}` from the explorer's
/// CFG/SSA serialisers (which match the Python CLI), rendered through the
/// shared sort-keys indent-2 adapter the AST/IR layers use. The two views are
/// read off `serialise_result`.
///
/// `analysis.deadStores` inside `postSsa` is the liveness-based set,
/// byte-identical wherever the SSA matches. A residual SSA-block construction
/// gap on complex scripts can still diverge (and cascade into
/// `inferredTypes`/`deadStores` there).
fn cfg_layer_payload(src: &str, dialect: &str) -> String {
    let result = tcl_explorer::run_pipeline(src, dialect);
    let serialised = tcl_explorer::serialise_result(&result);
    let mut payload: BTreeMap<String, Json> = BTreeMap::new();
    for (out_key, view_key) in [("preSsa", "cfgPreSsa"), ("postSsa", "cfgPostSsa")] {
        let view = serialised
            .get(view_key)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        payload.insert(out_key.to_owned(), value_to_json(&view));
    }
    Json::Object(payload).dumps_indent2()
}

/// Convert a `serde_json::Value` (the explorer serialisers' output) into the
/// snapshot [`Json`] so it renders through the shared `dumps_indent2`
/// (`json.dumps(indent=2, sort_keys=True)`) adapter the AST/IR layers use. The
/// CFG payload is integer-only (offsets / versions / counts), so a JSON number
/// is always an `i64`.
fn value_to_json(value: &serde_json::Value) -> Json {
    match value {
        serde_json::Value::Null => Json::Null,
        serde_json::Value::Bool(b) => Json::Bool(*b),
        // CFG payloads are integer-only; a non-`i64` number would be a bug, so
        // surface it as a string rather than silently truncating through `f64`.
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Json::Str(n.to_string()), Json::Int),
        serde_json::Value::String(s) => Json::Str(s.clone()),
        serde_json::Value::Array(items) => Json::Array(items.iter().map(value_to_json).collect()),
        serde_json::Value::Object(map) => Json::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect(),
        ),
    }
}

/// `tcl diff` — AST/IR/CFG structural diff.
#[allow(clippy::too_many_arguments)]
pub fn run_diff(
    left: Option<&Path>,
    right: Option<&Path>,
    left_source: Option<&str>,
    right_source: Option<&str>,
    dialect: &str,
    show: &[String],
    json_out: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let layers = parse_layers(show)?;

    let Some(left) = left else {
        anyhow::bail!("diff requires a left input");
    };
    let Some(right) = right else {
        anyhow::bail!("diff requires a right input");
    };
    let left_name = left.to_string_lossy().into_owned();
    let right_name = right.to_string_lossy().into_owned();

    let left_inline: Vec<String> = left_source.map(|s| vec![s.to_owned()]).unwrap_or_default();
    let right_inline: Vec<String> = right_source.map(|s| vec![s.to_owned()]).unwrap_or_default();
    let left_docs = read_input_documents(&[left.to_path_buf()], &left_inline, true)?;
    let right_docs = read_input_documents(&[right.to_path_buf()], &right_inline, true)?;
    let left_src = combine_sources(&left_docs);
    let right_src = combine_sources(&right_docs);

    let registry = registry_for_dialect(dialect);
    let left_index = LineIndex::new(&left_src);
    let right_index = LineIndex::new(&right_src);

    let mut results: Vec<(String, bool, Vec<String>)> = Vec::new();
    for layer in &layers {
        let lp = layer_payload(layer, &left_src, dialect, registry, &left_index)?;
        let rp = layer_payload(layer, &right_src, dialect, registry, &right_index)?;
        let (equal, lines) = compute_layer_diff(layer, &lp, &rp, &left_name, &right_name);
        results.push(((*layer).to_owned(), equal, lines));
    }

    let target = OutputTarget::from_arg(output);
    write_diff_result(
        &target,
        json_out,
        dialect,
        &left_name,
        &right_name,
        &left_docs,
        &right_docs,
        &results,
    )
}

/// Emit the multi-layer diff result as JSON or unified-diff text, returning the
/// exit code (1 when any layer differs).
#[allow(clippy::too_many_arguments)]
fn write_diff_result(
    target: &OutputTarget,
    json_out: bool,
    dialect: &str,
    left_name: &str,
    right_name: &str,
    left_docs: &[tcl_cli_support::InputDocument],
    right_docs: &[tcl_cli_support::InputDocument],
    results: &[(String, bool, Vec<String>)],
) -> anyhow::Result<u8> {
    let has_differences = results.iter().any(|(_, equal, _)| !equal);

    if json_out {
        let mut layer_obj = Map::new();
        for (layer, equal, lines) in results {
            let stripped: Vec<String> = lines
                .iter()
                .map(|l| l.trim_end_matches('\n').to_owned())
                .collect();
            layer_obj.insert(
                layer.clone(),
                json!({
                    "equal": equal,
                    "diff": stripped,
                    "diffLineCount": lines.len(),
                }),
            );
        }
        let payload = json!({
            "equal": !has_differences,
            "dialect": dialect,
            "layers": Value::Object(layer_obj),
            "leftInput": left_name,
            "rightInput": right_name,
            "leftDocuments": left_docs.iter().map(|d| d.label.clone()).collect::<Vec<_>>(),
            "rightDocuments": right_docs.iter().map(|d| d.label.clone()).collect::<Vec<_>>(),
        });
        write_text_output(
            target,
            &ensure_ascii(&serde_json::to_string_pretty(&payload)?),
        )?;
        return Ok(u8::from(has_differences));
    }

    let mut chunks: Vec<String> = Vec::new();
    for (layer, equal, lines) in results {
        if *equal {
            chunks.push(format!("{layer}: identical\n"));
            continue;
        }
        chunks.push(format!("=== {layer} diff ===\n"));
        let stripped: Vec<&str> = lines.iter().map(|l| l.trim_end_matches('\n')).collect();
        if stripped.is_empty() {
            chunks.push(
                "(differences detected, but no unified diff lines were produced)\n".to_owned(),
            );
        } else {
            for line in stripped {
                chunks.push(format!("{line}\n"));
            }
        }
    }
    let text = chunks.concat();
    write_text_output(target, text.trim_end_matches('\n'))?;
    Ok(u8::from(has_differences))
}
