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
//! | `lookup_event_info` | `EventInfo` | `tcl-registry` |
//! | `lookup_command_info` | `CommandInfo` | `tcl-registry` |
//! | `refactor_inline_variable` | `RefactorResult` | `tcl-lsp-core::refactor` |
//! | `refactor_if_to_switch` | `RefactorResult` | `tcl-lsp-core::refactor` |
//! | `refactor_switch_to_dict` | `RefactorResult` | `tcl-lsp-core::refactor` |
//! | `refactor_extract_variable` | `RefactorResult` | `tcl-lsp-core::refactor` |
//! | `refactor_brace_expr` | `RefactorResult` | `tcl-lsp-core::refactor` |
//! | `refactor_extract_datagroup` | `DataGroupResult` | `tcl-lsp-core::refactor` |
//! | `call_graph` | `dict` | `tcl-lsp-core::graphs` |
//! | `symbol_graph` | `dict` | `tcl-lsp-core::graphs` |
//! | `dataflow_graph` | `dict` | `tcl-lsp-core::graphs` |
//! | `diagram_data` | `dict` | `tcl-lsp-core::diagram` |
//! | `optimise` | `dict` | `tcl-compiler::optimiser` |
//! | `unminify_error` | `str` | `tcl-lsp-core::minify` |
//! | `generate_docstring_stub` | `str?` | `tcl-lsp-core::formatting` |
//! | `parse_docstring` | `dict` | `tcl-lsp-core::formatting` |
//! | `event_multiplicity` | `str` | `tcl-registry::events` |
//! | `order_events_for_file` | `list[str]` | `tcl-registry::events` |
//! | `simulate_irule` | `dict` | `tcl-irule-test` (bytecode VM) |
//! | `document_symbols` | `list[dict]` | `tcl-lsp-core::document_symbols` |
//! | `proc_docs` | `list[dict]` | `tcl-compiler::analyser` + docstring |
//! | `insert_docstring_stubs` | `dict` | `tcl-lsp-core::formatting` |
//! | `detect_dialect` | `str` | `tcl-registry::detect_dialect` |
//! | `def_use_chains` | `dict` | `tcl-lsp-core::graphs` (SSA) |
//! | `memory_aliases` | `dict` | `tcl-lsp-core::graphs` (memory-SSA) |
//! | `walk_commands` | `list[dict]` | `tcl-lsp-core::refactor` |
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
/// and the facades — on the Python module.
pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::register_with(m)?;
    options::register_with(m)?;
    results::register_with(m)?;
    facades::register_with(m)?;
    Ok(())
}
