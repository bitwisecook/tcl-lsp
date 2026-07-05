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
    /// The valid operation names and the human-readable "must be …" tail of the
    /// bad-operation error for this kind (the orders match tclsh 9.0).
    const fn ops(self) -> (&'static [&'static str], &'static str) {
        match self {
            TraceKind::Variable => (
                &["array", "read", "write", "unset"],
                "array, read, unset, or write",
            ),
            TraceKind::Command => (&["rename", "delete"], "delete or rename"),
            TraceKind::Execution => (
                &["enter", "leave", "enterstep", "leavestep"],
                "enter, leave, enterstep, or leavestep",
            ),
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
    let (valid, must_be) = kind.ops();
    let s =
        core::str::from_utf8(spec).map_err(|_| CmdError::new("unmatched open brace in list"))?;
    let elems =
        tcl_syntax::list::split_list(s).map_err(|e| CmdError::new(e.message().to_string()))?;
    if elems.is_empty() {
        return Err(CmdError::new(format!(
            "bad operation list \"\": must be one or more of {must_be}"
        )));
    }
    let mut out = Vec::with_capacity(elems.len());
    for o in &elems {
        match valid.iter().find(|v| ***v == **o) {
            Some(v) => out.push(*v),
            None => {
                return Err(CmdError::new(format!(
                    "bad operation \"{o}\": must be {must_be}"
                )));
            }
        }
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

/// Resolve a trace-type word (`trace add|remove|info <type> …`) to its
/// [`TraceKind`], applying Tcl's unambiguous-prefix rule: an exact match always
/// wins, otherwise a unique prefix matches (`var` → `variable`). Mirrors C's
/// `Tcl_GetIndexFromObj` over `traceTypeOptions` (`{variable, command,
/// execution}`), which is why `trace add var x write cb` is accepted
/// (set-2.4 / set-4.4).
///
/// # Errors
/// Returns [`bad_type_error`] when the word matches no type and
/// [`ambiguous_type_error`] when it is a prefix of more than one. An empty
/// word is reported as `bad` (it is a prefix of every option, but C treats the
/// empty index lookup as a miss here).
pub fn resolve_type(got: &str) -> Result<TraceKind, CmdError> {
    const NAMES: [(&str, TraceKind); 3] = [
        ("variable", TraceKind::Variable),
        ("command", TraceKind::Command),
        ("execution", TraceKind::Execution),
    ];
    if got.is_empty() {
        return Err(bad_type_error(got));
    }
    let mut found: Option<TraceKind> = None;
    let mut count = 0u32;
    for (name, kind) in NAMES {
        if name == got {
            return Ok(kind);
        }
        if name.starts_with(got) {
            found = Some(kind);
            count += 1;
        }
    }
    match (count, found) {
        (1, Some(k)) => Ok(k),
        (0, _) => Err(bad_type_error(got)),
        _ => Err(ambiguous_type_error(got)),
    }
}
