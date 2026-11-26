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

//! Optimiser tests that run the optimised program on the VM through
//! `tcl-engine-tclvm`. Helpers copied from `tcl-compiler/tests/it_3/optimiser.rs`.

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::manager::{apply_optimisations, optimise_with_dialect};
use tcl_registry::model::ingress::static_context_for;

mod nested_reads_and_writes;
mod rewrite_barriers;

const TCL: &str = "tcl8.6";

/// Every `Oxxx` code emitted by a single optimiser pass over `src`.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect()
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    opt_codes(src, dialect).iter().any(|c| c == code)
}

/// Apply all (non-hint) optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// `(code, replacement)` pairs from the optimiser for `src`.
fn opt_rewrites(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .into_iter()
        .map(|o| (o.code.as_str().to_owned(), o.replacement.clone()))
        .collect()
}

/// Every analyser diagnostic code for `src`, at any severity — `reparse_errors`
/// keeps only `Severity::Error`, so a warning-level code (W210, W211) needs
/// this instead.
fn analyser_codes(src: &str, dialect: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// Error-severity diagnostic codes the user-facing `tcl diag` surface reports
/// for `src` — the analyser pass plus `run_all_checks`, optimisation codes
/// dropped, mirroring `checks.rs::codes`. Empty means the source re-parses;
/// a rewrite that emits unbalanced text surfaces here as an `E2xx`.
fn reparse_errors(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for diag in run_all_checks(&cu, registry, d) {
        if !diag.code.is_optimisation() && diag.severity == Severity::Error {
            out.push(diag.code.to_string());
        }
    }
    out
}
