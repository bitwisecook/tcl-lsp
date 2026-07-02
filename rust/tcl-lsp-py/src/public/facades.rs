//! The public facades — `source/options in, structured result out`.
//!
//! Each facade is a thin translation layer over one pure crate entry
//! point: it builds the right config, calls the algorithm, resolves
//! spans to positions, and maps any error to the typed exception
//! hierarchy. No analysis logic lives here; that all stays in the
//! pure crates (`tcl-lexer`, `tcl-compiler`, `tcl-lsp-core`,
//! `tcl-bigip`, `tcl-bigip-query`).

use std::collections::HashMap;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use tcl_bigip_query::{QueryError, QueryOptions, run_query};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{Lexer, LexerConfig, LineIndex, SourceMap, Utf16Col};
use tcl_lsp_core::formatting::FormatterConfig;
use tcl_lsp_core::refactor::Refactoring;
use tcl_registry::CommandRegistry;
use tcl_registry::dialects::DialectSet;
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;

use super::errors::PublicError;
use super::options::FormatOptions;
use super::results::{
    AnalysisResult, BigipConfig, CommandInfo, DataGroupResult, Diagnostic, EventInfo, LexToken,
    ParseResult, ParsedCommand, QueryEdit, QueryFile, QueryResult, RefactorResult, WasmOutput,
};
use crate::compilation_unit::CompilationUnitHandle;

/// The default dialect for the Tcl facades — C Tcl 9.0.3 is the
/// reference standard (rewrite principle §0).
const DEFAULT_DIALECT: &str = "tcl9.0";

/// Tokenise and segment `source`, returning the token stream plus the
/// top-level command structure.
///
/// `dialect` gates version-specific lexing (`{*}` expansion, `0o`/`0b`
/// integer prefixes, iRules brace separators, …) via
/// [`LexerConfig::for_dialect`]. `uri`, when given, tags any raised
/// error. Raises `TclParseError` if the lexer rejects the source in a
/// strict-quoting dialect.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT, uri = None))]
pub(crate) fn parse_tcl(
    py: Python<'_>,
    source: &str,
    dialect: &str,
    uri: Option<String>,
) -> PyResult<ParseResult> {
    let config = LexerConfig::for_dialect(dialect);
    let lexer = Lexer::with_source_map(SourceMap::new(source), config);
    let sm = lexer.source_map().clone();
    let (tokens, warnings) = lexer.tokenise_all_with_warnings().map_err(|e| {
        PublicError::parse(e.to_string())
            .with_uri(uri)
            .into_pyerr(py)
    })?;

    let py_tokens = tokens
        .iter()
        .map(|tok| LexToken::from_token(py, &sm, tok))
        .collect::<PyResult<Vec<_>>>()?;

    let commands = segment_commands_with_offset_and_config(source, 0, config);
    let py_commands = commands
        .iter()
        .map(|cmd| ParsedCommand::from_segment(py, &sm, cmd))
        .collect::<PyResult<Vec<_>>>()?;

    Ok(ParseResult {
        tokens: py_tokens,
        commands: py_commands,
        warnings: warnings.into_iter().map(|w| w.message).collect(),
    })
}

/// Lower `source` to a [`CompilationUnit`], returning the opaque handle
/// downstream callers reuse across analyses.
///
/// `dialect=None` defaults to `tcl9.0` (the reference standard, matching
/// `parse_tcl` / `analyse_tcl`); a non-None dialect (e.g. `"f5-irules"`)
/// selects that dialect's registry and lowering branches.
/// `interprocedural=False` skips the (more expensive) interprocedural
/// summary pass. Raises `TclCompileError` if the source cannot be lexed.
///
/// The registry is built **for the effective dialect** so the lowering
/// honours the right Tcl version (`tcl9.0` reads leading zeros as decimal,
/// the 8.x-derived dialects as octal — `CommandRegistry::leading_zero_is_octal`)
/// and loads dialect-only commands (iRules `when`, …) needed to emit the
/// `::when::*` procedures.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = None, interprocedural = true, uri = None))]
pub(crate) fn compile_tcl(
    py: Python<'_>,
    source: &str,
    dialect: Option<&str>,
    interprocedural: bool,
    uri: Option<String>,
) -> PyResult<CompilationUnitHandle> {
    let effective_dialect = dialect.unwrap_or(DEFAULT_DIALECT);
    let registry = crate::registry::default_registry_for_dialect(effective_dialect);
    let config = LexerConfig::for_dialect(effective_dialect);
    if let Err(e) = Lexer::with_source_map(SourceMap::new(source), config).tokenise_all() {
        return Err(PublicError::compile(e.to_string())
            .with_uri(uri)
            .into_pyerr(py));
    }

    let unit = CompilationUnit::build_for_with_config(source, registry, false, config);
    let unit = if interprocedural {
        unit.with_interprocedural(registry, Some(effective_dialect))
    } else {
        unit
    };
    Ok(CompilationUnitHandle {
        inner: Arc::new(unit),
    })
}

/// Run the semantic analyser over `source`, returning its diagnostics
/// and the discovered top-level symbols.
///
/// `dialect` selects the command spec pack and dialect-gated checks
/// (e.g. `"f5-irules"` enables the iRule families). Raises
/// `TclAnalysisError` if the source cannot be lexed.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT, uri = None))]
pub(crate) fn analyse_tcl(
    py: Python<'_>,
    source: &str,
    dialect: &str,
    uri: Option<String>,
) -> PyResult<AnalysisResult> {
    let config = LexerConfig::for_dialect(dialect);
    if let Err(e) = Lexer::with_source_map(SourceMap::new(source), config).tokenise_all() {
        return Err(PublicError::analysis(e.to_string())
            .with_uri(uri)
            .into_pyerr(py));
    }

    let mut analyser = Analyser::new();
    let result = analyser.analyse(source, dialect);
    let sm = SourceMap::new(source);

    let diagnostics = result
        .diagnostics
        .iter()
        .map(|d| Diagnostic::from_core(py, &sm, d))
        .collect::<PyResult<Vec<_>>>()?;

    let mut procs: Vec<String> = result.all_procs.keys().cloned().collect();
    procs.sort();
    let mut classes: Vec<String> = result.all_classes.keys().cloned().collect();
    classes.sort();
    let mut variables: Vec<String> = result.all_variables.keys().cloned().collect();
    variables.sort();

    Ok(AnalysisResult {
        diagnostics,
        procs,
        classes,
        variables,
    })
}

/// Reformat `source` with the canonical Tcl formatter, returning the
/// formatted text.
///
/// `options` is an optional [`FormatOptions`]; omitting it uses the
/// formatter defaults.
#[pyfunction]
#[pyo3(signature = (source, *, options = None))]
pub(crate) fn format_tcl(source: &str, options: Option<PyRef<'_, FormatOptions>>) -> String {
    let registry = crate::registry::default_registry();
    let config = options.map_or_else(FormatterConfig::default, |o| o.config.clone());
    tcl_lsp_core::formatting::engine::format_tcl(source, &config, registry)
}

/// Parse `source` as a BIG-IP configuration, returning a summarised
/// [`BigipConfig`] plus its canonical JSON document.
///
/// `default_partition` names the partition bare object names are
/// rewritten under. With `strict=True`, a non-empty source that
/// yields no recognisable objects raises `BigipParseError` (the
/// "you passed the wrong file" guard); the parser is otherwise
/// recovering and never raises.
#[pyfunction]
#[pyo3(signature = (source, *, default_partition = "Common", strict = false, uri = None))]
pub(crate) fn parse_bigip_config(
    py: Python<'_>,
    source: &str,
    default_partition: &str,
    strict: bool,
    uri: Option<String>,
) -> PyResult<BigipConfig> {
    let config = tcl_bigip::parser::parse_bigip_conf(source, default_partition);
    let canonical = tcl_bigip::canonical::config_to_canonical(&config);
    let json = serde_json::to_string(&canonical).unwrap_or_else(|_| "{}".to_owned());
    let object_keys: Vec<String> = config
        .generic_objects
        .iter()
        .map(|(key, _)| key.clone())
        .collect();

    if strict && object_keys.is_empty() && !source.trim().is_empty() {
        return Err(PublicError::bigip_parse(
            "no recognisable BIG-IP objects in a non-empty source",
        )
        .with_uri(uri)
        .into_pyerr(py));
    }

    Ok(BigipConfig {
        default_partition: config.default_partition.clone(),
        object_count: object_keys.len(),
        object_keys,
        json,
    })
}

/// Run a BIG-IP query over one or more `(uri, source)` config sources,
/// returning the per-file values (read query) or rewrites (mutating
/// query).
///
/// `output` selects the render mode (`"auto"`, `"json"`, `"scf"`,
/// `"paths"`, `"raw"`, `"table"`, `"table-lineart"`, or a registered
/// renderer); an unknown mode raises `UnsupportedFeatureError`. Query
/// failures raise `BigipQueryError`. `enable_probes` opts in to live
/// network probes (off by default).
#[pyfunction]
#[pyo3(signature = (
    sources,
    query,
    *,
    names = None,
    partitions = None,
    merge = false,
    enable_probes = false,
    output = "auto",
))]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub(crate) fn query_bigip(
    py: Python<'_>,
    sources: Vec<(String, String)>,
    query: &str,
    names: Option<HashMap<String, String>>,
    partitions: Option<HashMap<String, String>>,
    merge: bool,
    enable_probes: bool,
    output: &str,
) -> PyResult<QueryResult> {
    if !is_supported_output(output) {
        return Err(PublicError::unsupported(format!(
            "unknown output mode {output:?}; expected one of \
             auto/json/scf/paths/raw/table/table-lineart or a registered renderer"
        ))
        .into_pyerr(py));
    }

    let opts = QueryOptions {
        names: names.unwrap_or_default(),
        partitions: partitions.unwrap_or_default(),
        side_inputs: Vec::new(),
        enable_probes,
        ca_bundle: None,
        ucs_cert_reader: None,
        merge,
    };

    let result = run_query(query, &sources, &opts).map_err(|e| query_error(py, query, &e))?;

    if result.has_mutation {
        let edits = result
            .edits_per_file
            .iter()
            .map(|(uri, applied)| {
                Py::new(
                    py,
                    QueryEdit {
                        uri: uri.clone(),
                        new_source: applied.new_source.clone(),
                        changed: applied.new_source != applied.original,
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(QueryResult {
            has_mutation: true,
            values: Vec::new(),
            edits,
        });
    }

    let values = result
        .values_per_file
        .iter()
        .map(|(uri, vals)| {
            let rendered = tcl_bigip_query::output::render(vals, output)
                .map_err(|e| query_error(py, query, &e))?;
            Py::new(
                py,
                QueryFile {
                    uri: uri.clone(),
                    output: rendered,
                },
            )
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(QueryResult {
        has_mutation: false,
        values,
        edits: Vec::new(),
    })
}

/// True if `mode` is a built-in or registered query render mode.
fn is_supported_output(mode: &str) -> bool {
    const BUILTIN: &[&str] = &[
        "auto",
        "scf",
        "raw",
        "paths",
        "json",
        "table",
        "table-lineart",
    ];
    BUILTIN.contains(&mode) || tcl_bigip_query::renderers::lookup(mode).is_some()
}

/// Translate a [`QueryError`] into the typed `BigipQueryError`, with a
/// variant-specific code and a resolved range for the positional
/// variants (`Lex` / `Parse`).
fn query_error(py: Python<'_>, query: &str, e: &QueryError) -> PyErr {
    let (code, positional) = match e {
        QueryError::Lex { .. } => ("BIGIP_QUERY_LEX", true),
        QueryError::Parse { .. } => ("BIGIP_QUERY_PARSE", true),
        QueryError::Eval(_) => ("BIGIP_QUERY_EVAL", false),
        QueryError::Edit(_) => ("BIGIP_QUERY_EDIT", false),
        QueryError::Builtin(_) => ("BIGIP_QUERY_BUILTIN", false),
        QueryError::Renderer(_) => ("BIGIP_QUERY_RENDERER", false),
    };
    let range = if positional {
        let sm = SourceMap::new(query);
        let pos = sm.position_at(u32::try_from(e.offset()).unwrap_or(0));
        Some((
            (pos.line, pos.character.get()),
            (pos.line, pos.character.get()),
        ))
    } else {
        None
    };
    PublicError::bigip_query(code, e.to_string())
        .with_range(range)
        .into_pyerr(py)
}

/// Look up registry metadata for an iRules `event` — a projection of
/// `tcl_registry::CommandRegistry::event_info` (the same data `f5 irule
/// event-info` serialises): whether it is known, its multiplicity, side,
/// transport, implied profiles, and the commands legal in it.
#[pyfunction]
#[pyo3(signature = (event, *, dialect = "f5-irules"))]
pub(crate) fn lookup_event_info(event: &str, dialect: &str) -> EventInfo {
    let mut registry = CommandRegistry::build_default();
    registry.load_dialect(DialectSet::IRULES);
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let info = registry.event_info(event, &events, &profiles);
    EventInfo {
        event: info.event,
        dialect: dialect.to_owned(),
        known: info.known,
        deprecated: info.deprecated,
        multiplicity: info.multiplicity.to_owned(),
        description: info.description,
        side: info.side.to_owned(),
        transport: info.transport,
        implied_profiles: info
            .implied_profiles
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        valid_commands: info.valid_commands,
    }
}

/// Look up registry metadata for a command in `dialect`: its hover summary and
/// synopsis, the switches it accepts, and (for iRules) the events it is legal
/// in (`valid_events`, via `irules_events_for_command`) with `valid_in_any_event`
/// set when the command carries no event constraint.
#[pyfunction]
#[pyo3(signature = (command, *, dialect = "f5-irules"))]
pub(crate) fn lookup_command_info(command: &str, dialect: &str) -> CommandInfo {
    let mut registry = CommandRegistry::build_default();
    registry.load_dialect(DialectSet::IRULES);
    let dset = DialectSet::parse(dialect).unwrap_or(DialectSet::IRULES);

    let query = command.trim();
    let Some(spec) = registry.get_for_dialect(query, dset) else {
        return CommandInfo {
            found: false,
            command: query.to_owned(),
            dialect: dialect.to_owned(),
            summary: String::new(),
            synopsis: Vec::new(),
            switches: Vec::new(),
            valid_events: Vec::new(),
            valid_in_any_event: false,
        };
    };
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    CommandInfo {
        found: true,
        command: query.to_owned(),
        dialect: dialect.to_owned(),
        summary: spec.hover.map_or(String::new(), |h| h.summary.to_owned()),
        synopsis: spec
            .hover
            .map(|h| h.synopsis.iter().map(|s| (*s).to_owned()).collect())
            .unwrap_or_default(),
        switches: spec
            .switch_names(Some(dset))
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        valid_events: registry
            .irules_events_for_command(query, &events, &profiles)
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        valid_in_any_event: spec.event_requires.is_none(),
    }
}

/// The byte offset of `(line, character)` (UTF-16 column) in `source`.
fn cursor_at(source: &str, line: u32, character: u32) -> (LineIndex, u32) {
    let line_index = LineIndex::new(source);
    let cursor = line_index.offset_at_utf16(line, Utf16Col::new(character), source);
    (line_index, cursor)
}

/// Package a [`Refactoring`] result for Python.
fn refactor_result(source: &str, r: &Refactoring) -> RefactorResult {
    RefactorResult {
        title: r.title.clone(),
        rewritten: r.apply(source),
        edit_count: r.edits.len(),
    }
}

/// Inline a single-use variable at `(line, character)` — replace the reference
/// with its value and remove the `set`. Returns `None` if the cursor is not on
/// an inlinable variable. Over `tcl_lsp_core::refactor::inline_variable`.
#[pyfunction]
#[pyo3(signature = (source, line, character, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn refactor_inline_variable(
    source: &str,
    line: u32,
    character: u32,
    dialect: &str,
) -> Option<RefactorResult> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let (line_index, cursor) = cursor_at(source, line, character);
    let analysis = Analyser::new().analyse(source, dialect);
    tcl_lsp_core::refactor::inline_variable(source, cursor, &analysis, registry, &line_index)
        .map(|r| refactor_result(source, &r))
}

/// Convert an `if`/`elseif` chain testing the same variable to a `switch` at
/// `(line, character)`. Returns `None` when the cursor is not on a convertible
/// chain. Over `tcl_lsp_core::refactor::if_to_switch`.
#[pyfunction]
#[pyo3(signature = (source, line, character, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn refactor_if_to_switch(
    source: &str,
    line: u32,
    character: u32,
    dialect: &str,
) -> Option<RefactorResult> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let (line_index, cursor) = cursor_at(source, line, character);
    tcl_lsp_core::refactor::if_to_switch(source, cursor, registry, &line_index)
        .map(|r| refactor_result(source, &r))
}

/// Convert a `switch` whose arms all assign the same variable to a `dict` at
/// `(line, character)`. Returns `None` when the cursor is not on a convertible
/// `switch`. Over `tcl_lsp_core::refactor::switch_to_dict`.
#[pyfunction]
#[pyo3(signature = (source, line, character, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn refactor_switch_to_dict(
    source: &str,
    line: u32,
    character: u32,
    dialect: &str,
) -> Option<RefactorResult> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let (line_index, cursor) = cursor_at(source, line, character);
    tcl_lsp_core::refactor::switch_to_dict(source, cursor, registry, &line_index)
        .map(|r| refactor_result(source, &r))
}

/// Extract the selection `(start_line, start_character)`–`(end_line,
/// end_character)` into a new `set var_name …` binding at `(line, character)`.
/// Returns `None` when the selection is not a valid value word. Over
/// `tcl_lsp_core::refactor::extract_variable`.
#[pyfunction]
#[pyo3(signature = (source, start_line, start_character, end_line, end_character, var_name = "result", *, dialect = DEFAULT_DIALECT))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn refactor_extract_variable(
    source: &str,
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
    var_name: &str,
    dialect: &str,
) -> Option<RefactorResult> {
    let _ = dialect;
    let line_index = LineIndex::new(source);
    let start_off = line_index.offset_at_utf16(start_line, Utf16Col::new(start_character), source);
    let end_off = line_index.offset_at_utf16(end_line, Utf16Col::new(end_character), source);
    tcl_lsp_core::refactor::extract_variable(source, start_off, end_off, var_name, &line_index)
        .map(|r| refactor_result(source, &r))
}

/// Convert the `expr` command at `(line, character)` to braced form for safety
/// and performance. Returns `None` when the cursor is not on an `expr`, it has
/// no arguments, or it is already braced. Over
/// `tcl_lsp_core::refactor::brace_expr`.
#[pyfunction]
#[pyo3(signature = (source, line, character, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn refactor_brace_expr(
    source: &str,
    line: u32,
    character: u32,
    dialect: &str,
) -> Option<RefactorResult> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let (_line_index, cursor) = cursor_at(source, line, character);
    tcl_lsp_core::refactor::brace_expr(source, cursor, registry)
        .map(|r| refactor_result(source, &r))
}

/// Extract an `if`/`elseif` chain or `switch` with literal values at `(line,
/// character)` into an iRules data-group. `dg_name` is auto-generated when
/// empty. Returns `None` when the cursor is not on a convertible construct.
/// Over `tcl_lsp_core::refactor::extract_to_datagroup`.
#[pyfunction]
#[pyo3(signature = (source, line, character, dg_name = "", *, dialect = "f5-irules"))]
pub(crate) fn refactor_extract_datagroup(
    source: &str,
    line: u32,
    character: u32,
    dg_name: &str,
    dialect: &str,
) -> Option<DataGroupResult> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let (line_index, cursor) = cursor_at(source, line, character);
    let r =
        tcl_lsp_core::refactor::extract_to_datagroup(source, cursor, dg_name, registry, &line_index)?;
    let dg = r.data_group.as_ref()?;
    Some(DataGroupResult {
        title: r.title.clone(),
        rewritten: r.apply(source),
        data_group_definition: tcl_lsp_core::refactor::data_group_tcl(dg),
        name: dg.name.clone(),
        value_type: dg.value_type.clone(),
        records: dg.records.clone(),
        edit_count: r.edits.len(),
    })
}

/// Split `s` into lines, keeping each line's trailing `\n` (like Python
/// `str.splitlines(keepends=True)` over `\n`). Re-joining with `concat()` is
/// loss-free.
fn split_keep_ends(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
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

/// Convert a `serde_json::Value` graph payload into a native Python object
/// (dict / list / scalar) for ergonomic consumption.
fn graph_to_py<'py>(
    py: Python<'py>,
    value: &serde_json::Value,
) -> PyResult<Bound<'py, PyAny>> {
    pythonize::pythonize(py, value)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("serialising graph: {e}")))
}

/// The proc call graph — nodes (one per proc: params, definition line, purity,
/// effect region), caller→callee edges with call-site positions, roots, and
/// leaf procs. Over `tcl_lsp_core::graphs::call_graph`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn call_graph<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    graph_to_py(py, &tcl_lsp_core::graphs::call_graph(source, registry, dialect))
}

/// The symbol relationship graph — scope hierarchy with proc/variable
/// definitions and references, the proc call-site index, package requirements,
/// and a summary. Over `tcl_lsp_core::graphs::symbol_graph`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn symbol_graph<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    graph_to_py(py, &tcl_lsp_core::graphs::symbol_graph(source, dialect))
}

/// The data-flow / taint graph — taint warnings, tainted variables per scope,
/// and per-proc side-effect classification. Over
/// `tcl_lsp_core::graphs::dataflow_graph`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn dataflow_graph<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    graph_to_py(
        py,
        &tcl_lsp_core::graphs::dataflow_graph(source, registry, dialect),
    )
}

/// The control-flow diagram tree — `{events, procedures}` with per-event and
/// per-proc flow nodes (if / switch / loop / action / `proc_call` / …). Over
/// `tcl_lsp_core::diagram::diagram_data`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = "f5-irules"))]
pub(crate) fn diagram_data<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    graph_to_py(py, &tcl_lsp_core::diagram::diagram_data(source, registry))
}

/// Run the optimiser under a named `profile` and return the rewritten source
/// plus the raw (ungrouped) optimisation suggestions. Over
/// `tcl_lsp_core`'s optimiser passes (`tcl_compiler::optimiser`).
///
/// The returned dict has `optimizations` (each `{code, message, range,
/// replacement, group?, hintOnly?}`), `optimized_source`, `changed`,
/// `profile`, `multi_pass`, and `iterations`. Grouping into logical
/// optimisations is presentation glue left to the caller.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT, profile = "full"))]
pub(crate) fn optimise<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
    profile: &str,
) -> PyResult<Bound<'py, PyAny>> {
    use tcl_compiler::optimiser::optimise_source_multipass_filtered;
    use tcl_compiler::optimiser::profiles::{OptimisationProfile, profile_to_disabled};

    let registry = crate::registry::default_registry_for_dialect(dialect);
    let profile = OptimisationProfile::parse(profile);
    let disabled: std::collections::HashSet<String> = profile_to_disabled(profile)
        .into_iter()
        .map(str::to_owned)
        .collect();

    let (optimised, opts, iterations) = optimise_source_multipass_filtered(
        source,
        registry,
        Some(dialect),
        profile.max_iterations(),
        &disabled,
    );

    let line_index = LineIndex::new(source);
    let pos = |offset: u32| {
        let p = line_index.position_at_utf16(offset, source);
        serde_json::json!({ "line": p.line, "character": p.character.get() })
    };
    let items: Vec<serde_json::Value> = opts
        .iter()
        .map(|o| {
            let mut item = serde_json::json!({
                "code": o.code.to_string(),
                "message": o.message,
                "range": { "start": pos(o.span.start()), "end": pos(o.span.end()) },
                "replacement": o.replacement,
            });
            let map = item.as_object_mut().expect("json object");
            if let Some(group) = o.group {
                map.insert("group".to_owned(), serde_json::json!(group));
            }
            if o.hint_only {
                map.insert("hintOnly".to_owned(), serde_json::json!(true));
            }
            item
        })
        .collect();

    let result = serde_json::json!({
        "optimizations": items,
        "optimized_source": optimised,
        "changed": optimised != source,
        "profile": profile.name(),
        "multi_pass": profile.is_multi_pass(),
        "iterations": iterations,
    });
    graph_to_py(py, &result)
}

/// Translate a runtime error message from minified names back to the original
/// symbols, using the `symbol_map` text produced by minification. When both
/// `minified_source` and `original_source` are given, minified line references
/// are remapped first. Over `tcl_lsp_core::minify`.
#[pyfunction]
#[pyo3(signature = (error_message, symbol_map, *, minified_source = "", original_source = ""))]
pub(crate) fn unminify_error(
    error_message: &str,
    symbol_map: &str,
    minified_source: &str,
    original_source: &str,
) -> String {
    let map = tcl_lsp_core::minify::SymbolMap::parse(symbol_map);
    let mut translated = error_message.to_owned();
    if !minified_source.is_empty() && !original_source.is_empty() {
        translated =
            tcl_lsp_core::minify::remap_line_references(&translated, minified_source, original_source);
    }
    tcl_lsp_core::minify::unminify_error(&translated, &map)
}

/// Generate a docstring stub for `proc_name` in `source`, or `None` when the
/// proc is not found. `style` is `"doxygen"` (default) or `"plain"`;
/// `decoration` adds a `# ....` rule above and below. Over
/// `tcl_lsp_core::formatting::generate_stub_for_proc`.
#[pyfunction]
#[pyo3(signature = (source, proc_name, *, style = "doxygen", decoration = false, dialect = DEFAULT_DIALECT))]
pub(crate) fn generate_docstring_stub(
    source: &str,
    proc_name: &str,
    style: &str,
    decoration: bool,
    dialect: &str,
) -> Option<String> {
    let analysis = Analyser::new().analyse(source, dialect);
    // Mirror `AnalysisResult.find_proc`: qualified (`::name`), then as-is, then
    // bare-name match.
    let qualified = format!("::{proc_name}");
    let proc = analysis
        .all_procs
        .values()
        .find(|p| p.qualified_name == qualified || p.qualified_name == proc_name)
        .or_else(|| analysis.all_procs.values().find(|p| p.name == proc_name))?;
    let tag = tcl_lsp_core::formatting::resolve_tag_style(style);
    Some(tcl_lsp_core::formatting::generate_stub_for_proc(
        proc, tag, decoration, '.', 70, "",
    ))
}

/// Parse raw docstring comment text into a structured dict (`{brief?,
/// description?, params?, returns?}`, empty keys omitted). Over
/// `tcl_lsp_core::formatting::parse_docstring`.
#[pyfunction]
#[pyo3(signature = (text, /))]
pub(crate) fn parse_docstring<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyAny>> {
    let info = tcl_lsp_core::formatting::parse_docstring(text);
    graph_to_py(py, &info.to_json())
}

/// The multiplicity category of an iRules event: `"init"`,
/// `"once_per_connection"`, `"per_request"`, or `"unknown"`. Over
/// `tcl_registry::events::EventRegistry::event_multiplicity`.
#[pyfunction]
#[pyo3(signature = (event, /))]
pub(crate) fn event_multiplicity(event: &str) -> String {
    EventRegistry::build().event_multiplicity(event).to_owned()
}

/// Scan the `when EVENT` handlers in `source` and return the distinct event
/// names in canonical firing order. Over
/// `tcl_registry::events::EventRegistry::order_events_for_file`.
#[pyfunction]
#[pyo3(signature = (source, /))]
pub(crate) fn order_events_for_file(source: &str) -> Vec<String> {
    EventRegistry::build().order_events_for_file(source)
}

/// Simulate an iRule under the embedded TMM-sim orchestrator on the bytecode
/// VM and return the pool/node decision, captured logs, and decision log.
///
/// **Runtime-ready, not yet fully runnable:** this drives `tcl-vm` in-process
/// (no `tclsh`), so it lights up as the VM grows the command surface the
/// orchestrator needs. Until then the returned dict carries a non-empty
/// `error` explaining why the run could not complete, with the other fields at
/// their defaults — it never raises for an incomplete VM. Over
/// `tcl_irule_test::simulate_irule`.
///
/// Returns `{pool, node, response_committed, logs, decisions, error}`; each
/// decision is `{category, action, value}`.
#[pyfunction]
#[pyo3(signature = (source, *, method = "GET", uri = "/", host = "", sni = "", profiles = None, pools = None))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn simulate_irule<'py>(
    py: Python<'py>,
    source: &str,
    method: &str,
    uri: &str,
    host: &str,
    sni: &str,
    profiles: Option<Vec<String>>,
    pools: Option<Vec<(String, Vec<String>)>>,
) -> PyResult<Bound<'py, PyAny>> {
    let profiles = profiles.unwrap_or_else(|| vec!["HTTP".to_owned()]);
    let pools = pools.unwrap_or_default();
    let request = tcl_irule_test::SimRequest {
        method: method.to_owned(),
        uri: uri.to_owned(),
        host: host.to_owned(),
        sni: sni.to_owned(),
        headers: Vec::new(),
    };
    let sources = [source.to_owned()];
    // The VM run is CPU-bound and self-contained — release the GIL for it.
    let outcome = py.detach(|| {
        tcl_irule_test::simulate_irule(&sources, &profiles, &pools, Some(&request))
    });
    let decisions: Vec<serde_json::Value> = outcome
        .decisions
        .iter()
        .map(|(category, action, value)| {
            serde_json::json!({ "category": category, "action": action, "value": value })
        })
        .collect();
    let result = serde_json::json!({
        "pool": outcome.pool,
        "node": outcome.node,
        "response_committed": outcome.response_committed,
        "logs": outcome.logs,
        "decisions": decisions,
        "error": outcome.error,
    });
    graph_to_py(py, &result)
}

/// Recursively serialise a document-symbol node to a JSON value.
fn doc_symbol_to_json(sym: &tcl_lsp_core::document_symbols::DocumentSymbol) -> serde_json::Value {
    let range = |r: &tcl_lsp_core::document_symbols::LineRange| {
        serde_json::json!({
            "start": { "line": r.start_line, "character": r.start_character },
            "end": { "line": r.end_line, "character": r.end_character },
        })
    };
    let mut node = serde_json::json!({
        "name": sym.name,
        "kind": sym.kind.as_str(),
        "range": range(&sym.range),
        "selection_range": range(&sym.selection_range),
        "children": sym.children.iter().map(doc_symbol_to_json).collect::<Vec<_>>(),
    });
    if let Some(detail) = &sym.detail {
        node.as_object_mut()
            .expect("json object")
            .insert("detail".to_owned(), serde_json::json!(detail));
    }
    node
}

/// The document-symbol outline tree — nested proc / class / method / namespace
/// / variable entries with LSP `SymbolKind`, outer + selection ranges, and
/// children. Over `tcl_lsp_core::document_symbols::document_symbols`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn document_symbols<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let symbols = tcl_lsp_core::document_symbols::document_symbols(source, dialect);
    let value = serde_json::Value::Array(symbols.iter().map(doc_symbol_to_json).collect());
    graph_to_py(py, &value)
}

/// Structured documentation for every proc in `source`: `name`,
/// `qualified_name`, `params` (`{name, default?}`), the raw + parsed docstring
/// (`doc_raw` / `doc`, `doc` is `null` when undocumented), and inferred
/// `param_traits`. Procs are returned in source (definition) order.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn proc_docs<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let analysis = Analyser::new().analyse(source, dialect);
    let mut procs: Vec<_> = analysis.all_procs.values().collect();
    procs.sort_by_key(|p| p.name_span.start());

    let entries: Vec<serde_json::Value> = procs
        .into_iter()
        .map(|proc| {
            let params: Vec<serde_json::Value> = proc
                .params
                .iter()
                .map(|p| {
                    if p.has_default {
                        serde_json::json!({ "name": p.name, "default": p.default_value })
                    } else {
                        serde_json::json!({ "name": p.name })
                    }
                })
                .collect();
            let mut entry = serde_json::json!({
                "name": proc.name,
                "qualified_name": proc.qualified_name,
                "params": params,
            });
            let map = entry.as_object_mut().expect("json object");
            if proc.doc.is_empty() {
                map.insert("doc".to_owned(), serde_json::Value::Null);
            } else {
                map.insert("doc_raw".to_owned(), serde_json::json!(proc.doc));
                map.insert(
                    "doc".to_owned(),
                    tcl_lsp_core::formatting::parse_docstring(&proc.doc).to_json(),
                );
            }
            if !proc.param_traits.is_empty() {
                let mut traits = serde_json::Map::new();
                for (name, set) in &proc.param_traits {
                    let mut names: Vec<&str> = set.iter().map(|t| t.as_str()).collect();
                    names.sort_unstable();
                    traits.insert(name.clone(), serde_json::json!(names));
                }
                map.insert("param_traits".to_owned(), serde_json::Value::Object(traits));
            }
            entry
        })
        .collect();
    graph_to_py(py, &serde_json::Value::Array(entries))
}

/// Insert a docstring stub above every *undocumented* proc in `source`,
/// returning `{source, documented}` (the rewritten source and the number of
/// procs documented). `style` is `"doxygen"` (default) or `"plain"`;
/// `decoration` adds `# ....` rule lines. Stubs are inserted bottom-up so
/// earlier insertions don't shift later line numbers.
#[pyfunction]
#[pyo3(signature = (source, *, style = "doxygen", decoration = false, dialect = DEFAULT_DIALECT))]
pub(crate) fn insert_docstring_stubs<'py>(
    py: Python<'py>,
    source: &str,
    style: &str,
    decoration: bool,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let analysis = Analyser::new().analyse(source, dialect);
    let line_index = LineIndex::new(source);
    let tag = tcl_lsp_core::formatting::resolve_tag_style(style);

    // Undocumented procs, by descending name-line so bottom-up insertion keeps
    // line offsets valid.
    let mut targets: Vec<_> = analysis
        .all_procs
        .values()
        .filter(|p| p.doc.is_empty())
        .collect();
    targets.sort_by_key(|p| std::cmp::Reverse(line_index.line_at(p.name_span.start())));

    // Split keeping line endings so re-joining is loss-free.
    let mut lines: Vec<String> = split_keep_ends(source);
    for proc in &targets {
        let stub = tcl_lsp_core::formatting::generate_stub_for_proc(
            proc, tag, decoration, '.', 70, "",
        );
        let at = line_index.line_at(proc.name_span.start()) as usize;
        let at = at.min(lines.len());
        lines.insert(at, format!("{stub}\n"));
    }

    let result = serde_json::json!({
        "source": lines.concat(),
        "documented": targets.len(),
    });
    graph_to_py(py, &result)
}

/// Detect the Tcl dialect of a document — the canonical guesser shared by the
/// LSP, editors, CLI tooling, and AI integrations. Applies, in priority order
/// over the first 8 KiB: a `# tcl-dialect:` directive, the `filename`
/// extension, the shebang, content signatures (iRules `when`, F5 tmsh/iApp,
/// EDA-tool commands, `expect`), then `package require Tcl`. Never fails —
/// returns `default` when no heuristic fires. Over
/// `tcl_registry::detect_dialect`.
#[pyfunction]
#[pyo3(signature = (source, *, filename = None, default = DEFAULT_DIALECT))]
pub(crate) fn detect_dialect(source: &str, filename: Option<&str>, default: &str) -> String {
    // `detect_dialect` returns a `&'static str` from `KNOWN_DIALECTS`; the
    // `default` is only echoed back, so a leaked owned copy is unnecessary —
    // fall back to a known static when the caller's default is non-static.
    let fallback = tcl_registry::KNOWN_DIALECTS
        .iter()
        .copied()
        .find(|&d| d == default)
        .unwrap_or(DEFAULT_DIALECT);
    tcl_registry::detect_dialect(source, filename, fallback).to_owned()
}

/// The SSA **def-use chain** graph — per-function def nodes (with use counts,
/// lattice values, types), def→use edges, and memory-SSA alias info. Over
/// `tcl_lsp_core::graphs::def_use_graph`. Returns
/// `{functions: [{name, nodes, edges, aliases, summary}], summary}` with
/// camelCase node/edge keys.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn def_use_chains<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    graph_to_py(py, &tcl_lsp_core::graphs::def_use_graph(source, registry, dialect))
}

/// The memory-SSA **alias** graph — per-function alias sets (variables the
/// analysis proved may refer to the same storage via `upvar` / `global` /
/// `variable`), each with its reason and locations, plus memory-op counts.
/// Over `tcl_lsp_core::graphs::memory_alias_graph`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn memory_aliases<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    graph_to_py(py, &tcl_lsp_core::graphs::memory_alias_graph(source, registry, dialect))
}

/// Recursively walk every command in `source`, including those nested inside
/// body arguments (proc / when / if / … bodies). Returns a list of
/// `{texts, line}` — `texts` is the command's words (name first), `line` is the
/// 0-based start line. Over `tcl_lsp_core::refactor::walk_commands`.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn walk_commands<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let commands: Vec<serde_json::Value> = tcl_lsp_core::refactor::walk_commands(source, registry)
        .into_iter()
        .map(|(texts, line, character)| {
            serde_json::json!({ "texts": texts, "line": line, "character": character })
        })
        .collect();
    graph_to_py(py, &serde_json::Value::Array(commands))
}

/// Compile `source` to a WebAssembly module — the same eval-fallback backend
/// the `tcl compwasm` verb and the explorer `wasm` view use. Returns a
/// [`WasmOutput`] with the `wasm` binary bytes, `wat` text, and counts.
///
/// The backend emits a valid module but is early-tier (unsupported constructs
/// route through host-`eval`); this facade wires the output through so it's
/// available across every surface as the codegen matures.
#[pyfunction]
#[pyo3(signature = (source, *, dialect = DEFAULT_DIALECT))]
pub(crate) fn compile_wasm(source: &str, dialect: &str) -> WasmOutput {
    let registry = crate::registry::default_registry_for_dialect(dialect);
    let ir = tcl_compiler::lowering::lower_to_ir(source, registry);
    let mut wasm = tcl_compiler::codegen::wasm::wasm_codegen_module(&ir, source);
    let function_count = wasm.functions.len();
    // Match the CLI ordering: encode the binary first, then render the WAT.
    let wasm_bytes = wasm.to_bytes();
    let wat = wasm.to_wat();
    WasmOutput {
        wat,
        function_count,
        byte_length: wasm_bytes.len(),
        wasm_bytes,
    }
}

/// Register the public facades on the module.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(compile_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(analyse_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(format_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(parse_bigip_config, m)?)?;
    m.add_function(wrap_pyfunction!(query_bigip, m)?)?;
    m.add_function(wrap_pyfunction!(lookup_event_info, m)?)?;
    m.add_function(wrap_pyfunction!(lookup_command_info, m)?)?;
    m.add_function(wrap_pyfunction!(refactor_inline_variable, m)?)?;
    m.add_function(wrap_pyfunction!(refactor_if_to_switch, m)?)?;
    m.add_function(wrap_pyfunction!(refactor_switch_to_dict, m)?)?;
    m.add_function(wrap_pyfunction!(refactor_extract_variable, m)?)?;
    m.add_function(wrap_pyfunction!(refactor_brace_expr, m)?)?;
    m.add_function(wrap_pyfunction!(refactor_extract_datagroup, m)?)?;
    m.add_function(wrap_pyfunction!(call_graph, m)?)?;
    m.add_function(wrap_pyfunction!(symbol_graph, m)?)?;
    m.add_function(wrap_pyfunction!(dataflow_graph, m)?)?;
    m.add_function(wrap_pyfunction!(diagram_data, m)?)?;
    m.add_function(wrap_pyfunction!(optimise, m)?)?;
    m.add_function(wrap_pyfunction!(unminify_error, m)?)?;
    m.add_function(wrap_pyfunction!(generate_docstring_stub, m)?)?;
    m.add_function(wrap_pyfunction!(parse_docstring, m)?)?;
    m.add_function(wrap_pyfunction!(event_multiplicity, m)?)?;
    m.add_function(wrap_pyfunction!(order_events_for_file, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_irule, m)?)?;
    m.add_function(wrap_pyfunction!(document_symbols, m)?)?;
    m.add_function(wrap_pyfunction!(proc_docs, m)?)?;
    m.add_function(wrap_pyfunction!(insert_docstring_stubs, m)?)?;
    m.add_function(wrap_pyfunction!(detect_dialect, m)?)?;
    m.add_function(wrap_pyfunction!(def_use_chains, m)?)?;
    m.add_function(wrap_pyfunction!(memory_aliases, m)?)?;
    m.add_function(wrap_pyfunction!(walk_commands, m)?)?;
    m.add_function(wrap_pyfunction!(compile_wasm, m)?)?;
    Ok(())
}

#[cfg(test)]
mod registry_info_tests {
    use super::{lookup_command_info, lookup_event_info};

    // Reference values captured from the Python `compiler.registry.info`
    // (`lookup_event_info` / `lookup_command_info`) on the same registry.

    #[test]
    fn event_info_matches_python() {
        let info = lookup_event_info("CLIENT_ACCEPTED", "f5-irules");
        assert!(info.known);
        assert_eq!(info.event, "CLIENT_ACCEPTED");
        assert_eq!(info.multiplicity, "once_per_connection");
        assert_eq!(info.side, "client-side");
        assert_eq!(info.transport.as_deref(), Some("tcp/udp"));
        assert_eq!(info.valid_commands.len(), 1146);

        let unknown = lookup_event_info("BOGUS_EVENT", "f5-irules");
        assert!(!unknown.known);
        assert!(unknown.valid_commands.is_empty());
    }

    #[test]
    fn command_info_matches_python() {
        let info = lookup_command_info("HTTP::respond", "f5-irules");
        assert!(info.found);
        assert_eq!(info.command, "HTTP::respond");
        assert!(info.summary.starts_with("Send an immediate HTTP respons"));
        assert_eq!(info.switches.len(), 5);
        assert_eq!(info.valid_events.len(), 74);
        assert!(!info.valid_in_any_event);

        let missing = lookup_command_info("NoSuchCommand::foo", "f5-irules");
        assert!(!missing.found);
        assert!(missing.valid_events.is_empty());
    }
}

#[cfg(test)]
mod refactor_tests {
    use super::{
        refactor_brace_expr, refactor_extract_datagroup, refactor_extract_variable,
        refactor_if_to_switch, refactor_inline_variable,
    };

    // Reference values captured from the Python `tooling.refactoring`
    // (`inline_variable` / `if_to_switch`) on the same source + cursor.

    #[test]
    fn inline_variable_matches_python() {
        let src = "set x 5\nputs $x\n";
        let r = refactor_inline_variable(src, 0, 4, "tcl9.0").expect("inlinable at cursor");
        assert_eq!(r.title, "Inline variable '$x'");
        assert_eq!(r.rewritten, "puts 5\n");
        assert_eq!(r.edit_count, 2);

        // Off-target: cursor on `puts`, not an inlinable variable.
        assert!(refactor_inline_variable(src, 1, 0, "tcl9.0").is_none());
    }

    #[test]
    fn if_to_switch_matches_python() {
        let src = "if {$x eq \"a\"} {\n  puts one\n} elseif {$x eq \"b\"} {\n  puts two\n}\n";
        let r = refactor_if_to_switch(src, 0, 0, "tcl9.0").expect("convertible chain at cursor");
        assert_eq!(r.title, "Convert to switch on $x");
        assert_eq!(r.edit_count, 1);
        assert!(r.rewritten.starts_with("switch -exact -"));
    }

    #[test]
    fn extract_variable_matches_python() {
        // Select `[expr {1 + 2}]` (chars 5..19 on line 0).
        let src = "puts [expr {1 + 2}]\n";
        let r = refactor_extract_variable(src, 0, 5, 0, 19, "result", "tcl9.0")
            .expect("valid selection");
        assert_eq!(r.title, "Extract into variable '$result'");
        assert_eq!(r.rewritten, "set result [expr {1 + 2}]\nputs $result\n");
        assert_eq!(r.edit_count, 2);
    }

    #[test]
    fn brace_expr_matches_python() {
        let src = "expr \"$a + $b\"\n";
        let r = refactor_brace_expr(src, 0, 0, "tcl9.0").expect("expr at cursor");
        assert_eq!(r.title, "Brace expr for safety and performance");
        assert_eq!(r.rewritten, "expr {$a + $b}\n");
        assert_eq!(r.edit_count, 1);

        assert!(refactor_brace_expr("expr {$a + $b}\n", 0, 0, "tcl9.0").is_none());
    }

    #[test]
    fn extract_datagroup_matches_python() {
        let src = "switch -exact -- $uri {\n    /api { set pool api_pool }\n    /web { set pool web_pool }\n    /cdn { set pool cdn_pool }\n}";
        let r = refactor_extract_datagroup(src, 0, 0, "uri_pool", "f5-irules")
            .expect("convertible switch");
        assert_eq!(r.title, "Extract to data-group 'uri_pool' (string)");
        assert_eq!(r.name, "uri_pool");
        assert_eq!(r.value_type, "string");
        assert_eq!(
            r.records,
            vec![
                ("/api".to_owned(), "api_pool".to_owned()),
                ("/web".to_owned(), "web_pool".to_owned()),
                ("/cdn".to_owned(), "cdn_pool".to_owned()),
            ]
        );
        assert_eq!(r.rewritten, "set pool [class lookup $uri uri_pool]");
        assert_eq!(r.edit_count, 1);
        assert!(r.data_group_definition.starts_with("ltm data-group internal uri_pool {"));
        assert!(r.data_group_definition.contains("type string"));
    }
}
