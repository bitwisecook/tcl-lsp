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

//! Semantic-analyser tests.
//!
//! These drive `analyser.analyse(src)` and assert over the
//! returned result's `.diagnostics` (W###/E###/I###/H### codes) *and* its
//! structured semantic model (`global_scope`, `all_procs`, `all_classes`,
//! `regex_patterns`, `package_requires`, `command_aliases`, `source_targets`,
//! `unknown_proc_info`). [`tcl_compiler::analyser::AnalysisResult`]
//! exposes all these fields.
//!
//! ## Diagnostic surface
//!
//! Diagnostic assertions go through [`codes`]/[`fires`], copied verbatim from
//! the in-crate harness `src/analyser/diagnostics/fp/mod.rs`: they merge the
//! analyser pass with the `run_all_checks` compiler-checks pass (shimmer /
//! taint / dead-store) and drop optimisation codes, exactly mirroring the
//! user-facing `tcl diag` surface.
//!
//! ## C-Tcl ground truth
//!
//! Diagnostics that mirror a real tclsh error are pinned to ground truth from
//! `scripts/dev/tclsh_check.sh` (tclsh8.6 + tclsh9.0 on PATH). The headline
//! correspondences, verified while authoring this file:
//!   * `puts $x`            → `can't read "x": no such variable`      (W210)
//!   * `set`                → `wrong # args: should be "set …"`       (E002)
//!   * `break extra`        → `wrong # args: should be "break"`       (E003)
//!   * `regsub a b c d e`   → `wrong # args: should be "regsub …"`    (E003)
//!   * `string bogus hello` → `unknown … subcommand "bogus"`          (W001)
//!   * `namespace`          → `wrong # args: should be "namespace …"` (E001)
//!   * `unset x` (unset)    → `can't unset "x": no such variable`     (W213)
//!
//! Pure static-analysis heuristics with no direct runtime analogue (unused
//! params, dead stores, paste-error hints, etc.) are noted as such inline.
//!
//! ## Behavioural notes
//!
//! A handful of cases target features the single-document analyser +
//! `run_all_checks` surface does not reproduce (they live in the cross-file
//! workspace index). Each is asserted against the actual verdict with an
//! explanatory comment; where the behaviour is tracked elsewhere it is omitted.
//! See the per-section notes below.

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::model::ingress::static_context_for;

mod aliases_and_renames;
mod diagnostic_reports;
mod lexical_checks;
mod procedures;
mod script_arguments;
mod tcloo;
mod tcloo_dispatch;
mod unresolved_commands;
mod variables;

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";

/// Every diagnostic code the full pipeline surfaces for `src` under `dialect`,
/// mirroring the user-facing `tcl diag` path: the analyser pass plus the
/// `run_all_checks` compiler-checks pass (shimmer / taint / dead-store), with
/// optimisation codes excluded exactly as `diag` excludes them.
///
/// Copied verbatim from `src/analyser/diagnostics/fp/mod.rs::codes`.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the full diagnostic set for `src`.
fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// Count of how many times `code` appears in the merged diagnostic set.
fn count(src: &str, dialect: &str, code: &str) -> usize {
    codes(src, dialect)
        .iter()
        .filter(|c| c.as_str() == code)
        .count()
}

/// Full `(code, message)` pairs from the *analyser pass only* — used when a
/// test pins a message substring or message-scoped count (the analyser
/// pass owns every code those tests inspect: W210/W211/W213/W214/W215/W216/
/// W123/E001/E002/E003/W001).
fn analyser_diags(src: &str, dialect: &str) -> Vec<(String, String, Severity)> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.message.clone(), d.severity))
        .collect()
}

/// Count of W210 diagnostics whose message names `var`.
fn w210_for(src: &str, var: &str) -> usize {
    analyser_diags(src, D)
        .iter()
        .filter(|(c, m, _)| c == "W210" && m.contains(var))
        .count()
}

// Class factories and dynamically-installed members.
//
// C-Tcl ground truth for every case below comes from tclsh 9.0.4 and
// tclsh 8.6.16, which agree on all of them:
//
// * `oo::class create Megawidget { superclass oo::class; self method create
//   {name superclasses body} { next $name [list superclass MegawidgetClass
//   {*}$superclasses]\;$body } }` then `Megawidget create SimpleWidget {}
//   {…}` / `Megawidget create FocusableWidget SimpleWidget {…}` /
//   `Megawidget create IconList FocusableWidget {…}` report
//   `info class superclasses` = `::MegawidgetClass`,
//   `::MegawidgetClass ::SimpleWidget`, and
//   `::MegawidgetClass ::FocusableWidget` respectively, and `my CreateHull`
//   from an `IconList` instance really runs `FocusableWidget`'s override.
// * `oo::define S { method {*}{foo {} {return foo-ran}} }` defines a real,
//   callable `foo`; so does the same `{*}` form inside an `oo::class create`
//   body.  `constructor {*}[info class constructor ::Base]` /
//   `method $m {*}[info class definition ::Base $m]` are equally real but
//   carry no statically-knowable element list.
// * `set ns tc; ${ns}::setdef …` dispatches to `::tc::setdef`.
mod class_factories;
mod method_bodies;
