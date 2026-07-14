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

//! `trace` argument validation — the op-list parser and the type-error
//! catalogue, shared across runtimes.
//!
//! `trace` is heavily stateful (each runtime owns its trace tables and the
//! firing wired into variable/command/execution access), so only the **argument
//! decoding** is shared: validating a trace op-list (`{read write}`) against the
//! operations valid for its type, and the `bad type` / `bad operation` messages.
//! Each runtime keeps its own trace storage and converts the canonical op names
//! this returns into its representation (the VM keeps the name list; the WASM
//! runtime folds them into an op bitset).
//!
//! Mirrors C's `TraceVariableObjCmd` / `TraceCommandObjCmd` /
//! `TraceExecutionObjCmd` option tables (`tclTrace.c`).

use crate::error::CmdError;
use crate::option_table::{OptionTable, Resolution, enumerate_names};

/// A trace category — selects the valid operations and the error wording.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TraceKind {
    /// `trace … variable` — `array`/`read`/`write`/`unset`.
    Variable,
    /// `trace … command` — `rename`/`delete`.
    Command,
    /// `trace … execution` — `enter`/`leave`/`enterstep`/`leavestep`.
    Execution,
}

impl TraceKind {
    /// The valid operation names for this kind, in C's `opStrings[]` table
    /// order (`tclTrace.c`) — the bad-operation error enumerates them in
    /// this order.
    const fn ops(self) -> &'static [&'static str] {
        match self {
            TraceKind::Variable => &["array", "read", "unset", "write"],
            TraceKind::Command => &["delete", "rename"],
            TraceKind::Execution => &["enter", "leave", "enterstep", "leavestep"],
        }
    }
}

/// Parse + validate a trace op-list `spec` (a Tcl list such as `{read write}`)
/// for `kind`, returning the canonical operation names in spec order. Mirrors
/// C's per-type op validation.
///
/// # Errors
/// A malformed list, an empty op-list (`bad operation list ""`), or an
/// unrecognised operation (`bad operation "X": must be …`).
pub fn parse_ops(spec: &[u8], kind: TraceKind) -> Result<Vec<&'static str>, CmdError> {
    let valid = kind.ops();
    // C resolves each op element with `TCL_EXACT` (`tclTrace.c`): unlike most
    // option words, a trace operation may NOT be abbreviated — `trace add
    // variable x w cb` is `bad operation "w"` in tclsh (probed 8.6.14).
    let table = OptionTable::exact_only("operation", valid);
    let s =
        core::str::from_utf8(spec).map_err(|_| CmdError::new("unmatched open brace in list"))?;
    let elems =
        tcl_syntax::list::split_list(s).map_err(|e| CmdError::new(e.message().to_string()))?;
    if elems.is_empty() {
        return Err(CmdError::new(format!(
            "bad operation list \"\": must be one or more of {}",
            enumerate_names(valid)
        )));
    }
    let mut out = Vec::with_capacity(elems.len());
    for o in &elems {
        let idx = table.index_of_str(o)?;
        out.push(valid[idx]);
    }
    Ok(out)
}

/// `bad type "X": must be execution, command, or variable` — the trace-type
/// option error (`trace add|remove|info <type> …`). C's `traceTypeOptions`
/// reports it as a `bad option`.
#[must_use]
pub fn bad_type_error(got: &str) -> CmdError {
    CmdError::new(format!(
        "bad option \"{got}\": must be execution, command, or variable"
    ))
}

/// `ambiguous option "X": must be execution, command, or variable` — the
/// trace-type option error when an abbreviation matches more than one type.
#[must_use]
pub fn ambiguous_type_error(got: &str) -> CmdError {
    CmdError::new(format!(
        "ambiguous option \"{got}\": must be execution, command, or variable"
    ))
}

// C's `traceTypeOptions[]` (`tclTrace.c`), resolved with abbreviations
// allowed (flags 0) — which is why `trace add var x write cb` is accepted
// (set-2.4 / set-4.4).
const TYPE_NAMES: [&str; 3] = ["execution", "command", "variable"];
const TYPE_KINDS: [TraceKind; 3] = [
    TraceKind::Execution,
    TraceKind::Command,
    TraceKind::Variable,
];
const TYPE_OPTIONS: OptionTable<'static> = OptionTable::abbreviating("option", &TYPE_NAMES);

/// Resolve a trace-type word (`trace add|remove|info <type> …`) to its
/// [`TraceKind`] with the shared [`OptionTable`] rule: an exact match always
/// wins, otherwise a unique prefix matches (`var` → `variable`).
///
/// # Errors
/// Returns [`bad_type_error`] when the word matches no type and
/// [`ambiguous_type_error`] when it abbreviates more than one — including the
/// empty word, which prefixes all three types (`trace add "" x …` is
/// `ambiguous option ""` in tclsh).
pub fn resolve_type(got: &str) -> Result<TraceKind, CmdError> {
    match TYPE_OPTIONS.resolve(got.as_bytes()) {
        Resolution::Exact(i) | Resolution::UniquePrefix(i) => Ok(TYPE_KINDS[i]),
        Resolution::Ambiguous => Err(ambiguous_type_error(got)),
        Resolution::NoMatch => Err(bad_type_error(got)),
    }
}
