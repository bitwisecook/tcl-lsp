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

//! Taint-analysis tests.
//!
//! These drive the taint pass and assert over the returned `TaintWarning` list
//! (`.code` / `.variable` / `.sink_command` / `.message`), plus a block of
//! pure-lattice tests over `TaintLattice` / `TaintColour` / `taint_join`. The
//! driver is [`tcl_compiler::taint::find_taint_warnings_for_cu`] (the
//! whole-unit taint pass that `run_all_checks` also drives), over the `pub`
//! [`tcl_compiler::taint::TaintLattice`] / [`TaintColour`] types.
//!
//! ## Driving a snippet
//!
//! [`warns`] builds a [`CompilationUnit`] for the snippet and runs
//! `find_taint_warnings_for_cu`, the `TaintWarning`-producing surface (sink
//! detection T100–T106, setter constraints IRULE3101, iRules URI-split
//! IRULE3103, destructive-file W313). [`codes`] returns the sorted diagnostic
//! codes; [`of_code`] filters warnings by diagnostic code. The lattice join is
//! [`TaintLattice::join`].
//!
//! ## Dialect handling
//!
//! The dialect is threaded explicitly into `find_taint_warnings_for_cu`, so
//! each test passes the dialect it needs:
//!   * Dialect-agnostic tests run under [`D`] (`tcl8.6`).
//!     iRules *sources* (`HTTP::uri`, `IP::client_addr`, …) still taint there —
//!     they are registered in every dialect (registry test
//!     `http_uri_is_a_source_in_every_dialect`) — so the T100/T101/T102/T103
//!     source+sink tests work under plain Tcl.
//!   * iRules *sinks* (IRULE3001–3004, `log` → IRULE3003) only fire under
//!     [`IR`] (`f5-irules`); those tests pass `IR`.
//!   * The `…_not_in_tcl86` / `…_tcl_dialect_clean` cases pass `D`
//!     explicitly to prove the iRules sink does NOT fire under plain Tcl.
//!
//! ## Policy-vs-semantic proof split (C-Tcl)
//!
//! Taint is a *security/policy* analysis, not a runtime value. Most assertions
//! are policy-level — "this source is tainted", "this sink is flagged", "this
//! colour suppresses that code" — and have no tclsh ground truth to pin: tclsh
//! does not implement taint, so there is nothing to compare against. Those are
//! pure policy checks.
//!
//! Where a test's *premise* is a Tcl-semantic fact — a command's actual
//! behaviour, a string transform, whether the first list element becomes the
//! command word — it is verified against `scripts/dev/tclsh_check.sh` (tclsh8.6
//! + tclsh9.0) and cited in a `// tclsh:` comment.
//!
//! The headline facts proven while authoring this file:
//!
//!   * `eval [list $raw]`        → tclsh runs `$raw` as the command word
//!     (`proc marker args {puts EXECUTED}; eval [list marker]` prints
//!     `EXECUTED`) — so `LIST_CANONICAL` does NOT suppress T100 (D5-T100).
//!   * `eval [list puts $raw]`   → tclsh prints `marker` (`puts` is the command
//!     word, `$raw` is its argument) — so a literal `[list <known-cmd> …]` head
//!     DOES suppress T100.
//!   * `string length $x`        → tclsh returns an int (`11` for an 11-char
//!     value) — confirming it is a taint sanitiser (fixed numeric return).
//!   * `regexp -- …`             → tclsh treats `--` as the option terminator
//!     (`regexp -- -foo bar` → `0`, not a `bad switch` error).
//!
//! F5/iRules-specific taint sources and sinks (`HTTP::*`, `IP::*`, `SSL::*`,
//! `HTTP::respond`, …) are NOT core-tclsh commands, so snippets that mention
//! them carry a `// f5-dialect` note — there is no tclsh proof to cite for the
//! command itself, only the policy verdict.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::taint::{TaintColour, TaintLattice, TaintWarning, find_taint_warnings_for_cu};
use tcl_registry::model::ingress::static_context_for;

mod colours;
mod lattice;
mod object_instance_sources;
mod option_sinks;
mod propagation;
mod setters;
mod sink_suppression;
mod sinks;
mod sources;
mod uri_validation;

/// Default dialect for snippets that are not dialect-sensitive. iRules sources
/// still taint under this dialect (registered globally), so the generic
/// T100–T106 tests use it.
const D: &str = "tcl8.6";
/// iRules dialect — gates the IRULE3001–3004 / IRULE3101 / IRULE3103 sinks and
/// `log` → IRULE3003.
const IR: &str = "f5-irules";

/// The source a fixture is analysed as. Only `when`, `proc`, `priority` and
/// `timing` are legal at the top level of an iRule, so an iRules fixture that
/// declares no handler of its own is the body of an `HTTP_REQUEST` handler.
/// Offsets in such a fixture's warnings are shifted by [`RULE_PREFIX`].
fn rule_source(src: &str, dialect: &str) -> String {
    if dialect == IR && !src.contains("when ") {
        format!("{RULE_PREFIX}{src}\n}}")
    } else {
        src.to_owned()
    }
}

const RULE_PREFIX: &str = "when HTTP_REQUEST {\n";

/// Every `TaintWarning` the whole-unit taint pass surfaces for `src` under
/// `dialect`.
fn warns(src: &str, dialect: &str) -> Vec<TaintWarning> {
    let src = &rule_source(src, dialect);
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    find_taint_warnings_for_cu(&cu, registry, dialect_opt)
}

/// [`warns`] for a unit built with the profile's invocation dialect.
fn dialect_warns(src: &str, dialect: &str) -> Vec<TaintWarning> {
    let src = &rule_source(src, dialect);
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for_dialect(src, registry, false, dialect);
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    find_taint_warnings_for_cu(&cu, registry, Some(profile))
}

/// [`dialect_warns`] where an undefined `$fd` gets a defined channel first.
fn channel_warns(src: &str, dialect: &str) -> Vec<TaintWarning> {
    if src.contains("$fd") && !src.contains("set fd") {
        dialect_warns(&format!("set fd stdin\n{src}"), dialect)
    } else {
        dialect_warns(src, dialect)
    }
}

/// [`codes`] for a unit built with the profile's invocation dialect.
fn dialect_codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = dialect_warns(src, dialect)
        .iter()
        .map(|w| w.code.to_string())
        .collect();
    out.sort();
    out
}

/// [`codes`] through [`channel_warns`].
fn channel_codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = channel_warns(src, dialect)
        .iter()
        .map(|w| w.code.to_string())
        .collect();
    out.sort();
    out
}

/// [`of_code`] for a unit built with the profile's invocation dialect.
fn dialect_of_code(src: &str, dialect: &str, code: &str) -> Vec<TaintWarning> {
    dialect_warns(src, dialect)
        .into_iter()
        .filter(|w| w.code.to_string() == code)
        .collect()
}

/// [`of_code`] through [`channel_warns`].
fn channel_of_code(src: &str, dialect: &str, code: &str) -> Vec<TaintWarning> {
    channel_warns(src, dialect)
        .into_iter()
        .filter(|w| w.code.to_string() == code)
        .collect()
}

/// Sorted list of diagnostic codes from the taint pass.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = warns(src, dialect)
        .iter()
        .map(|w| w.code.to_string())
        .collect();
    out.sort();
    out
}

/// Taint warnings of one diagnostic `code`.
fn of_code(src: &str, dialect: &str, code: &str) -> Vec<TaintWarning> {
    warns(src, dialect)
        .into_iter()
        .filter(|w| w.code.to_string() == code)
        .collect()
}
