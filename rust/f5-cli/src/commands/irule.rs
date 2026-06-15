//! The `f5 irule` verb group — iRules-specific sub-subcommands.
//!
//! Port of `tooling/f5/verbs/irule.py`. The sub-subcommands split into two
//! groups by which engine they reuse:
//!
//! - **Ported, byte-parity** — `event-order` (firing-order event metadata from
//!   [`tcl_registry::events`]), `extract` (per-rule bodies from the
//!   [`tcl_bigip`] model), and `format` / `minify` (the [`tcl_lsp_core`]
//!   formatter / minifier engines, driven with the `f5-irules` dialect exactly
//!   as the `tcl` CLI drives them).
//! - **Deferred** — `event-info` (its `validCommands` cross-product needs the
//!   full f5-irules command corpus, which the Rust registry does not yet carry),
//!   `lint` / `context` (the iRule analyser), and `trace` / `pgo` (the compiler
//!   lowering / CFG / VM). Each parses its args (so `--help` works) but its
//!   handler prints a clear "not yet ported" error and exits 2.

use std::path::{Path, PathBuf};

use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_io::paths::read_path;
use tcl_cli_support::registry_for_dialect;
use tcl_lsp_core::formatting::{FormatterConfig, IndentStyle, formatting_with};
use tcl_lsp_core::minify::{minify_tcl, minify_tcl_aggressive, minify_tcl_compact};

use crate::cli::{IruleColourArgs, IruleCommand, IruleFormatterArgs, IruleInputArgs};

/// A single iRule body resolved from a CLI input (port of `IruleInput`).
struct IruleInput {
    /// Display label, used for diagnostics and synthesising output names.
    label: String,
    /// The iRule source body.
    source: String,
    /// BIG-IP full path when extracted from a config; `None` for standalone.
    rule_full_path: Option<String>,
}

const IRULE_SUFFIXES: &[&str] = &["tcl", "irul", "irule"];

fn suffix_lower(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Read each path's rules / synthesise a single-rule body, plus inline
/// `--source` snippets. Mirrors `load_irule_inputs`.
fn load_irule_inputs(
    paths: &[String],
    inline_sources: &[String],
) -> Result<Vec<IruleInput>, String> {
    let mut inputs: Vec<IruleInput> = Vec::new();

    for (index, source_text) in inline_sources.iter().enumerate() {
        let n = index + 1;
        inputs.push(IruleInput {
            label: format!("<inline:{n}>"),
            source: source_text.clone(),
            rule_full_path: None,
        });
    }

    let opts = tcl_bigip_io::PassphraseOptions::default();
    for path_str in paths {
        let suffix = if path_str == "-" {
            String::new()
        } else {
            suffix_lower(path_str)
        };
        let (_origin, text) = read_path(path_str, false, &opts).map_err(|e| e.to_string())?;
        let label = if path_str == "-" {
            "<stdin>".to_owned()
        } else {
            path_str.clone()
        };

        // Standalone iRule file: never parse as a bigip.conf.
        if IRULE_SUFFIXES.contains(&suffix.as_str()) {
            inputs.push(IruleInput {
                label,
                source: text,
                rule_full_path: None,
            });
            continue;
        }

        // bigip.conf / SCF / unknown / stdin text — sniff for `ltm rule`.
        let cfg = parse_bigip_conf(&text, "Common");
        let rules = collect_rules(&cfg);
        if rules.is_empty() {
            inputs.push(IruleInput {
                label,
                source: text,
                rule_full_path: None,
            });
        } else {
            for (rule_path, src) in rules {
                let label = format!("{label}::{rule_path}");
                inputs.push(IruleInput {
                    label,
                    source: src,
                    rule_full_path: Some(rule_path),
                });
            }
        }
    }

    Ok(inputs)
}

/// Collect `(full_path, source)` for every `ltm rule` in a parsed config, in
/// source order (mirrors `BigipConfig.rules` iteration).
fn collect_rules(cfg: &tcl_bigip::parser::BigipConfig) -> Vec<(String, String)> {
    cfg.objects
        .iter()
        .filter(|p| p.table_name == "rules")
        .filter_map(|p| match &p.object {
            ModelObject::Rule(r) => Some((p.full_path.clone(), r.source.clone())),
            _ => None,
        })
        .collect()
}

/// Resolve iRule inputs with CLI error handling (mirrors
/// `_resolve_irule_inputs`): prints the error and returns the exit code on
/// failure.
fn resolve_irule_inputs(input: &IruleInputArgs) -> Result<Vec<IruleInput>, u8> {
    if input.paths.is_empty() && input.source.is_empty() {
        eprintln!("error: no input provided; pass files, --source, or `-` for stdin");
        return Err(2);
    }
    match load_irule_inputs(&input.paths, &input.source) {
        Ok(inputs) => Ok(inputs),
        Err(msg) => {
            eprintln!("error: {msg}");
            Err(2)
        }
    }
}

fn flatten_rule_path(rule_full_path: Option<&str>, fallback_label: &str) -> String {
    if let Some(p) = rule_full_path {
        return p.trim_start_matches('/').replace('/', "__");
    }
    let mut stem = Path::new(fallback_label)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if stem.is_empty() || stem == "<stdin>" || stem == "-" {
        "irule".clone_into(&mut stem);
    }
    let known = [".tcl", ".irul", ".irule", ".conf", ".scf", ".ucs"];
    if known.iter().any(|s| stem.ends_with(s))
        && let Some(base) = Path::new(&stem).file_stem()
    {
        stem = base.to_string_lossy().into_owned();
    }
    stem.replace('/', "__")
}

/// Decide whether `output` should be treated as a directory (port of
/// `_is_directory_target`).
fn is_directory_target(output: &str) -> bool {
    if output == "-" {
        return false;
    }
    Path::new(output).is_dir() || output.ends_with('/') || output.ends_with('\\')
}

/// Common output dispatcher for irule format / minify (port of
/// `_write_iRule_outputs`).
fn write_irule_outputs(
    output: &str,
    inputs: &[IruleInput],
    transformed: &[String],
    file_extension: &str,
    colour: Option<&IruleColourArgs>,
) -> Result<u8, String> {
    if is_directory_target(output) {
        let out_dir = Path::new(output);
        std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
        for (entry, text) in inputs.iter().zip(transformed.iter()) {
            let stem = flatten_rule_path(entry.rule_full_path.as_deref(), &entry.label);
            let target = out_dir.join(format!("{stem}{file_extension}"));
            let body = if text.ends_with('\n') {
                text.clone()
            } else {
                format!("{text}\n")
            };
            std::fs::write(&target, body).map_err(|e| e.to_string())?;
        }
        eprintln!("wrote {} iRule(s) to {}", transformed.len(), output);
        return Ok(0);
    }

    // Single-file / stdout: emit the concatenation.
    let text = if transformed.len() <= 1 {
        transformed.first().cloned().unwrap_or_default()
    } else {
        let mut chunks: Vec<String> = Vec::with_capacity(transformed.len());
        for (entry, body) in inputs.iter().zip(transformed.iter()) {
            let label = entry
                .rule_full_path
                .clone()
                .unwrap_or_else(|| entry.label.clone());
            chunks.push(format!("# ===== {label} =====\n{}\n", body.trim_end()));
        }
        chunks.join("\n")
    };
    write_highlighted(output, &text, colour)?;
    Ok(0)
}

/// Resolve the effective tab width (port of `_resolve_tab_width`).
fn resolve_tab_width(colour: Option<&IruleColourArgs>) -> usize {
    colour.and_then(|c| c.tabs).unwrap_or(4)
}

/// Whether ANSI colour should be applied (port of `_resolve_use_colour`).
fn resolve_use_colour(output: &str, colour: Option<&IruleColourArgs>) -> bool {
    use tcl_cli_support::OutputTarget;
    let (force, no) = colour.map_or((false, false), |c| (c.colour, c.no_colour));
    let target = if output == "-" {
        OutputTarget::Stdout
    } else {
        OutputTarget::File(PathBuf::from(output))
    };
    tcl_cli_support::resolve_use_colour(force, no, &target)
}

/// Write `text` with optional highlighting / tab expansion to `output`
/// (mirrors `_write_highlighted_output`).
fn write_highlighted(
    output: &str,
    text: &str,
    colour: Option<&IruleColourArgs>,
) -> Result<(), String> {
    use tcl_cli_support::OutputTarget;
    let use_colour = resolve_use_colour(output, colour);
    let tab_width = resolve_tab_width(colour);
    let target = if output == "-" {
        OutputTarget::Stdout
    } else {
        OutputTarget::File(PathBuf::from(output))
    };
    tcl_cli_support::write_highlighted_output(&target, text, use_colour, tab_width, "f5-irules")
        .map_err(|e| e.to_string())
}

/// Write plain text to `output` (mirrors `_write_text_output`).
fn write_text(output: &str, text: &str) -> Result<(), String> {
    use tcl_cli_support::OutputTarget;
    let target = if output == "-" {
        OutputTarget::Stdout
    } else {
        OutputTarget::File(PathBuf::from(output))
    };
    tcl_cli_support::write_text_output(&target, text).map_err(|e| e.to_string())
}

/// Dispatch the `irule` verb group.
///
/// Returns `anyhow::Result<u8>` to slot into the top-level `dispatch` match
/// uniformly; the handlers print their own errors and resolve to an exit code,
/// so this never returns `Err`.
#[allow(clippy::unnecessary_wraps)]
pub fn run_irule(action: &IruleCommand) -> anyhow::Result<u8> {
    let rc = match action {
        IruleCommand::EventOrder { input, json } => run_event_order(input, *json),
        IruleCommand::Extract { paths, output } => run_extract(paths, output),
        IruleCommand::Format {
            input,
            colour,
            formatter,
        } => run_format(input, colour, formatter),
        IruleCommand::Minify {
            input,
            compact,
            symbol_map,
            aggressive,
            isolated,
            colour,
        } => run_minify(
            input,
            *compact,
            symbol_map.as_deref(),
            *aggressive,
            *isolated,
            colour,
        ),
        // Deferred subs: each needs an engine the Rust port does not yet carry.
        IruleCommand::EventInfo { .. } => Err(deferred(
            "event-info",
            "full f5-irules command-registry (validCommands cross-product)",
        )),
        IruleCommand::Lint { .. } => Err(deferred("lint", "analyser")),
        IruleCommand::Context { .. } => Err(deferred("context", "analyser")),
        IruleCommand::Trace { .. } => Err(deferred("trace", "compiler-VM")),
        IruleCommand::Pgo { .. } => Err(deferred("pgo", "compiler-VM")),
    };
    // Both the success path and the printed-error path resolve to a process
    // exit code (mirroring the Python handlers, which `return` an int either way).
    Ok(rc.unwrap_or_else(|code| code))
}

/// Print the standard "not yet ported" message for a deferred sub and return
/// the exit code (2).
fn deferred(sub: &str, engine: &str) -> u8 {
    eprintln!("error: f5 irule {sub} is not yet ported (requires the {engine} engine)");
    2
}

// ---------------------------------------------------------------------------
// event-order
// ---------------------------------------------------------------------------

/// Scan `when EVENT` blocks and return events in canonical firing order
/// (mirrors `order_events_for_file` + `order_events`).
fn order_events_for_file(
    source: &str,
    events: &tcl_registry::events::EventRegistry,
) -> Vec<String> {
    // `\bwhen\s+([A-Z_][A-Z0-9_]*)` — the Python `scan_file_events` regex.
    let mut found: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while let Some(rel) = source[i..].find("when") {
        let start = i + rel;
        // `\b` before "when": preceding char must not be a word char.
        let prev_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = start + 4;
        if prev_ok && after < bytes.len() && bytes[after].is_ascii_whitespace() {
            // skip whitespace
            let mut j = after;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            // first char [A-Z_]
            if j < bytes.len() && (bytes[j] == b'_' || bytes[j].is_ascii_uppercase()) {
                let name_start = j;
                while j < bytes.len()
                    && (bytes[j] == b'_'
                        || bytes[j].is_ascii_uppercase()
                        || bytes[j].is_ascii_digit())
                {
                    j += 1;
                }
                found.insert(source[name_start..j].to_owned());
            }
        }
        i = start + 4;
    }

    // order_events: known events by master-order index, unknown sorted after.
    let order: Vec<&str> = events.master_order().iter().map(|o| o.event).collect();
    let mut known: Vec<(usize, String)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for evt in &found {
        if let Some(idx) = order.iter().position(|e| *e == evt) {
            known.push((idx, evt.clone()));
        } else {
            unknown.push(evt.clone());
        }
    }
    known.sort();
    unknown.sort();
    known.into_iter().map(|(_, e)| e).chain(unknown).collect()
}

fn is_word_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// Multiplicity category for a single event (mirrors `event_multiplicity`).
fn event_multiplicity(name: &str, events: &tcl_registry::events::EventRegistry) -> &'static str {
    if name == "RULE_INIT" {
        "init"
    } else if events.is_once_per_connection(name) {
        "once_per_connection"
    } else if events.is_per_request(name) {
        "per_request"
    } else {
        "unknown"
    }
}

fn run_event_order(input: &IruleInputArgs, json: bool) -> Result<u8, u8> {
    let loaded = resolve_irule_inputs(input)?;
    let events = tcl_registry::events::EventRegistry::build();

    let combined: String = loaded
        .iter()
        .map(|e| e.source.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n\n");

    let ordered = order_events_for_file(&combined, &events);
    let items: Vec<(usize, String, &str)> = ordered
        .iter()
        .enumerate()
        .map(|(i, name)| (i + 1, name.clone(), event_multiplicity(name, &events)))
        .collect();

    if json {
        use std::fmt::Write as _;
        // `json.dumps(payload, indent=2)` — insertion-ordered keys, no sort.
        let mut out = String::from("{\n");
        let _ = writeln!(out, "  \"count\": {},", items.len());
        out.push_str("  \"dialect\": ");
        out.push_str(&json_string(&input.dialect));
        out.push_str(",\n  \"events\": ");
        if items.is_empty() {
            out.push_str("[]");
        } else {
            out.push_str("[\n");
            for (i, (index, name, mult)) in items.iter().enumerate() {
                out.push_str("    {\n");
                let _ = writeln!(out, "      \"index\": {index},");
                out.push_str("      \"name\": ");
                out.push_str(&json_string(name));
                out.push_str(",\n      \"multiplicity\": ");
                out.push_str(&json_string(mult));
                out.push_str("\n    }");
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ]");
        }
        out.push_str("\n}");
        write_text(&input.output, &out).map_err(|_| 1u8)?;
        return Ok(0);
    }

    let mut lines = vec![format!("event order: {} event(s)", items.len())];
    for (index, name, mult) in &items {
        lines.push(format!("  {index}. {name} ({mult})"));
    }
    write_text(&input.output, &lines.join("\n")).map_err(|_| 1u8)?;
    Ok(0)
}

// ---------------------------------------------------------------------------
// extract
// ---------------------------------------------------------------------------

fn run_extract(paths: &[String], output: &Path) -> Result<u8, u8> {
    if paths.is_empty() {
        eprintln!("error: no input provided; pass bigip.conf / SCF / UCS files");
        return Err(2);
    }
    let standalone = ["tcl", "irul", "irule"];
    let bad: Vec<&String> = paths
        .iter()
        .filter(|p| *p != "-" && standalone.contains(&suffix_lower(p).as_str()))
        .collect();
    if !bad.is_empty() {
        let joined = bad
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "error: extract only accepts bigip.conf / SCF / UCS; refusing standalone \
             iRule file(s): {joined}"
        );
        return Err(2);
    }

    let opts = tcl_bigip_io::PassphraseOptions::default();
    let mut all_rules: Vec<Vec<(String, String)>> = Vec::new();
    for p in paths {
        let (_origin, text) = match read_path(p, false, &opts) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: {e}");
                return Err(2);
            }
        };
        let cfg = parse_bigip_conf(&text, "Common");
        all_rules.push(collect_rules(&cfg));
    }

    if let Err(e) = std::fs::create_dir_all(output) {
        eprintln!("error: {e}");
        return Err(2);
    }

    let mut written = 0;
    for rules in &all_rules {
        for (rule_path, source) in rules {
            let flat = rule_path.trim_start_matches('/').replace('/', "__");
            let target = output.join(format!("{flat}.tcl"));
            if let Err(e) = std::fs::write(&target, format!("{source}\n")) {
                eprintln!("error: {e}");
                return Err(2);
            }
            written += 1;
        }
    }

    eprintln!("extracted {written} iRule(s) to {}", output.display());
    Ok(0)
}

// ---------------------------------------------------------------------------
// format / minify
// ---------------------------------------------------------------------------

fn build_formatter_config(formatter: &IruleFormatterArgs) -> FormatterConfig {
    let mut config = FormatterConfig::default();
    if let Some(size) = formatter.indent_size {
        config.indent_size = size;
    }
    if let Some(style) = formatter.indent_style.as_deref() {
        config.indent_style = if style == "tabs" {
            IndentStyle::Tabs
        } else {
            IndentStyle::Spaces
        };
    }
    if let Some(max) = formatter.max_line_length {
        config.max_line_length = max;
    }
    if let Some(goal) = formatter.goal_line_length {
        config.goal_line_length = goal;
    }
    if formatter.expand_bodies {
        config.expand_single_line_bodies = true;
    }
    if formatter.no_semicolons {
        config.replace_semicolons_with_newlines = true;
    }
    if formatter.keep_semicolons {
        config.replace_semicolons_with_newlines = false;
    }
    config
}

fn run_format(
    input: &IruleInputArgs,
    colour: &IruleColourArgs,
    formatter: &IruleFormatterArgs,
) -> Result<u8, u8> {
    let loaded = resolve_irule_inputs(input)?;
    let registry = registry_for_dialect(&input.dialect);
    let config = build_formatter_config(formatter);

    let rendered: Vec<String> = loaded
        .iter()
        .map(|entry| {
            let edits = formatting_with(&entry.source, &config, registry);
            edits
                .into_iter()
                .next()
                .map_or_else(|| entry.source.clone(), |edit| edit.new_text)
        })
        .collect();

    write_irule_outputs(&input.output, &loaded, &rendered, ".irule", Some(colour)).map_err(|_| 2u8)
}

fn run_minify(
    input: &IruleInputArgs,
    compact: bool,
    symbol_map: Option<&Path>,
    aggressive: bool,
    isolated: bool,
    colour: &IruleColourArgs,
) -> Result<u8, u8> {
    let loaded = resolve_irule_inputs(input)?;
    let registry = registry_for_dialect(&input.dialect);

    let mut minified: Vec<String> = Vec::with_capacity(loaded.len());
    let mut symbol_maps: Vec<String> = Vec::new();
    if aggressive {
        for entry in &loaded {
            let result = minify_tcl_aggressive(&entry.source, &input.dialect, isolated, registry);
            minified.push(result.source);
            symbol_maps.push(result.symbol_map.format());
        }
    } else if compact {
        for entry in &loaded {
            let (text, sm) = minify_tcl_compact(&entry.source, &input.dialect, isolated, registry);
            minified.push(text);
            symbol_maps.push(sm.format());
        }
    } else {
        for entry in &loaded {
            minified.push(minify_tcl(&entry.source, &input.dialect, registry));
        }
    }

    let rc = write_irule_outputs(&input.output, &loaded, &minified, ".irule", Some(colour))
        .map_err(|_| 2u8)?;
    if rc != 0 {
        return Ok(rc);
    }

    if let Some(map_path) = symbol_map
        && !symbol_maps.is_empty()
    {
        let mut blocks: Vec<String> = Vec::new();
        for (entry, m) in loaded.iter().zip(symbol_maps.iter()) {
            if m.is_empty() {
                continue;
            }
            let label = entry
                .rule_full_path
                .clone()
                .unwrap_or_else(|| entry.label.clone());
            blocks.push(format!("# {label}\n{m}").trim_end().to_owned());
        }
        let joined = blocks.join("\n\n");
        let text = if joined.is_empty() {
            String::new()
        } else {
            format!("{joined}\n")
        };
        let map_str = map_path.to_string_lossy();
        write_text(&map_str, &text).map_err(|_| 2u8)?;
    }
    Ok(0)
}

/// `json.dumps`-compatible quoted string (reuses [`tcl_bigip::jsonfmt`]).
fn json_string(s: &str) -> String {
    tcl_bigip::jsonfmt::json_string(s)
}
