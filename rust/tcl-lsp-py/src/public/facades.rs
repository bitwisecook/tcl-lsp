//! The six public facades — `source/options in, structured result out`.
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
use tcl_lexer::{Lexer, LexerConfig, SourceMap};
use tcl_lsp_core::formatting::FormatterConfig;

use super::errors::PublicError;
use super::options::FormatOptions;
use super::results::{
    AnalysisResult, BigipConfig, Diagnostic, LexToken, ParseResult, ParsedCommand, QueryEdit,
    QueryFile, QueryResult,
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
        Some(((pos.line, pos.character), (pos.line, pos.character)))
    } else {
        None
    };
    PublicError::bigip_query(code, e.to_string())
        .with_range(range)
        .into_pyerr(py)
}

/// Register the six facades on the module.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(compile_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(analyse_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(format_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(parse_bigip_config, m)?)?;
    m.add_function(wrap_pyfunction!(query_bigip, m)?)?;
    Ok(())
}
