#![allow(clippy::useless_conversion)]
//! `PyO3` wrapper for `tcl_lexer::expr_lexer`.

use pyo3::prelude::*;

use tcl_lexer::{
    tokenise_expr as core_tokenise, ExprToken as CoreExprToken, ExprTokenType as CoreExprTokenType,
};

#[pyclass(name = "ExprTokenType", eq, hash, frozen, module = "tcl_lsp_py")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyExprTokenType {
    #[pyo3(name = "NUMBER")]
    Number = 1,
    #[pyo3(name = "STRING")]
    String = 2,
    #[pyo3(name = "VARIABLE")]
    Variable = 3,
    #[pyo3(name = "COMMAND")]
    Command = 4,
    #[pyo3(name = "OPERATOR")]
    Operator = 5,
    #[pyo3(name = "PAREN_OPEN")]
    ParenOpen = 6,
    #[pyo3(name = "PAREN_CLOSE")]
    ParenClose = 7,
    #[pyo3(name = "COMMA")]
    Comma = 8,
    #[pyo3(name = "FUNCTION")]
    Function = 9,
    #[pyo3(name = "BOOL")]
    Bool = 10,
    #[pyo3(name = "TERNARY_Q")]
    TernaryQ = 11,
    #[pyo3(name = "TERNARY_C")]
    TernaryC = 12,
    #[pyo3(name = "WHITESPACE")]
    Whitespace = 13,
    #[pyo3(name = "EOF")]
    Eof = 14,
}

// `&self` on `Copy` `pyclass` enums is required by `PyO3`, so we can't
// take `self` by value here even though clippy::trivially_copy_pass_by_ref
// would prefer it.
#[allow(clippy::trivially_copy_pass_by_ref)]
#[pymethods]
impl PyExprTokenType {
    #[getter]
    fn name(&self) -> &'static str {
        CoreExprTokenType::from(*self).name()
    }

    #[getter]
    fn value(&self) -> u32 {
        *self as u32
    }

    fn __repr__(&self) -> String {
        format!("<ExprTokenType.{}: {}>", self.name(), self.value())
    }
}

impl From<CoreExprTokenType> for PyExprTokenType {
    fn from(t: CoreExprTokenType) -> Self {
        match t {
            CoreExprTokenType::Number => Self::Number,
            CoreExprTokenType::String => Self::String,
            CoreExprTokenType::Variable => Self::Variable,
            CoreExprTokenType::Command => Self::Command,
            CoreExprTokenType::Operator => Self::Operator,
            CoreExprTokenType::ParenOpen => Self::ParenOpen,
            CoreExprTokenType::ParenClose => Self::ParenClose,
            CoreExprTokenType::Comma => Self::Comma,
            CoreExprTokenType::Function => Self::Function,
            CoreExprTokenType::Bool => Self::Bool,
            CoreExprTokenType::TernaryQ => Self::TernaryQ,
            CoreExprTokenType::TernaryC => Self::TernaryC,
            CoreExprTokenType::Whitespace => Self::Whitespace,
            CoreExprTokenType::Eof => Self::Eof,
        }
    }
}

impl From<PyExprTokenType> for CoreExprTokenType {
    fn from(t: PyExprTokenType) -> Self {
        match t {
            PyExprTokenType::Number => Self::Number,
            PyExprTokenType::String => Self::String,
            PyExprTokenType::Variable => Self::Variable,
            PyExprTokenType::Command => Self::Command,
            PyExprTokenType::Operator => Self::Operator,
            PyExprTokenType::ParenOpen => Self::ParenOpen,
            PyExprTokenType::ParenClose => Self::ParenClose,
            PyExprTokenType::Comma => Self::Comma,
            PyExprTokenType::Function => Self::Function,
            PyExprTokenType::Bool => Self::Bool,
            PyExprTokenType::TernaryQ => Self::TernaryQ,
            PyExprTokenType::TernaryC => Self::TernaryC,
            PyExprTokenType::Whitespace => Self::Whitespace,
            PyExprTokenType::Eof => Self::Eof,
        }
    }
}

#[pyclass(name = "ExprToken", eq, hash, frozen, module = "tcl_lsp_py")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PyExprToken {
    kind: PyExprTokenType,
    text: String,
    start_offset: u32,
    end_offset: u32,
}

#[pymethods]
impl PyExprToken {
    #[getter]
    #[pyo3(name = "type")]
    fn token_type<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let class = py.get_type_bound::<PyExprTokenType>();
        class.getattr(self.kind.name())
    }

    #[getter]
    fn text(&self) -> &str {
        &self.text
    }

    #[getter]
    fn start(&self) -> u32 {
        self.start_offset
    }

    #[getter]
    fn end(&self) -> u32 {
        self.end_offset
    }
}

fn lift(tok: CoreExprToken) -> PyExprToken {
    PyExprToken {
        kind: PyExprTokenType::from(tok.kind),
        text: tok.text,
        start_offset: tok.start,
        end_offset: tok.end,
    }
}

#[pyfunction]
#[pyo3(signature = (source, /, dialect=None))]
pub fn expr_tokenise(source: &str, dialect: Option<&str>) -> Vec<PyExprToken> {
    core_tokenise(source, dialect)
        .into_iter()
        .map(lift)
        .collect()
}

#[pyfunction]
#[pyo3(signature = (source, /, dialect=None))]
pub fn expr_tokenise_checked(source: &str, dialect: Option<&str>) -> (Vec<PyExprToken>, bool) {
    let (tokens, has_unknown) = tcl_lexer::tokenise_expr_checked(source, dialect);
    (tokens.into_iter().map(lift).collect(), has_unknown)
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExprTokenType>()?;
    m.add_class::<PyExprToken>()?;
    m.add_function(wrap_pyfunction!(expr_tokenise, m)?)?;
    m.add_function(wrap_pyfunction!(expr_tokenise_checked, m)?)?;
    Ok(())
}
