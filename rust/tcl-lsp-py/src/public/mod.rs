//! The designed public `PyO3` API surface.
//!
//! This module is the terminal
//! public API: a small, semver-stable set of narrow facades —
//! `source/bytes/options in, structured result out` — over the layered
//! Rust crates, paired with a typed error hierarchy. It is deliberately
//! **not** a re-export of the whole crate graph (the broader
//! per-subsystem surface lives in the sibling binding modules).
//!
//! The surface:
//!
//! | Facade | Returns | Over |
//! |---|---|---|
//! | [`parse_tcl`] | [`ParseResult`] | `tcl-lexer` + segmenter |
//! | [`compile_tcl`] | [`CompilationUnit`] | `tcl-compiler` |
//! | [`analyse_tcl`] | [`AnalysisResult`] | `tcl-compiler::analyser` |
//! | [`format_tcl`] | `str` | `tcl-lsp-core::formatting` |
//! | [`parse_bigip_config`] | [`BigipConfig`] | `tcl-bigip` |
//! | [`query_bigip`] | [`QueryResult`] | `tcl-bigip-query` |
//!
//! paired with [`TclLspError`] and its six subclasses
//! ([`errors`]). Every facade resolves spans to `(line, character)`
//! positions and maps recoverable failures to the typed exceptions at
//! this boundary, keeping the pure crates `pyo3`-free.
//!
//! [`parse_tcl`]: facades::parse_tcl
//! [`compile_tcl`]: facades::compile_tcl
//! [`analyse_tcl`]: facades::analyse_tcl
//! [`format_tcl`]: facades::format_tcl
//! [`parse_bigip_config`]: facades::parse_bigip_config
//! [`query_bigip`]: facades::query_bigip
//! [`ParseResult`]: results::ParseResult
//! [`AnalysisResult`]: results::AnalysisResult
//! [`BigipConfig`]: results::BigipConfig
//! [`QueryResult`]: results::QueryResult
//! [`CompilationUnit`]: crate::compilation_unit::CompilationUnitHandle
//! [`TclLspError`]: errors::TclLspError

use pyo3::prelude::*;

pub(crate) mod errors;
pub(crate) mod facades;
pub(crate) mod options;
pub(crate) mod results;

/// Register the whole public surface — errors, options, result types,
/// and the six facades — on the Python module.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::register_with(m)?;
    options::register_with(m)?;
    results::register_with(m)?;
    facades::register_with(m)?;
    Ok(())
}
