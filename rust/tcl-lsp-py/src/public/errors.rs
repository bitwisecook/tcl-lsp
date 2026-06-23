//! The public, typed error hierarchy raised by the facade surface.
//!
//! The designed `PyO3` API (see [`crate::public`]) translates every
//! recoverable failure in the underlying pure crates into one of a
//! small, stable set of Python exceptions, all rooted at
//! [`TclLspError`]:
//!
//! ```text
//! Exception
//!  └─ TclLspError                 (base — never raised directly)
//!      ├─ TclParseError           parse_tcl
//!      ├─ TclCompileError         compile_tcl
//!      ├─ TclAnalysisError        analyse_tcl
//!      ├─ BigipParseError         parse_bigip_config
//!      ├─ BigipQueryError         query_bigip
//!      └─ UnsupportedFeatureError any facade, for a recognised-but-
//!                                 unimplemented option
//! ```
//!
//! Every instance the facades raise carries four attributes —
//! `code` (a stable, machine-readable identifier), `message` (the
//! human-readable text, also `str(err)`), `uri` (the document the
//! error anchors to, or `None`), and `range` (a
//! `((start_line, start_char), (end_line, end_char))` tuple, or
//! `None` when no position is available). Downstream embedders match
//! on the exception *type* for coarse handling and read `code` for
//! the precise reason.
//!
//! This is the one place the error vocabulary is owned. The pure
//! crates return `Result<_, E>` with their own error enums; the
//! translation to a Python exception happens here, at the binding
//! boundary.

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::{PyErr, create_exception};

create_exception!(
    tcl_lsp_py,
    TclLspError,
    PyException,
    "Base class for every error raised by the public tcl-lsp facades."
);
create_exception!(
    tcl_lsp_py,
    TclParseError,
    TclLspError,
    "The Tcl lexer rejected the source (strict-quoting syntax error)."
);
create_exception!(
    tcl_lsp_py,
    TclCompileError,
    TclLspError,
    "Lowering the Tcl source to a compilation unit failed."
);
create_exception!(
    tcl_lsp_py,
    TclAnalysisError,
    TclLspError,
    "The semantic analyser could not produce a result for the source."
);
create_exception!(
    tcl_lsp_py,
    BigipParseError,
    TclLspError,
    "The BIG-IP / iApp APL configuration parser rejected the source."
);
create_exception!(
    tcl_lsp_py,
    BigipQueryError,
    TclLspError,
    "Lexing, parsing, evaluating, or rendering a BIG-IP query failed."
);
create_exception!(
    tcl_lsp_py,
    UnsupportedFeatureError,
    TclLspError,
    "A facade was asked for an option it recognises but does not yet implement."
);

/// An `((start_line, start_char), (end_line, end_char))` 0-based
/// position pair, the Python-facing shape of a resolved [`Span`].
///
/// [`Span`]: tcl_lexer::Span
pub(crate) type RangeTuple = ((u32, u32), (u32, u32));

/// Which concrete exception type a [`PublicError`] maps to.
#[derive(Clone, Copy)]
enum Kind {
    Parse,
    Compile,
    Analysis,
    BigipParse,
    BigipQuery,
    Unsupported,
}

/// A facade-boundary error, carrying everything the Python exception
/// needs: its concrete type, a stable code, the message, and the
/// optional document URI / source range.
///
/// Construct one with the named constructors ([`PublicError::parse`],
/// [`PublicError::unsupported`], …), refine it with the
/// [`PublicError::with_uri`] / [`PublicError::with_range`] builders,
/// and hand it to [`PublicError::into_pyerr`] at the `?` boundary.
pub(crate) struct PublicError {
    kind: Kind,
    code: String,
    message: String,
    uri: Option<String>,
    range: Option<RangeTuple>,
}

impl PublicError {
    fn new(kind: Kind, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
            uri: None,
            range: None,
        }
    }

    /// A `TclParseError` — the lexer rejected the source.
    pub(crate) fn parse(message: impl Into<String>) -> Self {
        Self::new(Kind::Parse, "TCL_PARSE", message)
    }

    /// A `TclCompileError` — lowering to a compilation unit failed.
    pub(crate) fn compile(message: impl Into<String>) -> Self {
        Self::new(Kind::Compile, "TCL_COMPILE", message)
    }

    /// A `TclAnalysisError` — the analyser could not produce a result.
    pub(crate) fn analysis(message: impl Into<String>) -> Self {
        Self::new(Kind::Analysis, "TCL_ANALYSIS", message)
    }

    /// A `BigipParseError` — the BIG-IP / APL parser rejected the source.
    pub(crate) fn bigip_parse(message: impl Into<String>) -> Self {
        Self::new(Kind::BigipParse, "BIGIP_PARSE", message)
    }

    /// A `BigipQueryError` with a query-specific `code` sub-identifier.
    pub(crate) fn bigip_query(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Kind::BigipQuery, code, message)
    }

    /// An `UnsupportedFeatureError` — a recognised option is not yet built.
    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::new(Kind::Unsupported, "UNSUPPORTED_FEATURE", message)
    }

    /// Attach the document URI the error anchors to.
    #[must_use]
    pub(crate) fn with_uri(mut self, uri: Option<String>) -> Self {
        self.uri = uri;
        self
    }

    /// Attach the resolved source range the error anchors to.
    #[must_use]
    pub(crate) fn with_range(mut self, range: Option<RangeTuple>) -> Self {
        self.range = range;
        self
    }

    /// Materialise the matching Python exception, with `code`,
    /// `message`, `uri`, and `range` set as instance attributes.
    pub(crate) fn into_pyerr(self, py: Python<'_>) -> PyErr {
        let Self {
            kind,
            code,
            message,
            uri,
            range,
        } = self;
        let args = (message.clone(),);
        let err = match kind {
            Kind::Parse => TclParseError::new_err(args),
            Kind::Compile => TclCompileError::new_err(args),
            Kind::Analysis => TclAnalysisError::new_err(args),
            Kind::BigipParse => BigipParseError::new_err(args),
            Kind::BigipQuery => BigipQueryError::new_err(args),
            Kind::Unsupported => UnsupportedFeatureError::new_err(args),
        };
        // Decorate the freshly-built instance. `setattr` on a brand-new
        // exception value cannot fail, so swallowing the result keeps
        // the signature `-> PyErr` without masking real problems.
        let value = err.value(py).as_any();
        let _ = value.setattr("code", code);
        let _ = value.setattr("message", message);
        let _ = value.setattr("uri", uri);
        let _ = value.setattr("range", range);
        err
    }
}

/// Register the exception hierarchy on the module so Python can both
/// catch (`except tcl_lsp_py.TclParseError`) and introspect them.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("TclLspError", py.get_type::<TclLspError>())?;
    m.add("TclParseError", py.get_type::<TclParseError>())?;
    m.add("TclCompileError", py.get_type::<TclCompileError>())?;
    m.add("TclAnalysisError", py.get_type::<TclAnalysisError>())?;
    m.add("BigipParseError", py.get_type::<BigipParseError>())?;
    m.add("BigipQueryError", py.get_type::<BigipQueryError>())?;
    m.add(
        "UnsupportedFeatureError",
        py.get_type::<UnsupportedFeatureError>(),
    )?;
    Ok(())
}
