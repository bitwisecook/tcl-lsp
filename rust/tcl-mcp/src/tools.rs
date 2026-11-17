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
use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_dialect::DialectProfile;
#[cfg(test)]
use tcl_lexer::LexerConfig;
use tcl_lexer::{LineIndex, SourceMap, Span, Utf16Col};
use tcl_lsp_core::config_ini;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::diagnostic_policy::{
    Directives, Finding, Policy, PolicyBuilder, PolicyLayer, Report,
};
use tcl_lsp_core::diagnostic_report::{
    DocumentSource, SourcePass, StandaloneDocument, StandaloneFindings,
    document_report_with_analysis, standalone_findings,
};
use tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH;
use tcl_registry::CommandRegistry;

mod diagnostics;
mod docstrings;
mod editor_queries;
mod refactorings;
mod source_structure;
mod tool_table;
mod transforms;

#[cfg(test)]
use diagnostics::{analyze, code_actions, find_legacy};
#[cfg(test)]
use diagnostics::{
    analyze_with, code_actions_from_analysis, code_actions_with, find_legacy_with, review_with,
    validate_with,
};
#[cfg(test)]
use docstrings::{generate_docstring, read_proc_docs, update_docstrings};
#[cfg(test)]
use refactorings::datagroup_advice;
#[cfg(test)]
use refactorings::{brace_expr, extract_datagroup, refactor};
#[cfg(test)]
use source_structure::{detect_events, event_order_list, memory_aliases};
#[cfg(test)]
use tool_table::DIALECT_PARAM;
use tool_table::{TOOLS, dialect_names, input_schema};
#[cfg(test)]
use transforms::{format_source, optimize_with};

const IRULES_DIALECT: &str = "f5-irules";

/// The release an MCP session starts on: the newest stable Tcl, so a call that
/// names no dialect sees every current command. It is separate from the
/// editors' and the language server's `DEFAULT_ENVIRONMENT_ID`.
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
            // `detect_dialect`'s default must be `&'static`; the session
            // name's canonical environment id supplies one, else the
            // built-in default. The session always holds a canonical id
            // (`set_dialect` stores one), so the lookup returns it unchanged.
            let default =
                crate::environment::canonical_id_for_dialect(&session).unwrap_or(DEFAULT_DIALECT);
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
    let requested = arg_str(args, "dialect");
    // Aliases (`irules`, `tclspec`, …) resolve here, so the session holds a
    // canonical environment id.
    //
    // The validator is the one `Environment::resolve`, matching the LSP's
    // `setDialect` — so this accepts every *declared* name (canonical ids,
    // aliases, and the contributed editor identities), and still rejects an
    // unknown spelling. The advertised `enum` and the `valid_dialects` list
    // are the selectable canonical ids.
    let Some(profile) = crate::environment::known_profile_for_dialect(requested) else {
        return json!({
            "error": format!(
                "Unknown dialect '{requested}'; valid surface: {}",
                dialect_names().join(", ")
            ),
            "valid_dialects": dialect_names(),
        });
    };
    let resolved = profile.name.to_owned();
    let mut guard = session_dialect().lock().expect("session dialect lock");
    let previous = std::mem::replace(&mut *guard, resolved.clone());
    let changed = previous != resolved;
    json!({
        "dialect": resolved,
        "display_name": profile.display_name,
        "requested": requested,
        "previous": previous,
        "changed": changed,
    })
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

/// Registry for dialect-taking tools, including shipped `.tclspec` loadables.
/// EDA vendor commands come from bundled packs selected by the dialect's
/// ambient package set (`docs/design/registry/spec-packs.md`).
fn registry(dialect: &str) -> std::sync::Arc<CommandRegistry> {
    tcl_spectcl::bundled::registry_for_dialect(dialect)
}

/// The byte offset of `(line, character)` (UTF-16 column) in `source`.
fn cursor(source: &str, line: u32, character: u32) -> (LineIndex, u32) {
    let idx = LineIndex::new(source);
    let off = idx.offset_at_utf16(line, Utf16Col::new(character), source);
    (idx, off)
}

/// The configuration layers one tool call resolves its policy under
/// (`docs/design/compiler/diagnostic-policy.md` § Configuration): the user's
/// global `config.ini`, and the call's `disable` / `enable` arguments as its
/// invocation layer in the editor layer's slot. An MCP `source` string has no
/// path, so there is no project layer at all. The `optimize` tool's
/// `profile` is not a layer but the request's own ([`optimize_with`]).
struct PolicyInputs {
    global: Value,
    invocation: Value,
}

impl PolicyInputs {
    /// The layers for a call with `args`, over `section` (`diagnostics` for
    /// the diagnostics tools, `optimiser` for `optimize`).
    fn for_call(args: &Value, section: &str) -> Self {
        Self {
            global: config_ini::global_layer(),
            invocation: invocation_layer(args, section),
        }
    }

    /// A builder with the layers applied, lowest first.
    ///
    /// `[features]` configures the language server's features
    /// (`docs/design/contracts/xdg-config.md` § `[features]`); a tool
    /// reports whatever it is asked to, so neither whole-document gate
    /// reaches it: reporting is on, and nothing is excluded.
    fn builder(&self) -> PolicyBuilder {
        PolicyBuilder::new()
            .reporting(true)
            .layer(PolicyLayer::Global, &self.global)
            .layer(PolicyLayer::Invocation, &self.invocation)
    }
}

/// The `disable` / `enable` arguments (comma-separated codes) as one settings
/// layer over `section`. A later `enable` turns a code back on, which is what
/// reaches a default-off code such as W242.
fn invocation_layer(args: &Value, section: &str) -> Value {
    let mut codes = Map::new();
    for (key, on) in [("disable", false), ("enable", true)] {
        for code in arg_str(args, key).split(',') {
            let code = code.trim();
            if !code.is_empty() {
                codes.insert(code.to_ascii_uppercase(), Value::Bool(on));
            }
        }
    }
    let mut layer = Map::new();
    layer.insert(section.to_owned(), Value::Object(codes));
    Value::Object(layer)
}

/// One source analysed as `tcl diag` analyses a document — the analyser and
/// the compiler checks over one unit (`standalone_findings`) — and the
/// policy its findings are decided under.
struct Analysed {
    /// The call's `source`, as the client sent it — what W118 reads.
    source: String,
    /// Its analysis form (a lone `\r` rewritten to `\n`, the same length):
    /// what every producer reads, and the text the rewrites and the code
    /// actions are computed over, as `tcl opt` and the editor compute them.
    analysis_text: String,
    /// The source's dialect.
    dialect: &'static DialectProfile,
    /// The analysis, findings and actual shared unit from one producer.
    findings: StandaloneFindings,
    policy: Policy,
}

impl Analysed {
    /// What `analyze`, `validate`, `review` and `find-legacy` render: the
    /// report `tcl diag` builds for the same text — the analyser, the
    /// compiler checks, the source-style pass and, for a `sslictcl` source,
    /// the loader — with the optimiser off, as `tcl diag` has it: the
    /// rewrites are `optimize`'s, so an O-code a check emits is an
    /// `OptimiserOff` suppression rather than a finding that never existed,
    /// and one only the optimiser emits is in the optimiser's not-run entry
    /// for the same reason.
    fn diagnostics_report(&self) -> Report {
        let mut policy = self.policy.clone();
        policy.optimiser.enabled = false;
        let mut report = self.report_under(self.findings.produced.clone(), &policy);
        report.declare_optimiser_skip(&policy);
        report
    }

    /// What `code_actions` reads: the producers' findings and the optimiser's
    /// `rewrites`, under the policy as the layers resolve it, the optimiser
    /// switch included — the editor's lightbulb decides the same way.
    fn actions_report(&self, rewrites: Vec<Finding>) -> Report {
        let mut produced = self.findings.produced.clone();
        produced.extend(rewrites);
        self.report_under(produced, &self.policy)
    }

    /// `produced` with the report's own producers under `policy`, the
    /// analyser's skip declared.
    fn report_under(&self, produced: Vec<Finding>, policy: &Policy) -> Report {
        let mut report = document_report_with_analysis(
            &DocumentSource {
                text: &self.source,
                analysis_text: &self.analysis_text,
                decode: None,
                dialect: self.dialect,
                pass: SourcePass::Tcl {
                    line_length: DEFAULT_LINE_LENGTH,
                },
            },
            produced,
            policy,
            Some(&self.findings.analysis),
        );
        report.declare_analyser_skip(policy);
        report
    }
}

/// Analyse `source` under `dialect` (fresh analyser per call, like the facades)
/// for a tool that reads the analysis's facts — symbols, scopes, docstrings —
/// and reports no diagnostic. A diagnostics tool uses [`analyse_under`].
///
/// [`registry`] first, then the overlay key it built: the analyser resolves its
/// own registry from the dialect profile, so without the key it would miss the
/// bundled `.tclspec` loadables and call every EDA vendor command unknown.
pub(crate) fn analyse(source: &str, dialect: &str) -> AnalysisResult {
    let _ = registry(dialect);
    Analyser::new()
        .with_pack_overlay(tcl_spectcl::bundled::packs().key)
        .analyse(source, dialect)
}

/// [`analyse`] for a diagnostics tool: the producer run `tcl diag` makes
/// (`standalone_findings`) — the analyser, with the policy's production skip
/// that `inputs` resolve, exactly as the editor's `file_analysis` is handed
/// it, and the compiler checks over the same unit — whose findings are then
/// decided under the analysis's own directives.
///
/// [`registry`] first, as [`analyse`] does, so the bundled loadables the
/// overlay key names are installed; the analysis form of the source (a lone
/// `\r` rewritten) is what the producers read, as on every other surface.
fn analyse_under(source: &str, dialect: &str, inputs: &PolicyInputs) -> Analysed {
    let registry = registry(dialect);
    let profile = crate::environment::profile_for_dialect(dialect);
    let skip = inputs.builder().dialect(profile).build().production_skip();
    let analysis_text = tcl_lexer::normalise_lone_cr(source).into_owned();
    let standalone = standalone_findings(
        &StandaloneDocument {
            source: &analysis_text,
            file_path: None,
            dialect: profile,
            registry: &registry,
            pack_overlay: tcl_spectcl::bundled::packs().key,
            external_call_sites: None,
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
        },
        &skip,
    );
    let policy = inputs
        .builder()
        .dialect(profile)
        .directives(Directives::from_analysis(&standalone.analysis, source))
        .build();
    Analysed {
        source: source.to_owned(),
        analysis_text,
        dialect: profile,
        findings: standalone,
        policy,
    }
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

// ── Diagnostics tools ─────────────────────────────────────────────────

#[cfg(test)]
mod format_dialect_tests;

#[cfg(test)]
mod find_legacy_tests;

#[cfg(test)]
mod wasm_pipeline_tests;

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

/// Every tool as `(name, description, input_schema)` — the data the rmcp
/// `list_tools` handler turns into `rmcp::model::Tool`s.
pub fn tool_schemas() -> Vec<(&'static str, &'static str, Value)> {
    TOOLS
        .iter()
        .map(|t| (t.name, t.description, input_schema(t)))
        .collect()
}

/// The tool table of `docs/kcs/features/kcs-feature-mcp-server.md`, rendered
/// from [`TOOLS`]: one row per tool with its description and its arguments,
/// required ones marked `*` and closed ones followed by their values.
#[cfg(test)]
fn kcs_tool_table() -> String {
    use std::fmt::Write as _;
    let cell = |text: &str| text.replace('|', "\\|");
    let mut out =
        String::from("| Tool | Description | Arguments |\n|------|-------------|-----------|\n");
    for tool in TOOLS {
        let schema = input_schema(tool);
        let arguments: Vec<String> = tool
            .params
            .iter()
            .map(|(name, _, _)| {
                let mut argument = format!("`{name}`");
                if tool.required.contains(name) {
                    argument.push('*');
                }
                if *name != DIALECT_PARAM
                    && let Some(values) = schema["properties"][*name]["enum"].as_array()
                {
                    let values: Vec<&str> = values.iter().filter_map(Value::as_str).collect();
                    let _ = write!(argument, " ({})", values.join(", "));
                }
                argument
            })
            .collect();
        let _ = writeln!(
            out,
            "| `{}` | {} | {} |",
            tool.name,
            cell(tool.description),
            cell(&arguments.join(", "))
        );
    }
    out
}

/// Run a tool by name against its JSON `arguments`, returning the raw JSON
/// result — or `None` when the tool is unknown.
pub fn dispatch(name: &str, args: &Value) -> Option<Value> {
    TOOLS
        .iter()
        .find(|t| t.name == name)
        .map(|t| (t.handler)(args))
}

/// The KCS note's tool table is generated from [`TOOLS`]; set
/// `TCL_MCP_BLESS=1` to rewrite it (#2266).
#[cfg(test)]
mod kcs_note_tests;

#[cfg(test)]
mod dialect_param_tests;

#[cfg(test)]
mod source_integrity_tests;

#[cfg(test)]
mod policy_tests;

#[cfg(test)]
mod docstring_tests;

#[cfg(test)]
mod original_refactor_request_tests;

#[cfg(test)]
mod original_event_source_tests;

#[cfg(test)]
mod retained_graph_input_tests;

#[cfg(test)]
mod check_payload_wire_tests;
