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
//! decoding** is shared: resolving the option word against the release's own
//! option set, validating a trace op-list (`{read write}` or the legacy
//! `rwua` letters) against the operations valid for its type, rendering the
//! legacy `trace vinfo` letters, and the `bad option` / `bad type` /
//! `bad operation` messages. Each runtime keeps its own trace storage and
//! converts the canonical op names this returns into its representation (the
//! VM keeps the name list; the WASM runtime folds them into an op bitset).
//!
//! Op sets come back in [`TraceKind::info_order`] — the order C's `TRACE_INFO`
//! arms render them, which is what a runtime must store to make `trace info`
//! byte-identical.
//!
//! Mirrors C's `Tcl_TraceObjCmd` dispatcher plus the `TraceVariableObjCmd` /
//! `TraceCommandObjCmd` / `TraceExecutionObjCmd` option tables (`tclTrace.c`).

use crate::error::CmdError;
use crate::prefix::{OptionTable, Resolution, choice_list};

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

    /// The operation names in the order `trace info` reports them — the order
    /// each `TRACE_INFO` arm tests the stored flag bits in (`tclTrace.c`
    /// 8.6.16 / 9.0.4: variable at `TraceVariableObjCmd`, command at
    /// `TraceCommandObjCmd`, execution at `TraceExecutionObjCmd`).
    ///
    /// This is **not** [`Self::ops`]: C's `opStrings[]` tables are sorted for
    /// the error message, while the `TRACE_INFO` arms hard-code a different
    /// sequence for `variable` (`array read write unset`) and `command`
    /// (`rename delete`). Because Tcl stores the selection as a bitset, the
    /// reported order never depends on how the op list was spelled — so this
    /// is the canonical order a runtime stores an op set in.
    #[must_use]
    pub const fn info_order(self) -> &'static [&'static str] {
        match self {
            TraceKind::Variable => &["array", "read", "write", "unset"],
            TraceKind::Command => &["rename", "delete"],
            TraceKind::Execution => &["enter", "leave", "enterstep", "leavestep"],
        }
    }
}

/// Parse + validate a trace op-list `spec` (a Tcl list such as `{read write}`)
/// for `kind`, returning the canonical operation set in [`TraceKind::info_order`].
/// Mirrors C's per-type op validation.
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
            choice_list(valid)
        )));
    }
    for o in &elems {
        table.index_of_str(o)?;
    }
    // Tcl stores trace operations as a bitset.  Consequently repeated words
    // collapse and `trace info` always reports the selected operations in the
    // fixed `TRACE_INFO` order, irrespective of how the list was spelled.
    Ok(canonical_set(&elems, kind))
}

/// The `spec` operations selected out of `kind`'s [`TraceKind::info_order`] —
/// the bitset collapse C performs, applied to already-validated words.
fn canonical_set(elems: &[impl AsRef<str>], kind: TraceKind) -> Vec<&'static str> {
    let order = kind.info_order();
    let mut out = Vec::with_capacity(order.len());
    for op in order {
        if elems.iter().any(|elem| elem.as_ref() == *op) {
            out.push(*op);
        }
    }
    out
}

/// The legacy `rwua` letter ↔ canonical operation-name mapping, in the fixed
/// order `trace vinfo` renders (`TRACE_OLD_VINFO`, `tclTrace.c` 8.6.16).
const LEGACY_LETTERS: [(u8, &str); 4] = [
    (b'r', "read"),
    (b'w', "write"),
    (b'u', "unset"),
    (b'a', "array"),
];

/// Parse the deprecated Tcl 8.x variable-trace operation string.
///
/// Unlike modern `trace add variable`, the legacy `trace variable` and
/// `trace vdelete` forms take a concatenation of the letters `r`, `w`, `u`,
/// and `a`, not a Tcl list. C expands them to a word list in letter order,
/// duplicates included, and hands that to the same
/// [`TraceKind::Variable`] op parser, which folds it into a bitset — so
/// repeated letters collapse and the stored set is the *same* canonical
/// [`TraceKind::info_order`] set [`parse_ops`] produces. Callers therefore
/// need no legacy-specific storage or matching; only the `trace vinfo`
/// rendering differs, and that is [`legacy_ops_letters`].
///
/// # Errors
/// An empty string or any byte outside `rwua` produces C Tcl's legacy error.
pub fn parse_legacy_variable_ops(spec: &[u8]) -> Result<Vec<&'static str>, CmdError> {
    if spec.is_empty() || spec.iter().any(|byte| !b"rwua".contains(byte)) {
        let got = String::from_utf8_lossy(spec);
        return Err(CmdError::new(format!(
            "bad operations \"{got}\": should be one or more of rwua"
        )));
    }
    let words: Vec<&str> = LEGACY_LETTERS
        .iter()
        .filter(|(letter, _)| spec.contains(letter))
        .map(|(_, operation)| *operation)
        .collect();
    Ok(canonical_set(&words, TraceKind::Variable))
}

/// The operation word a variable-trace callback is invoked with. A trace
/// installed by the deprecated `trace variable` form keeps C's
/// `TCL_TRACE_OLD_STYLE` flag and is called with the single `rwua` **letter**
/// instead of the operation name (`TraceVarProc`, `tclTrace.c` 8.6.16:2002-2011)
/// — the one place the legacy form is not just a spelling of `trace add
/// variable`. The flag never affects matching: `trace remove` masks it out, so
/// either form removes a trace the other installed.
#[must_use]
pub fn callback_op_word(op: &str, old_style: bool) -> &str {
    if !old_style {
        return op;
    }
    LEGACY_LETTERS
        .iter()
        .find(|(_, operation)| *operation == op)
        .map_or(op, |(letter, _)| match letter {
            b'r' => "r",
            b'w' => "w",
            b'u' => "u",
            _ => "a",
        })
}

/// Render a canonical variable-trace operation set as the `rwua` letter string
/// `trace vinfo` reports — C's `TRACE_OLD_VINFO` arm, which tests the stored
/// flags in the fixed `r`, `w`, `u`, `a` order (`tclTrace.c` 8.6.16).
/// Unrecognised names are ignored, so a runtime can pass its stored op list
/// whatever byte type it keeps it in.
#[must_use]
pub fn legacy_ops_letters<S: AsRef<[u8]>>(ops: &[S]) -> String {
    LEGACY_LETTERS
        .iter()
        .filter(|(_, operation)| ops.iter().any(|op| op.as_ref() == operation.as_bytes()))
        .map(|(letter, _)| char::from(*letter))
        .collect()
}

/// Resolve `trace`'s first word against the option set the emulated release
/// carries, with C's `Tcl_GetIndexFromObj(… traceOptions, "option", 0 …)`
/// rule: an exact match wins, otherwise a unique prefix (`trace var x w cb`
/// is `trace variable` in 8.6).
///
/// `visible` is the release-gated option list — the registry retires
/// `variable`/`vdelete`/`vinfo` at 9.0, and C's error enumerates exactly the
/// options the build has (`#ifndef TCL_REMOVE_OBSOLETE_TRACES`), so the
/// caller passes what the active dialect declares rather than a fixed list.
///
/// # Errors
/// `bad option "X": must be …` when nothing matches, `ambiguous option "X":
/// must be …` when a prefix matches several (`trace v` in 8.6).
pub fn resolve_option<'a>(word: &str, visible: &[&'a str]) -> Result<&'a str, CmdError> {
    let table = OptionTable::abbreviating("option", visible);
    let index = table.index_of_str(word)?;
    Ok(visible[index])
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

#[cfg(test)]
mod tests {
    use super::{
        TraceKind, callback_op_word, legacy_ops_letters, parse_legacy_variable_ops, parse_ops,
        resolve_option,
    };

    /// An old-style registration is called with the `rwua` letter, a modern
    /// one with the operation name.
    #[test]
    fn old_style_callbacks_get_the_letter() {
        for (name, letter) in [
            ("read", "r"),
            ("write", "w"),
            ("unset", "u"),
            ("array", "a"),
        ] {
            assert_eq!(callback_op_word(name, true), letter);
            assert_eq!(callback_op_word(name, false), name);
        }
    }

    /// The stored set is C's `TRACE_INFO` order, not the `opStrings[]` table
    /// order the error message uses — tclsh 8.6.16 / 9.0.4 report
    /// `{array read write unset}` and `{rename delete}`.
    #[test]
    fn operations_are_a_canonical_set_in_info_order() {
        assert_eq!(
            parse_ops(b"write read write unset", TraceKind::Variable).unwrap(),
            vec!["read", "write", "unset"]
        );
        assert_eq!(
            parse_ops(b"unset write read array", TraceKind::Variable).unwrap(),
            vec!["array", "read", "write", "unset"]
        );
        assert_eq!(
            parse_ops(b"delete rename", TraceKind::Command).unwrap(),
            vec!["rename", "delete"]
        );
        assert_eq!(
            parse_ops(b"leavestep leave enterstep enter", TraceKind::Execution).unwrap(),
            vec!["enter", "leave", "enterstep", "leavestep"]
        );
    }

    /// The bad-operation error still enumerates C's `opStrings[]` order.
    #[test]
    fn bad_operation_enumerates_table_order() {
        assert_eq!(
            parse_ops(b"w", TraceKind::Variable).unwrap_err().message(),
            "bad operation \"w\": must be array, read, unset, or write"
        );
    }

    #[test]
    fn legacy_variable_operations_are_flags_not_a_list() {
        // C expands the letters to a word list and reuses the modern parser,
        // so the stored set is the same canonical set either spelling gives.
        assert_eq!(
            parse_legacy_variable_ops(b"awrw").unwrap(),
            vec!["array", "read", "write"]
        );
        assert_eq!(
            parse_legacy_variable_ops(b"rrw").unwrap(),
            parse_ops(b"write read", TraceKind::Variable).unwrap()
        );
        assert_eq!(
            parse_legacy_variable_ops(b"").unwrap_err().message(),
            "bad operations \"\": should be one or more of rwua"
        );
        assert_eq!(
            parse_legacy_variable_ops(b"read").unwrap_err().message(),
            "bad operations \"read\": should be one or more of rwua"
        );
    }

    /// `trace vinfo` reports letters in C's fixed `r`, `w`, `u`, `a` order,
    /// whatever order the set is stored in.
    #[test]
    fn legacy_letters_render_in_rwua_order() {
        let ops = parse_ops(b"unset write read array", TraceKind::Variable).unwrap();
        assert_eq!(legacy_ops_letters(&ops), "rwua");
        assert_eq!(
            legacy_ops_letters(&parse_legacy_variable_ops(b"wr").unwrap()),
            "rw"
        );
        assert_eq!(legacy_ops_letters::<&str>(&[]), "");
    }

    /// The option word follows `Tcl_GetIndexFromObj` with flags `0`, over the
    /// release's own option set: 8.x carries the three legacy forms, 9.0 does
    /// not, and both the resolution and the error text follow.
    #[test]
    fn option_word_resolves_against_the_release_option_set() {
        const V8: [&str; 6] = ["add", "info", "remove", "variable", "vdelete", "vinfo"];
        const V9: [&str; 3] = ["add", "info", "remove"];
        assert_eq!(resolve_option("var", &V8).unwrap(), "variable");
        assert_eq!(resolve_option("add", &V9).unwrap(), "add");
        assert_eq!(
            resolve_option("v", &V8).unwrap_err().message(),
            "ambiguous option \"v\": must be add, info, remove, variable, vdelete, or vinfo"
        );
        assert_eq!(
            resolve_option("variable", &V9).unwrap_err().message(),
            "bad option \"variable\": must be add, info, or remove"
        );
        assert_eq!(
            resolve_option("zzz", &V8).unwrap_err().message(),
            "bad option \"zzz\": must be add, info, remove, variable, vdelete, or vinfo"
        );
    }
}
