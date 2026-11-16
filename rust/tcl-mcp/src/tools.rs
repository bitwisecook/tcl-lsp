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

use std::collections::HashSet;

use serde_json::{Map, Value, json};
use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_dialect::DialectProfile;
use tcl_lexer::{LexerConfig, LineIndex, SourceMap, Span, Utf16Col};
use tcl_lsp_core::config_ini;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::diagnostic_policy::{
    Directives, Finding, Policy, PolicyBuilder, PolicyLayer, Reason, Report, Shown,
};
use tcl_lsp_core::diagnostic_report::{
    DocumentSource, SourcePass, StandaloneDocument, StandaloneFindings,
    document_report_with_analysis, optimise_under_policy, standalone_findings,
};
use tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH;
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;

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

fn refactoring_json(source: &str, r: &tcl_lsp_core::refactor::Refactoring) -> Value {
    let mut result = json!({
        "title": r.title,
        "rewritten": r.apply(source),
        "edit_count": r.edits.len(),
    });
    if let Some(reason) = &r.disabled {
        result["disabled"] = json!(reason);
    }
    result
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

/// Serialise one shown finding to `{code, severity, message, range,
/// category, fixes?}` (the `_facade_diagnostic_to_dict` wire shape), at the
/// severity the policy resolved.
fn diag_to_json(shown: &Shown<'_>, sm: &SourceMap<'_>) -> Value {
    let d = shown.finding;
    let code = d.code.as_str();
    let mut obj = json!({
        "code": code,
        "severity": shown.severity.as_str(),
        "message": d.message,
        "range": byte_range(sm, d.span),
        "category": crate::diag_meta::meta().categorise(code),
    });
    if let Some(subject) = d.structured_data() {
        obj["data"] = subject;
    }
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

/// One suppressed finding as `{code, range, reason, message}`
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, MCP JSON).
/// `message` is added beside the page's three keys: without it a suppressed
/// W210 does not say which variable.
fn suppressed_to_json(finding: &Finding, reason: Reason, sm: &SourceMap<'_>) -> Value {
    json!({
        "code": finding.code.as_str(),
        "range": byte_range(sm, finding.span),
        "reason": reason.to_string(),
        "message": finding.message,
    })
}

/// The `suppressed` array a diagnostics tool payload gains: every suppressed
/// finding `keep` allows through [`suppressed_to_json`], then every declared
/// gap but the default-off seed (`null` range and message), then one entry
/// per producer the tool did not run and reason, `{producer, codes, range,
/// reason, message}` — its codes listed together, not an entry each: like
/// the seed, they are the same on every call and would bury the answer.
/// `keep` restricts every part (a not-run entry keeps the codes that pass,
/// and goes when none does), so an agent can see that a finding exists and
/// was suppressed rather than concluding the code is clean.
fn suppressed_json(report: &Report, sm: &SourceMap<'_>, keep: impl Fn(&str) -> bool) -> Vec<Value> {
    let mut out: Vec<Value> = report
        .suppressed()
        .filter(|(finding, _)| keep(finding.code.as_str()))
        .map(|(finding, reason)| suppressed_to_json(finding, reason, sm))
        .collect();
    out.extend(
        report
            .gaps()
            .filter(|(code, reason)| !matches!(reason, Reason::DefaultOff) && keep(code.as_str()))
            .map(|(code, reason)| {
                json!({
                    "code": code.as_str(),
                    "range": Value::Null,
                    "reason": reason.to_string(),
                    "message": Value::Null,
                })
            }),
    );
    out.extend(report.not_run().into_iter().filter_map(|row| {
        let codes: Vec<&str> = row
            .codes
            .iter()
            .map(|code| code.as_str())
            .filter(|code| keep(code))
            .collect();
        (!codes.is_empty()).then(|| {
            json!({
                "producer": row.producer.as_str(),
                "codes": codes,
                "range": Value::Null,
                "reason": row.reason.to_string(),
                "message": row.message(),
            })
        })
    }));
    out
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

/// The independently selected source event cards in this current analysis.
fn event_source_cards(
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<tcl_lsp_core::document_symbols::DocumentSymbol> {
    tcl_lsp_core::document_symbols::document_symbols_from_analysis(source, analysis)
        .into_iter()
        .filter(|card| card.kind == tcl_lsp_core::document_symbols::SymbolKind::Event)
        .collect()
}

/// Order supported authored event units after actual source-role selection.
/// The catalogue supplies presentation ordering, never runtime event authority.
fn event_order_list(source: &str, analysis: &AnalysisResult) -> Vec<Value> {
    let events = EventRegistry::build();
    let mut seen = HashSet::new();
    let cards = event_source_cards(source, analysis);
    let Some(declarations) = tcl_lsp_core::vendor_declaration::declarations(source, analysis)
    else {
        return Vec::new();
    };
    let index = LineIndex::new(source);
    let names = declarations
        .into_iter()
        .filter(|row| {
            row.symbol_metadata()
                .is_some_and(|metadata| metadata.kind == tcl_registry::DefinedSymbolKind::Event)
        })
        .filter(|row| {
            cards.iter().any(|card| {
                index.offset_at_utf16(
                    card.selection_range.start_line,
                    Utf16Col::new(card.selection_range.start_character),
                    source,
                ) == row.span().start()
            })
        })
        .filter_map(|row| row.input().literal_units(row.purpose()))
        .filter(|units| units.is_ascii())
        .filter_map(|units| std::str::from_utf8(units).ok())
        .map(str::to_ascii_uppercase)
        .filter(|event| seen.insert(event.clone()))
        .collect::<Vec<_>>();
    events
        .order_events(&names)
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let multiplicity = events.event_multiplicity(&name);
            json!({"index":i+1,"name":name,"multiplicity":multiplicity})
        })
        .collect()
}

/// Readable selected source event headers; unavailable units remain source cards.
fn detect_events(source: &str, analysis: &AnalysisResult) -> Vec<Value> {
    let mut seen = HashSet::new();
    event_source_cards(source, analysis)
        .into_iter()
        .filter(|card| seen.insert(card.name.clone()))
        .map(|card| json!({"name":card.name,"line":card.selection_range.start_line}))
        .collect()
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

type SourceGraphBuilder = fn(
    &str,
    &CommandRegistry,
    Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> Result<Value, tcl_lsp_core::graphs::GraphSourceInputDecline>;

fn supplied_source_graph(args: &Value, build: SourceGraphBuilder) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let Some(input) = analysis.resolved_input.as_ref() else {
        return json!({ "error": "The graph's retained source input is unavailable" });
    };
    let context = input.context_registry();
    build(source, context.commands(), Some(input))
        .unwrap_or_else(|reason| json!({ "error": reason.to_string() }))
}

fn call_graph(args: &Value) -> Value {
    supplied_source_graph(args, tcl_lsp_core::graphs::call_graph_with_source_input)
}

fn symbol_graph(args: &Value) -> Value {
    supplied_source_graph(args, tcl_lsp_core::graphs::symbol_graph_with_source_input)
}

fn dataflow_graph(args: &Value) -> Value {
    supplied_source_graph(args, tcl_lsp_core::graphs::dataflow_graph_with_source_input)
}

fn def_use_chains(args: &Value) -> Value {
    let mut graph =
        supplied_source_graph(args, tcl_lsp_core::graphs::def_use_graph_with_source_input);
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
    supplied_source_graph(
        args,
        tcl_lsp_core::graphs::memory_alias_graph_with_source_input,
    )
}

fn diagram(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_diagram::diagram_data_for_dialect(
        source,
        &registry(&dialect),
        tcl_lsp_core::profile_for_dialect(&dialect),
    )
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
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let ordered = event_order_list(source, &analysis);
    json!({ "events": ordered, "total": ordered.len() })
}

fn format_source(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    // The tool's resolved dialect drives the formatter as one fact:
    // lexer grammar, rewrite-candidate release, and forward range —
    // otherwise every dialect would format as modern Tcl.
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
    optimize_with(args, &PolicyInputs::for_call(args, "optimiser"))
}

/// [`optimize`] under `inputs`: the rewrites the policy shows are applied,
/// on every pass, so a `# noqa`, a file-wide directive or a code the profile
/// or a layer turned off keeps a rewrite off exactly as it keeps a squiggle
/// off (`docs/design/compiler/diagnostic-policy.md` § Adapters).
///
/// A non-empty `profile` argument is the profile in force over the global
/// file's `[optimiser] profile`, which applies only when the call names none,
/// and `full` when neither does: the profile is a request parameter with a
/// project default (§ Configuration). The profile in force sets the passes.
///
/// The loop reads the analysis form of the source (a lone `\r` rewritten to
/// `\n`), as `tcl opt` does, so `optimized_source` and every range are on the
/// client's line model, and `changed` says whether a rewrite applied.
fn optimize_with(args: &Value, inputs: &PolicyInputs) -> Value {
    use tcl_compiler::optimiser::profiles::OptimisationProfile;

    let raw = arg_str(args, "source");
    let dialect = resolve_dialect(args, raw);
    let analysis_text = tcl_lexer::normalise_lone_cr(raw);
    let source: &str = &analysis_text;
    let named = arg_str(args, "profile");
    let requested = (!named.is_empty()).then(|| OptimisationProfile::parse(named));
    let dialect_profile = crate::environment::profile_for_dialect(&dialect);
    let policy = inputs
        .builder()
        .requested_profile(requested)
        .default_profile(OptimisationProfile::Full)
        .dialect(dialect_profile)
        .build();
    let profile = policy.optimiser.profile;
    let registry = registry(&dialect);
    let overlay = tcl_spectcl::bundled::packs().key;
    let context = match tcl_registry::model::resolve_environment(dialect_profile.name)
        .context_registry(&tcl_registry::model::KeyedVersions::default(), overlay)
    {
        Ok(context) => context.with_command_store(std::sync::Arc::clone(&registry)),
        Err(miss) => return json!({ "error": miss.to_string() }),
    };
    let analysis_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        dialect_profile,
        dialect_profile,
        std::sync::Arc::new(context),
        LexerConfig::for_file_grammar(dialect_profile.grammar),
    );
    let optimised = optimise_under_policy(
        source,
        &registry,
        Some(&analysis_input),
        profile.max_iterations(),
        &policy,
    );
    let (optimised, opts, iterations) = (optimised.text, optimised.applied, optimised.iterations);

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
    let profile = crate::environment::profile_for_dialect(&dialect);
    let unit = tcl_compiler::compilation_unit::CompilationUnit::build_for_profile(
        source, &registry, false, profile,
    );
    let mut wasm = tcl_codegen::codegen::wasm::compile_wasm(
        &unit,
        &registry,
        tcl_codegen::codegen::wasm::WasmCompileOptions::hosted(),
    );
    let codegen_plan = wasm.plan.as_str();
    let semantic_decline = wasm
        .plan
        .semantic_decline()
        .map(tcl_codegen::codegen::wasm::WasmSemanticDecline::as_str);
    let semantic_decline_detail = wasm
        .plan
        .semantic_decline()
        .map(tcl_codegen::codegen::wasm::WasmSemanticDecline::detail_kind);
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
        &AnalysisResult,
        &LineIndex,
    ) -> Option<tcl_lsp_core::refactor::Refactoring>,
) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = analyse(source, &dialect);
    match f(source, off, &analysis, &idx) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

fn inline_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let (idx, off) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let analysis = analyse(source, &dialect);
    let Some(context) = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry)
    else {
        return Value::Null;
    };
    match tcl_lsp_core::refactor::inline_variable(
        source,
        off,
        &analysis,
        context.commands().as_ref(),
        &idx,
    ) {
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
    let analysis = analyse(source, &dialect);
    match tcl_lsp_core::refactor::brace_expr(source, off, &analysis) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

// ── Diagnostics tools ─────────────────────────────────────────────────

// Each diagnostics tool reads the report's shown set and keeps its own
// grouping — the `diag_meta` categories, the security / taint / thread
// split, the convertible-code table — which is presentation and stays here
// (`docs/design/compiler/diagnostic-policy.md` § Adapters). The `_with`
// forms take the layers explicitly, so a test can resolve under layers of
// its own rather than under the machine's `config.ini`.

fn analyze(args: &Value) -> Value {
    analyze_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

fn analyze_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(source);
    let diagnostics: Vec<Value> = report.shown().map(|d| diag_to_json(&d, &sm)).collect();
    let symbols: Vec<Value> = tcl_lsp_core::document_symbols::document_symbols_from_analysis(
        source,
        &analysed.findings.analysis,
    )
    .iter()
    .map(doc_symbol_to_json)
    .collect();
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "diagnostic_count": diagnostics.len(),
        "diagnostics": diagnostics,
        "symbols": symbols,
        "events": detect_events(source, &analysed.findings.analysis),
        "event_order": event_order_list(source, &analysed.findings.analysis),
        "suppressed": suppressed_json(&report, &sm, |_| true),
    })
}

fn validate(args: &Value) -> Value {
    validate_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

fn validate_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(source);
    let meta = crate::diag_meta::meta();
    let mut categories = Map::new();
    for (key, label) in &meta.category_order {
        let items: Vec<Value> = report
            .shown()
            .filter(|d| meta.categorise(d.finding.code.as_str()) == key)
            .map(|d| diag_to_json(&d, &sm))
            .collect();
        if !items.is_empty() {
            categories.insert(key.clone(), json!({ "label": label, "items": items }));
        }
    }
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "categories": Value::Object(categories),
        "total": report.shown().count(),
        "suppressed": suppressed_json(&report, &sm, |_| true),
    })
}

fn review(args: &Value) -> Value {
    review_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

fn review_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(source);
    let meta = crate::diag_meta::meta();
    let filt = |set: &std::collections::HashSet<String>| -> Vec<Value> {
        report
            .shown()
            .filter(|d| set.contains(d.finding.code.as_str()))
            .map(|d| diag_to_json(&d, &sm))
            .collect()
    };
    let security = filt(&meta.security_codes);
    let taint = filt(&meta.taint_codes);
    let thread = filt(&meta.thread_codes);
    let total = security.len() + taint.len() + thread.len();
    let suppressed = suppressed_json(&report, &sm, |code| {
        meta.security_codes.contains(code)
            || meta.taint_codes.contains(code)
            || meta.thread_codes.contains(code)
    });
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "security": security,
        "taint": taint,
        "thread_safety": thread,
        "total": total,
        "suppressed": suppressed,
    })
}

fn find_legacy(args: &Value) -> Value {
    find_legacy_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

fn find_legacy_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(source);
    // Shared with `tcl-cli`'s `find-legacy` verb (`tcl_cli::CONVERTIBLE_CODES`/
    // `conversion_for`) rather than a second hand-duplicated copy of the same
    // 6-code table.
    let patterns: Vec<Value> = report
        .shown()
        .filter(|d| tcl_cli::CONVERTIBLE_CODES.contains(&d.finding.code.as_str()))
        .map(|d| {
            let mut obj = diag_to_json(&d, &sm);
            let conversion = tcl_cli::conversion_for(d.finding.code.as_str());
            obj.as_object_mut()
                .expect("json object")
                .insert("conversion".to_owned(), json!(conversion));
            obj
        })
        .collect();
    let suppressed = suppressed_json(&report, &sm, |code| {
        tcl_cli::CONVERTIBLE_CODES.contains(&code)
    });
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "total": patterns.len(),
        "patterns": patterns,
        "suppressed": suppressed
    })
}

#[cfg(test)]
mod format_dialect_tests;

#[cfg(test)]
mod find_legacy_tests;

#[cfg(test)]
mod wasm_pipeline_tests;

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
    let command = arg_str(args, "command_name");
    let reg = registry(IRULES_DIALECT);
    // Query availability through the fixed iRules context over this store.
    let irules = crate::environment::context_for_dialect(IRULES_DIALECT);
    let Some(spec) = irules.resolve_spec(&reg, command) else {
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
    let keyed_range = irules.keyed_version_range(spec);
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
    let switches = irules.available_option_names(spec);
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
    let analysis = analyse(source, &dialect);
    let syms: Vec<Value> =
        tcl_lsp_core::document_symbols::document_symbols_from_analysis(source, &analysis)
            .iter()
            .map(doc_symbol_to_json)
            .collect();
    json!({ "symbols": syms })
}

fn hover(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands().as_ref());
    match tcl_lsp_core::hover::hover(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        commands,
    ) {
        Some(h) => json!({ "contents": h.value }),
        None => Value::Null,
    }
}

fn complete(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands().as_ref());
    let items = tcl_lsp_core::completion::completions(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        commands,
        None,
        tcl_lsp_core::profile_for_dialect(&dialect),
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
        tcl_lsp_core::profile_for_dialect(&dialect),
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
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands().as_ref());
    let edits = tcl_lsp_core::rename::rename(
        source,
        tcl_lsp_core::profile_for_dialect(&dialect),
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        arg_str(args, "new_name"),
        &analysis,
        commands,
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
    code_actions_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

/// [`code_actions`] under `inputs`: one report over the analyser's
/// findings, the compiler checks and the optimiser's rewrites, so a fix is
/// offered for a finding the document shows and for no other — and a shown
/// rewrite is an offerable action, not only a diagnostic payload.
fn code_actions_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    code_actions_from_analysis(args, &analysed)
}

/// Render actions from the already supplied producer. Optimisation reads its
/// retained unit and complete current input rather than rebuilding a world.
fn code_actions_from_analysis(args: &Value, analysed: &Analysed) -> Value {
    let range = LspRange {
        start_line: arg_u32(args, "start_line"),
        start_character: arg_u32(args, "start_character"),
        end_line: arg_u32(args, "end_line"),
        end_character: arg_u32(args, "end_character"),
    };
    // The MCP tool analyses one standalone source string with no workspace
    // behind it, so the analysed set — the analyser's and the compiler
    // checks' findings — is the published set; the rewrites join it under
    // the same policy, whose optimiser switch stays the layers'. Both run
    // over the analysis form, as the editor's lightbulb does, so a lone-`\r`
    // source's ranges are on the client's line model.
    let rewrites: Vec<Finding> = analysed
        .findings
        .optimisation_candidates(&analysed.analysis_text)
        .into_iter()
        .map(Finding::from)
        .collect();
    let report = analysed.actions_report(rewrites);
    let actions: Vec<Value> = tcl_lsp_core::code_actions::code_actions(
        &analysed.analysis_text,
        range,
        Some(&analysed.findings.analysis),
        &report,
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
            if let Some(reason) = &a.disabled {
                m.insert("disabled".to_owned(), json!(reason));
            }
            obj
        })
        .collect();
    json!({ "actions": actions })
}

// ── Refactor tools ────────────────────────────────────────────────────

fn extract_variable(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
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
        (start_off, end_off),
        var_name,
        &analysis,
        &line_index,
    ) {
        Some(r) => refactoring_json(source, &r),
        None => Value::Null,
    }
}

/// Both tool surfaces keep the analysis owner through conditional source edits.
fn datagroup_advice(
    source: &str,
    cursor: u32,
    name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<tcl_lsp_core::refactor::Refactoring> {
    tcl_lsp_core::refactor::extract_to_datagroup_with_analysis(
        source, cursor, name, analysis, line_index,
    )
}

fn extract_datagroup(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let (line_index, cursor) = cursor(source, arg_u32(args, "line"), arg_u32(args, "character"));
    let dg_name = arg_str(args, "dg_name");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let Some(r) = datagroup_advice(source, cursor, dg_name, &analysis, &line_index) else {
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
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands());
    let mut available: Vec<Value> = Vec::new();
    let mut push = |tool: &str, r: Option<tcl_lsp_core::refactor::Refactoring>| {
        if let Some(r) = r {
            let mut candidate = json!({ "tool": tool, "title": r.title });
            if let Some(reason) = r.disabled {
                candidate["disabled"] = json!(reason);
            }
            available.push(candidate);
        }
    };
    // Extract-variable only applies to a non-empty selection.
    if start_off != end_off {
        push(
            "extract_variable",
            tcl_lsp_core::refactor::extract_variable(
                source,
                (start_off, end_off),
                "result",
                &analysis,
                &line_index,
            ),
        );
    }
    if let Some(commands) = commands {
        push(
            "inline_variable",
            tcl_lsp_core::refactor::inline_variable(
                source,
                start_off,
                &analysis,
                commands.as_ref(),
                &line_index,
            ),
        );
    }
    push(
        "if_to_switch",
        tcl_lsp_core::refactor::if_to_switch(source, start_off, &analysis, &line_index),
    );
    push(
        "switch_to_dict",
        tcl_lsp_core::refactor::switch_to_dict(source, start_off, &analysis, &line_index),
    );
    push(
        "brace_expr",
        tcl_lsp_core::refactor::brace_expr(source, start_off, &analysis),
    );
    push(
        "extract_datagroup",
        datagroup_advice(source, start_off, "", &analysis, &line_index),
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
    let Some(declarations) =
        tcl_lsp_core::procedure_symbol::for_literal_name(source, &analysis, proc_name)
    else {
        return json!({ "error": "The current original procedure inventory is unavailable" });
    };
    match declarations.as_slice() {
        [proc] => {
            let tag = tcl_lsp_core::formatting::resolve_tag_style(style);
            let docstring = tcl_lsp_core::formatting::generate_stub_for_proc(
                proc, tag, decoration, '.', 70, "",
            );
            json!({ "proc": proc_name, "docstring": docstring })
        }
        [] => json!({
            "error": format!("Proc '{proc_name}' not found at global scope; use its qualified name for a namespace declaration"),
        }),
        _ => json!({
            "error": format!("Proc '{proc_name}' has multiple source declarations"),
            "declarations": declarations.iter().map(|proc| json!({
                "name_start": proc.name_span.start(),
                "name_end": proc.name_span.end(),
            })).collect::<Vec<_>>(),
        }),
    }
}

fn read_proc_docs(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let Some(procs) = tcl_lsp_core::procedure_symbol::declarations(source, &analysis) else {
        return json!({ "error": "The current original procedure inventory is unavailable" });
    };
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
            if let Some(original) = analysis
                .original_procedure_declarations()
                .find(|row| row.metadata().name_span == proc.name_span)
            {
                m.insert(
                    "name_source".to_owned(),
                    json!(source.get(original.name_input().span().as_range())),
                );
            }
            m.insert("name_start".to_owned(), json!(proc.name_span.start()));
            m.insert("name_end".to_owned(), json!(proc.name_span.end()));
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
    let Some(procs) = tcl_lsp_core::procedure_symbol::declarations(source, &analysis) else {
        return json!({ "error": "The current original procedure inventory is unavailable" });
    };
    let total_procs = procs.len();

    // Insert from the last original declaration so earlier source positions
    // stay valid while each independent declaration receives its own comment.
    let mut targets: Vec<_> = procs
        .into_iter()
        .filter(|proc| proc.doc.is_empty())
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

/// The property name every dialect-taking tool spells its dialect argument
/// with. [`input_schema`] keys the catalogue enum off it, so a tool advertises
/// the full dialect list by naming its parameter rather than by repeating the
/// catalogue per tool.
const DIALECT_PARAM: &str = "dialect";

/// [`spectcl_check`](crate::spectcl::spectcl_check)'s discovery-tier
/// parameter. [`input_schema`] gives it a closed `enum`, the four
/// [`tcl_spectcl::Tier`] spellings.
const TIER_PARAM: &str = "tier";

/// [`spectcl_check`](crate::spectcl::spectcl_check)'s Workspace Trust
/// parameter. [`input_schema`] gives it a closed `enum`, the two
/// [`tcl_dialect::model::WorkspaceTrust`] spellings.
const TRUST_PARAM: &str = "trust";

/// Every selectable environment's canonical id, in selectable order, read from
/// the live registry.
fn dialect_names() -> Vec<String> {
    tcl_registry::model::selectable_environments()
        .iter()
        .map(|environment| environment.id.to_string())
        .collect()
}

/// The schema for a dialect-valued property: the selectable environments as a
/// JSON-Schema `enum`, plus each `name: description` pair appended to `desc` so
/// a model reading only the description still sees what each name means.
///
/// The `enum` lists canonical ids only; the runtime ingress keeps accepting
/// every declared name.
fn dialect_schema(desc: &str) -> Value {
    let selectable = tcl_registry::model::selectable_environments();
    let pairs: Vec<String> = selectable
        .iter()
        .map(|environment| format!("{}: {}", environment.id, environment.description()))
        .collect();
    json!({
        "type": "string",
        "description": format!(
            "{desc}. Canonical names: {}. Registered aliases are accepted too.",
            pairs.join("; ")
        ),
        "enum": dialect_names(),
    })
}

const SRC: Param = ("source", "string", "Tcl or iRules source code");
const DISABLE: Param = (
    "disable",
    "string",
    "Comma-separated diagnostic codes to turn off for this call — the invocation \
     layer, over the user's global config.ini",
);
const ENABLE: Param = (
    "enable",
    "string",
    "Comma-separated diagnostic codes to turn on for this call, including a \
     default-off code such as W242",
);
const OPT_DISABLE: Param = (
    "disable",
    "string",
    "Comma-separated optimisation codes to turn off on top of the profile",
);
const OPT_ENABLE: Param = (
    "enable",
    "string",
    "Comma-separated optimisation codes to turn on that the profile turns off",
);
const DIALECT: Param = (
    DIALECT_PARAM,
    "string",
    "Language dialect; auto-detected if empty",
);
const TIER: Param = (
    TIER_PARAM,
    "string",
    "Discovery tier to preview this pack loading at — bundled, user, workspace, or \
     studio-override; defaults to workspace, the tier a .tclspec file actually installs \
     at. Governs untrusted_tier_refusal (never raised for bundled or user, which no trust \
     state makes untrusted) and, with trust, dormant_hooks and stamp_refusals; never which \
     commands, notices, or hooks are reported — the pack is always evaluated as trusted",
);
const TRUST: Param = (
    TRUST_PARAM,
    "string",
    "Workspace Trust state to preview this pack loading under — trusted or untrusted; \
     defaults to trusted. With tier, decides dormant_hooks (a workspace hook body stays \
     dormant only when untrusted)",
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
        params: &[SRC, DIALECT],
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
        description: "Find optimisation opportunities and produce rewritten source. Only the \
                      rewrites the source's policy shows are applied: a `# noqa` on a command, a \
                      top-of-file `# tcl-lsp: disable=`, the profile and the global config.ini \
                      keep a rewrite off exactly as they keep a diagnostic off.",
        params: &[
            SRC,
            DIALECT,
            (
                "profile",
                "string",
                "off | readability | standard | full | aggressive. Named, it wins over the \
                 global config.ini's [optimiser] profile; omitted, that profile applies, \
                 else full",
            ),
            OPT_DISABLE,
            OPT_ENABLE,
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
        description: "Full analysis: diagnostics (+category), document symbols, detected events, and event firing order. \
                      Diagnostics are the source's shown set, including the compiler checks (S1xx, T1xx, \
                      IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118) — inline `# noqa`, \
                      top-of-file `# tcl-lsp: disable=`, the global config.ini and the call's disable/enable \
                      arguments all apply, as in the editor; and a `suppressed` array: every finding the policy \
                      hides, with its reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: analyze,
    },
    ToolDef {
        name: "validate",
        description: "Diagnostics grouped by category (security, taint, thread-safety, control-flow, performance, style, …), \
                      from the source's shown set, including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and \
                      the source-style pass (W111, W112, W115, W118) (directives, the global config.ini and \
                      disable/enable apply); and a `suppressed` array: every finding the policy hides, with its \
                      reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: validate,
    },
    ToolDef {
        name: "review",
        description: "Security, taint, and thread-safety diagnostics for a focused review, from the source's shown set, \
                      including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, \
                      W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a \
                      `suppressed` array: every finding the policy hides, with its reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
        required: &["source"],
        handler: review,
    },
    ToolDef {
        name: "find-legacy",
        description: "Auto-convertible legacy patterns with a modernisation hint per finding, from the source's shown set, \
                      including the compiler checks (S1xx, T1xx, IRULE1xxx–5xxx) and the source-style pass (W111, \
                      W112, W115, W118) (directives, the global config.ini and disable/enable apply); and a \
                      `suppressed` array: every finding the policy hides, with its reason.",
        params: &[SRC, DIALECT, DISABLE, ENABLE],
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
        description: "Code actions (quick-fixes + refactors) for a selection range: a fix for every finding the source \
                      shows — analyser, compiler-check and optimiser rewrite alike — and for no other.",
        params: &[
            SRC, START_LINE, START_CHAR, END_LINE, END_CHAR, DIALECT, DISABLE, ENABLE,
        ],
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
            DIALECT,
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
            DIALECT,
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
            DIALECT_PARAM,
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
            (
                DIALECT_PARAM,
                "string",
                "Filter features by dialect (optional)",
            ),
            ("limit", "integer", "Max matches (default 20)"),
        ],
        required: &[],
        handler: help,
    },
    ToolDef {
        name: "xc_translate",
        description: "Translate an iRule to F5 Distributed Cloud (XC) config — terraform HCL, ves.io JSON, or pasteable XC Console documents, with per-command coverage.",
        params: &[
            SRC,
            (
                "output_format",
                "string",
                "'terraform', 'json', 'console' (one pasteable Console document per object), or 'both' (default: terraform + json)",
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
        description: "Build the shared, versioned static Tk UI model without executing source. Returns registry-resolved widgets, literal options, geometry evidence, source spans, conflicts, and explicit uncertainty.",
        params: &[SRC, DIALECT],
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
        description: "Validate a SpecTcl (.tclspec) pack by evaluating it in the deterministic sandbox (no clock/IO, budgets, transactional registration — safe on generated packs). Returns per-command draft fields, loader notices (dropped/unknown words), hooks with family and cacheability, hooks reading past their `-inputs`, and collisions with the shipped registry; plus `load_error` (determinism denial, budget, or Tcl error — nothing loads), `target_dependent` (`available?` was queried), `provenance` (what `tier`/`trust` resolve to), `untrusted_tier_refusal` (what an untrusted install at `tier` would refuse the pack for), `dormant_hooks` (hook bodies that stay dormant under that provenance — declarative facts install regardless; empty when that provenance refuses the pack, since nothing of it loads), and `stamp_refusals` (codegen_hook, inline_codegen_hook, or semantic_operation Intrinsic rows that install drops: only a bundled pack may stamp, and only as its `alias_of` target's own). Use spectcl_expand to see what a templated pack registered.",
        params: &[
            (
                "source",
                "string",
                "SpecTcl pack source text (the contents of a .tclspec file)",
            ),
            (
                DIALECT_PARAM,
                "string",
                "Dialect whose shipped registry to check names against; defaults to the session dialect",
            ),
            TIER,
            TRUST,
        ],
        required: &["source"],
        handler: crate::spectcl::spectcl_check,
    },
    ToolDef {
        name: "spectcl_expand",
        description: "Expand a SpecTcl (.tclspec) pack to canonical form: evaluate it in the sandbox and return the straight-line source of every registration it made (no proc/foreach/set). Review the expansion, not the loop, before shipping a templated pack and after every template edit; a straight-line pack comes back as itself. Also reports target-dependence (`available?` queried) and load notices. Hand the pack to spectcl_check for field-level validation.",
        params: &[(
            "source",
            "string",
            "SpecTcl pack source text (the contents of a .tclspec file)",
        )],
        required: &["source"],
        handler: crate::spectcl::spectcl_expand,
    },
    ToolDef {
        name: "spec_import",
        description: "Derive version ranges for a Tcl package's commands from LOCAL release snapshots (directories, .zip, or .tar.gz — never fetches): drafts each release, diffs them, and returns the rendered .tclspec pack (evidence in its header), per-command introduced/retired versions, and every contradiction. Obtain sources first with `tcl spec import --github OWNER/REPO` or git; hand the pack to spectcl_check.",
        params: &[
            (
                "snapshots",
                "array",
                "One entry per release, each {\"version\": \"1.2\", \"path\": \"/path/to/that/release\"} where path is a local directory, .zip or .tar.gz; order does not matter",
            ),
            (
                DIALECT_PARAM,
                "string",
                "Dialect every snapshot is analysed as; defaults to the session dialect",
            ),
            (
                "package",
                "string",
                "Pack name for the rendered speclib block; defaults to the name the sources `package provide`",
            ),
            (
                "complete_history",
                "boolean",
                "True only when the snapshots are EVERY release: presence in the earliest one is then an introduction. Defaults to false, which leaves introduced_version unset there",
            ),
        ],
        required: &["snapshots"],
        handler: crate::spec_import::spec_import,
    },
];

/// The four discovery-tier spellings [`TIER`] advertises — each tier's own
/// label, owned beside the argument's reader in `spectcl.rs`.
const TIER_VALUES: [&str; 4] = crate::spectcl::TIER_ARGUMENTS;

/// The two [`tcl_dialect::model::WorkspaceTrust`] spellings [`TRUST`]
/// advertises.
const TRUST_VALUES: [&str; 2] = ["trusted", "untrusted"];

/// The schema for a property whose value is one of a fixed, small word list —
/// a JSON-Schema `enum`, the same shape [`dialect_schema`] gives the dialect
/// property, without that property's dynamic catalogue lookup.
fn closed_string_schema(desc: &str, values: &[&str]) -> Value {
    json!({ "type": "string", "description": desc, "enum": values })
}

/// The JSON-Schema input-schema object (`{type, properties, required}`) for a
/// tool's parameters.
fn input_schema(t: &ToolDef) -> Value {
    let mut properties = Map::new();
    for (name, ty, desc) in t.params {
        let schema = if *name == DIALECT_PARAM {
            dialect_schema(desc)
        } else if *name == TIER_PARAM {
            closed_string_schema(desc, &TIER_VALUES)
        } else if *name == TRUST_PARAM {
            closed_string_schema(desc, &TRUST_VALUES)
        } else {
            json!({ "type": ty, "description": desc })
        };
        properties.insert((*name).to_owned(), schema);
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
