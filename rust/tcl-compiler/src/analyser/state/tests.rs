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

use super::*;
use crate::analyser::types::ScopeKind;

mod analysis_entries;
mod definitions_and_instances;
mod flow_diagnostics;
mod ignored_catch_results;
mod injection_warnings;
mod malformed_if;
mod nested_invocations;
mod recorded_aliases_and_patterns;
mod unknown_subcommands;
mod usage_lints;

// Incremental analysis differential oracle

/// A projection of `AnalysisResult` capturing the observable
/// identity an incremental analysis must preserve: sorted
/// `(code, start, end)` diagnostics, proc names, global vars, and
/// `(invocation name, start)` pairs.
type ResultProjection = (
    Vec<(String, u32, u32)>,
    Vec<String>,
    Vec<String>,
    Vec<(String, u32)>,
);

fn project_result(r: &AnalysisResult) -> ResultProjection {
    let mut diags: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.span.start(), d.span.end()))
        .collect();
    diags.sort();
    let mut procs: Vec<_> = r.all_procs.keys().cloned().collect();
    procs.sort();
    let mut vars: Vec<_> = r.global_scope.variables.keys().cloned().collect();
    vars.sort();
    let mut invs: Vec<_> = r
        .command_invocations
        .iter()
        .map(|i| (i.name.clone(), i.range.start()))
        .collect();
    invs.sort();
    (diags, procs, vars, invs)
}
