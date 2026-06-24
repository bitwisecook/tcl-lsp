//! `PyO3` wrappers for the `tcl_lexer` token types.
//!
//! These types expose the lexer token data to Python. The wrappers are
//! deliberately the only place where Python compatibility leaks in: the
//! underlying [`tcl_lexer`] crate stays Python-agnostic, and Rust callers
//! consume those types directly.
//!
//! The wrappers own their text (via `String`) because Python objects can
//! outlive the source buffer they were lexed from, and the source buffer
//! itself is a Python `str` whose lifetime we cannot model in Rust.

use pyo3::prelude::*;

use tcl_lexer::{ByteCol, SourcePosition as CoreSourcePosition, TokenType as CoreTokenType};

/// `enum.Enum`-style token kind exposed to Python as
/// `tcl_lsp_py.TokenType` (also reachable via the
/// `tcl_lsp_rust.TokenType` alias).
///
/// Variants carry explicit 1-indexed discriminants so `TokenType.X.value`
/// stays stable for any external scripts that
/// happened to read it.
#[pyclass(
    name = "TokenType",
    eq,
    hash,
    frozen,
    module = "tcl_lsp_py",
    from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyTokenType {
    /// Plain string fragment.
    #[pyo3(name = "ESC")]
    Esc = 1,
    /// Braced string `{…}`.
    #[pyo3(name = "STR")]
    Str = 2,
    /// Command substitution `[…]`.
    #[pyo3(name = "CMD")]
    Cmd = 3,
    /// Variable substitution `$name`.
    #[pyo3(name = "VAR")]
    Var = 4,
    /// Whitespace separator.
    #[pyo3(name = "SEP")]
    Sep = 5,
    /// End-of-line / `;`.
    #[pyo3(name = "EOL")]
    Eol = 6,
    /// End-of-input.
    #[pyo3(name = "EOF")]
    Eof = 7,
    /// Comment from `#` to end of line.
    #[pyo3(name = "COMMENT")]
    Comment = 8,
    /// `{*}` expansion prefix.
    #[pyo3(name = "EXPAND")]
    Expand = 9,
}

// `&self` on `Copy` `pyclass` enums is required by `PyO3`, so we can't
// take `self` by value here even though clippy::trivially_copy_pass_by_ref
// would prefer it.
#[allow(clippy::trivially_copy_pass_by_ref)]
#[pymethods]
impl PyTokenType {
    /// Symbolic name of the variant — e.g. `"ESC"` for `TokenType.ESC`.
    /// Mirrors `enum.Enum.name`.
    #[getter]
    fn name(&self) -> &'static str {
        CoreTokenType::from(*self).name()
    }

    /// Numeric discriminant.
    #[getter]
    fn value(&self) -> u32 {
        *self as u32
    }

    fn __repr__(&self) -> String {
        format!("<TokenType.{}: {}>", self.name(), self.value())
    }

    fn __str__(&self) -> String {
        format!("TokenType.{}", self.name())
    }
}

impl From<CoreTokenType> for PyTokenType {
    fn from(t: CoreTokenType) -> Self {
        match t {
            CoreTokenType::Esc => Self::Esc,
            CoreTokenType::Str => Self::Str,
            CoreTokenType::Cmd => Self::Cmd,
            CoreTokenType::Var => Self::Var,
            CoreTokenType::Sep => Self::Sep,
            CoreTokenType::Eol => Self::Eol,
            CoreTokenType::Eof => Self::Eof,
            CoreTokenType::Comment => Self::Comment,
            CoreTokenType::Expand => Self::Expand,
        }
    }
}

impl From<PyTokenType> for CoreTokenType {
    fn from(t: PyTokenType) -> Self {
        match t {
            PyTokenType::Esc => Self::Esc,
            PyTokenType::Str => Self::Str,
            PyTokenType::Cmd => Self::Cmd,
            PyTokenType::Var => Self::Var,
            PyTokenType::Sep => Self::Sep,
            PyTokenType::Eol => Self::Eol,
            PyTokenType::Eof => Self::Eof,
            PyTokenType::Comment => Self::Comment,
            PyTokenType::Expand => Self::Expand,
        }
    }
}

/// A position in source text. Exposed to Python as
/// `tcl_lsp_py.SourcePosition` (also reachable via the
/// `tcl_lsp_rust.SourcePosition` alias).
#[pyclass(
    name = "SourcePosition",
    eq,
    hash,
    frozen,
    module = "tcl_lsp_py",
    from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PySourcePosition {
    inner: CoreSourcePosition,
}

impl PySourcePosition {
    /// Construct from the pure-Rust core type. Used by other binding
    /// modules (e.g. the future lexer wrapper) when crossing the FFI
    /// boundary.
    #[must_use]
    pub const fn from_core(inner: CoreSourcePosition) -> Self {
        Self { inner }
    }

    /// Borrow the wrapped pure-Rust `SourcePosition`.
    #[must_use]
    pub const fn as_core(&self) -> &CoreSourcePosition {
        &self.inner
    }
}

#[pymethods]
impl PySourcePosition {
    #[new]
    #[pyo3(signature = (line, character, offset))]
    fn new(line: u32, character: u32, offset: u32) -> Self {
        Self {
            inner: CoreSourcePosition::new(line, ByteCol::new(character), offset),
        }
    }

    #[getter]
    fn line(&self) -> u32 {
        self.inner.line
    }

    #[getter]
    fn character(&self) -> u32 {
        self.inner.character.get()
    }

    #[getter]
    fn offset(&self) -> u32 {
        self.inner.offset
    }

    fn __repr__(&self) -> String {
        format!(
            "SourcePosition(line={}, character={}, offset={})",
            self.inner.line, self.inner.character, self.inner.offset
        )
    }
}

/// A token: kind, text, source range, quoting context. Exposed to Python
/// as `tcl_lsp_py.Token` (also reachable via the
/// `tcl_lsp_rust.Token` alias).
///
/// Owns its text as `String` and is immutable from Python — the type is
/// `#[pyclass(frozen)]`.
#[pyclass(
    name = "Token",
    eq,
    hash,
    frozen,
    module = "tcl_lsp_py",
    from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PyToken {
    kind: PyTokenType,
    text: String,
    start: PySourcePosition,
    end: PySourcePosition,
    in_quote: bool,
}

impl PyToken {
    /// Rust-side constructor used by other binding modules (notably
    /// `crate::lexer`) when lifting a pure-Rust `tcl_lexer::Token<'_>`
    /// into a Python-visible `PyToken`. Separate from `#[new]` so the
    /// signature stays Rust-friendly; `__init__` is a thin
    /// wrapper that delegates here.
    #[must_use]
    pub fn new_from_core(
        kind: PyTokenType,
        text: String,
        start: PySourcePosition,
        end: PySourcePosition,
        in_quote: bool,
    ) -> Self {
        Self {
            kind,
            text,
            start,
            end,
            in_quote,
        }
    }
}

#[pymethods]
impl PyToken {
    #[new]
    #[pyo3(signature = (r#type, text, start, end, in_quote = false))]
    fn new(
        r#type: PyTokenType,
        text: String,
        start: PySourcePosition,
        end: PySourcePosition,
        in_quote: bool,
    ) -> Self {
        Self {
            kind: r#type,
            text,
            start,
            end,
            in_quote,
        }
    }

    /// Token kind. Exposed as `tok.type` for Python compatibility — the
    /// Rust field is named `kind` because `type` is a reserved keyword.
    ///
    /// The getter resolves the variant via a class-attribute lookup on
    /// `TokenType` class so callers get the cached singleton
    /// (`tok.type is TokenType.ESC` is `True`). Callers rely on
    /// identity comparison; if this getter returned a fresh
    /// `PyTokenType` value instead they would all silently start
    /// failing.
    #[getter]
    #[pyo3(name = "type")]
    fn token_type<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let class = py.get_type::<PyTokenType>();
        class.getattr(self.kind.name())
    }

    #[getter]
    fn text(&self) -> &str {
        &self.text
    }

    #[getter]
    fn start(&self) -> PySourcePosition {
        self.start
    }

    #[getter]
    fn end(&self) -> PySourcePosition {
        self.end
    }

    #[getter]
    fn in_quote(&self) -> bool {
        self.in_quote
    }

    fn __repr__(&self) -> String {
        format!(
            "Token(type={}, text={:?}, start={}, end={}, in_quote={})",
            self.kind.__str__(),
            self.text,
            self.start.__repr__(),
            self.end.__repr__(),
            if self.in_quote { "True" } else { "False" }
        )
    }
}

/// Register the token types with a `#[pymodule]`.
///
/// Called from `lib.rs` so the registration list stays in one place:
/// as more domain types arrive, they all get one
/// `register_with(m)` call.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTokenType>()?;
    m.add_class::<PySourcePosition>()?;
    m.add_class::<PyToken>()?;
    Ok(())
}
