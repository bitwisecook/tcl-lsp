//! Typed options objects for the public facades.
//!
//! The facades take their primary knobs as keyword arguments, but the
//! formatter has a wide configuration surface, so it gets a dedicated
//! [`FormatOptions`] bag. Constructing one with no arguments yields the
//! formatter defaults; each keyword overrides exactly one field.

use pyo3::prelude::*;

use tcl_lsp_core::formatting::{FormatterConfig, IndentStyle};

use super::errors::PublicError;

/// Formatter configuration for
/// [`format_tcl`](super::facades::format_tcl).
///
/// Exposes the commonly-tuned subset of the engine's
/// [`FormatterConfig`]; every other field stays at its default. Pass an
/// instance as `format_tcl(source, options=FormatOptions(indent_size=2))`.
#[pyclass(module = "tcl_lsp_py", name = "FormatOptions")]
pub(crate) struct FormatOptions {
    pub(crate) config: FormatterConfig,
}

#[pymethods]
impl FormatOptions {
    /// Build a [`FormatOptions`], overriding the formatter defaults
    /// with any supplied keyword.
    #[new]
    #[pyo3(signature = (
        *,
        indent_size = None,
        indent_style = None,
        continuation_indent = None,
        max_line_length = None,
        space_between_braces = None,
        trim_trailing_whitespace = None,
        ensure_final_newline = None,
        line_ending = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        indent_size: Option<usize>,
        indent_style: Option<String>,
        continuation_indent: Option<usize>,
        max_line_length: Option<usize>,
        space_between_braces: Option<bool>,
        trim_trailing_whitespace: Option<bool>,
        ensure_final_newline: Option<bool>,
        line_ending: Option<String>,
    ) -> PyResult<Self> {
        let mut config = FormatterConfig::default();
        if let Some(v) = indent_size {
            config.indent_size = v;
        }
        if let Some(v) = indent_style {
            config.indent_style = match v.as_str() {
                "space" | "spaces" => IndentStyle::Spaces,
                "tab" | "tabs" => IndentStyle::Tabs,
                other => {
                    return Err(PublicError::unsupported(format!(
                        "unknown indent_style {other:?}; expected \"spaces\" or \"tabs\""
                    ))
                    .into_pyerr(py));
                }
            };
        }
        if let Some(v) = continuation_indent {
            config.continuation_indent = v;
        }
        if let Some(v) = max_line_length {
            config.max_line_length = v;
        }
        if let Some(v) = space_between_braces {
            config.space_between_braces = v;
        }
        if let Some(v) = trim_trailing_whitespace {
            config.trim_trailing_whitespace = v;
        }
        if let Some(v) = ensure_final_newline {
            config.ensure_final_newline = v;
        }
        if let Some(v) = line_ending {
            config.line_ending = v;
        }
        Ok(FormatOptions { config })
    }

    /// The configured indent width.
    #[getter]
    fn indent_size(&self) -> usize {
        self.config.indent_size
    }

    /// The configured maximum line length.
    #[getter]
    fn max_line_length(&self) -> usize {
        self.config.max_line_length
    }

    fn __repr__(&self) -> String {
        format!(
            "FormatOptions(indent_size={}, max_line_length={})",
            self.config.indent_size, self.config.max_line_length
        )
    }
}

/// Register the options classes on the module.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FormatOptions>()?;
    Ok(())
}
