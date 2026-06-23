//! Structured result types returned by the public facades.
//!
//! Every facade returns a small, frozen `#[pyclass]` (or a `str`)
//! rather than an `Any`-shaped dict, so downstream embedders get a
//! stable, typed surface they can introspect and that survives the
//! eventual retirement of the legacy soft-dependency shims. Each
//! positional field is exposed as a resolved
//! `(line, character)` pair — the facade resolves [`Span`]s against a
//! [`SourceMap`] at the boundary, so Python never sees a bare byte
//! offset it would have to resolve itself.
//!
//! [`Span`]: tcl_lexer::Span
//! [`SourceMap`]: tcl_lexer::SourceMap

use pyo3::prelude::*;

use tcl_compiler::analyser::Diagnostic as CoreDiagnostic;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{SourceMap, Token};

use super::errors::RangeTuple;

/// Resolve a token/segment/diagnostic span to the Python-facing
/// `((start_line, start_char), (end_line, end_char))` shape.
pub(crate) fn range_of(sm: &SourceMap<'_>, span: tcl_lexer::Span) -> RangeTuple {
    let (start, end) = sm.range_positions(span);
    ((start.line, start.character), (end.line, end.character))
}

/// One lexer token: its kind name, source text, and position.
#[pyclass(frozen, module = "tcl_lsp_py", name = "LexToken")]
pub(crate) struct LexToken {
    /// The token kind: `"VAR"`, `"CMD"`, `"STR"`, `"SEP"`, `"EOL"`,
    /// `"EOF"`, `"COMMENT"`, `"ESC"`, or `"EXPAND"`.
    #[pyo3(get)]
    kind: String,
    /// The exact source text the token spans.
    #[pyo3(get)]
    text: String,
    /// 0-based `(line, character)` of the token start.
    #[pyo3(get)]
    start: (u32, u32),
    /// 0-based `(line, character)` of the token end (exclusive).
    #[pyo3(get)]
    end: (u32, u32),
    /// `(start, end)` byte offsets into the source.
    #[pyo3(get)]
    byte_range: (u32, u32),
}

#[pymethods]
impl LexToken {
    fn __repr__(&self) -> String {
        format!(
            "LexToken(kind={:?}, text={:?}, start={:?})",
            self.kind, self.text, self.start
        )
    }
}

impl LexToken {
    /// Build a [`LexToken`] from a core [`Token`], resolving its span.
    pub(crate) fn from_token(
        py: Python<'_>,
        sm: &SourceMap<'_>,
        tok: &Token,
    ) -> PyResult<Py<LexToken>> {
        let (start, end) = range_of(sm, tok.span);
        Py::new(
            py,
            LexToken {
                kind: tok.kind.name().to_owned(),
                text: sm.text(tok.span).to_owned(),
                start,
                end,
                byte_range: (tok.span.start(), tok.span.end()),
            },
        )
    }
}

/// One top-level command from the segmenter: its name, literal
/// arguments, and source range.
#[pyclass(frozen, module = "tcl_lsp_py", name = "ParsedCommand")]
pub(crate) struct ParsedCommand {
    /// The command word (the first word of the command).
    #[pyo3(get)]
    name: String,
    /// The argument words, in source order. Words with substitutions
    /// appear with their substitution markers intact.
    #[pyo3(get)]
    args: Vec<String>,
    /// 0-based `(line, character)` of the command start.
    #[pyo3(get)]
    start: (u32, u32),
    /// 0-based `(line, character)` of the command end (exclusive).
    #[pyo3(get)]
    end: (u32, u32),
}

#[pymethods]
impl ParsedCommand {
    fn __repr__(&self) -> String {
        format!(
            "ParsedCommand(name={:?}, args={}, start={:?})",
            self.name,
            self.args.len(),
            self.start
        )
    }
}

impl ParsedCommand {
    /// Build a [`ParsedCommand`] from a [`SegmentedCommand`].
    pub(crate) fn from_segment(
        py: Python<'_>,
        sm: &SourceMap<'_>,
        cmd: &SegmentedCommand,
    ) -> PyResult<Py<ParsedCommand>> {
        let (start, end) = range_of(sm, cmd.span);
        Py::new(
            py,
            ParsedCommand {
                name: cmd.name().to_owned(),
                args: cmd.args().to_vec(),
                start,
                end,
            },
        )
    }
}

/// One analyser diagnostic: a stable code, message, severity, and the
/// source range it anchors to.
#[pyclass(frozen, module = "tcl_lsp_py", name = "Diagnostic")]
pub(crate) struct Diagnostic {
    /// The stable diagnostic code (e.g. `"W101"`, `"IRULE5002"`).
    #[pyo3(get)]
    code: String,
    /// The one-line, user-facing message.
    #[pyo3(get)]
    message: String,
    /// `"error"`, `"warning"`, or `"hint"`.
    #[pyo3(get)]
    severity: String,
    /// 0-based `(line, character)` of the diagnostic start.
    #[pyo3(get)]
    start: (u32, u32),
    /// 0-based `(line, character)` of the diagnostic end (exclusive).
    #[pyo3(get)]
    end: (u32, u32),
}

#[pymethods]
impl Diagnostic {
    fn __repr__(&self) -> String {
        format!(
            "Diagnostic(code={:?}, severity={:?}, start={:?})",
            self.code, self.severity, self.start
        )
    }
}

impl Diagnostic {
    /// Build a [`Diagnostic`] from the analyser's core diagnostic,
    /// resolving its span against `sm`.
    pub(crate) fn from_core(
        py: Python<'_>,
        sm: &SourceMap<'_>,
        d: &CoreDiagnostic,
    ) -> PyResult<Py<Diagnostic>> {
        let (start, end) = range_of(sm, d.span);
        Py::new(
            py,
            Diagnostic {
                code: d.code.clone(),
                message: d.message.clone(),
                severity: d.severity.as_str().to_owned(),
                start,
                end,
            },
        )
    }
}

/// The result of [`parse_tcl`](super::facades::parse_tcl): the token
/// stream, the top-level command structure, and any lexer warnings.
#[pyclass(frozen, module = "tcl_lsp_py", name = "ParseResult")]
pub(crate) struct ParseResult {
    /// Every token in the source, in order, including the trailing EOF.
    #[pyo3(get)]
    pub(crate) tokens: Vec<Py<LexToken>>,
    /// The top-level commands (the segmenter's view of the source).
    #[pyo3(get)]
    pub(crate) commands: Vec<Py<ParsedCommand>>,
    /// Non-fatal lexer warnings, as human-readable strings.
    #[pyo3(get)]
    pub(crate) warnings: Vec<String>,
}

#[pymethods]
impl ParseResult {
    fn __repr__(&self) -> String {
        format!(
            "ParseResult(tokens={}, commands={}, warnings={})",
            self.tokens.len(),
            self.commands.len(),
            self.warnings.len()
        )
    }
}

/// The result of [`analyse_tcl`](super::facades::analyse_tcl): the
/// diagnostics plus the discovered top-level symbols.
#[pyclass(frozen, module = "tcl_lsp_py", name = "AnalysisResult")]
pub(crate) struct AnalysisResult {
    /// Every diagnostic the analyser emitted, in emission order.
    #[pyo3(get)]
    pub(crate) diagnostics: Vec<Py<Diagnostic>>,
    /// Qualified names of every discovered procedure, sorted.
    #[pyo3(get)]
    pub(crate) procs: Vec<String>,
    /// Qualified names of every discovered class, sorted.
    #[pyo3(get)]
    pub(crate) classes: Vec<String>,
    /// Qualified names of every tracked variable, sorted.
    #[pyo3(get)]
    pub(crate) variables: Vec<String>,
}

#[pymethods]
impl AnalysisResult {
    fn __repr__(&self) -> String {
        format!(
            "AnalysisResult(diagnostics={}, procs={}, classes={}, variables={})",
            self.diagnostics.len(),
            self.procs.len(),
            self.classes.len(),
            self.variables.len()
        )
    }
}

/// The result of
/// [`parse_bigip_config`](super::facades::parse_bigip_config): the
/// parsed configuration, summarised, plus the canonical JSON document.
#[pyclass(frozen, module = "tcl_lsp_py", name = "BigipConfig")]
pub(crate) struct BigipConfig {
    /// The partition short-name bare objects were rewritten under.
    #[pyo3(get)]
    pub(crate) default_partition: String,
    /// The number of top-level objects (stanzas) parsed.
    #[pyo3(get)]
    pub(crate) object_count: usize,
    /// The `"<module>::<type>::<id>"` key of every parsed object, in
    /// source order.
    #[pyo3(get)]
    pub(crate) object_keys: Vec<String>,
    /// The canonical JSON document, compact.
    pub(crate) json: String,
}

#[pymethods]
impl BigipConfig {
    /// Return the canonical JSON document (compact) for this config.
    fn to_json(&self) -> &str {
        &self.json
    }

    fn __repr__(&self) -> String {
        format!(
            "BigipConfig(partition={:?}, objects={})",
            self.default_partition, self.object_count
        )
    }
}

/// One file's rendered output from a read-only
/// [`query_bigip`](super::facades::query_bigip).
#[pyclass(frozen, module = "tcl_lsp_py", name = "QueryFile")]
pub(crate) struct QueryFile {
    /// The source URI this output was produced from.
    #[pyo3(get)]
    pub(crate) uri: String,
    /// The rendered query output, in the requested output mode.
    #[pyo3(get)]
    pub(crate) output: String,
}

/// One file's rewritten source from a mutating
/// [`query_bigip`](super::facades::query_bigip).
#[pyclass(frozen, module = "tcl_lsp_py", name = "QueryEdit")]
pub(crate) struct QueryEdit {
    /// The source URI this edit applies to.
    #[pyo3(get)]
    pub(crate) uri: String,
    /// The rewritten configuration source.
    #[pyo3(get)]
    pub(crate) new_source: String,
    /// Whether the rewrite actually changed the source.
    #[pyo3(get)]
    pub(crate) changed: bool,
}

/// The result of [`query_bigip`](super::facades::query_bigip).
///
/// A read-only query populates `values`; a mutating query (one that
/// assigns to a path) sets `has_mutation` and populates `edits`.
#[pyclass(frozen, module = "tcl_lsp_py", name = "QueryResult")]
pub(crate) struct QueryResult {
    /// Whether the query attempted any mutation.
    #[pyo3(get)]
    pub(crate) has_mutation: bool,
    /// Per-file rendered output for a read-only query (empty for a
    /// mutating query).
    #[pyo3(get)]
    pub(crate) values: Vec<Py<QueryFile>>,
    /// Per-file rewritten source for a mutating query (empty for a
    /// read-only query).
    #[pyo3(get)]
    pub(crate) edits: Vec<Py<QueryEdit>>,
}

#[pymethods]
impl QueryResult {
    fn __repr__(&self) -> String {
        format!(
            "QueryResult(has_mutation={}, values={}, edits={})",
            self.has_mutation,
            self.values.len(),
            self.edits.len()
        )
    }
}

/// Register every public result class on the module.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<LexToken>()?;
    m.add_class::<ParsedCommand>()?;
    m.add_class::<Diagnostic>()?;
    m.add_class::<ParseResult>()?;
    m.add_class::<AnalysisResult>()?;
    m.add_class::<BigipConfig>()?;
    m.add_class::<QueryFile>()?;
    m.add_class::<QueryEdit>()?;
    m.add_class::<QueryResult>()?;
    Ok(())
}
