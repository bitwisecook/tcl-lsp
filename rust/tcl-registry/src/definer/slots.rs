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

//! `TclOO` slot operations (`-append`, `-set`, ...) and how a slot applies them to its list.

use tcl_dialect::TclVersion;

/// One operation on a `TclOO` **slot** — the list-valued definition words
/// (`filter`, `superclass`, `mixin`, `variable`) that are `oo::Slot`
/// instances in real Tcl rather than plain assignments.
///
/// A slot call's first argument may name the operation explicitly
/// (`filter -set x`, `mixin -append M`); a bare word list uses the slot's
/// own default operation.  Oracle for the operation set, C Tcl 9.0.4
/// `tclOODefineCmds.c` (`slotMethods[]`), confirmed live on tclsh 9.0.4:
///
/// ```text
/// unknown method "-bogus": must be -append, -appendifnew, -clear,
/// -prepend, -remove or -set
/// ```
///
/// Tcl 8.6 has only `-set` / `-append` / `-clear` (`tclOO.c`'s embedded
/// slot script: `export -set -append -clear`); `-appendifnew` / `-prepend`
/// / `-remove` are 9.0 additions.  The fold below accepts all six under
/// every dialect — modelling an 8.6 script that uses a 9.0-only operation
/// as the operation it names is strictly closer to what the author meant
/// than treating `-prepend` as a member name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOp {
    /// `-set` — replace the slot's contents with the arguments.
    Set,
    /// `-append` — add the arguments at the end (no deduplication:
    /// tclsh 9.0.4, `filter a ; filter a b` → `a a b`).
    Append,
    /// `-appendifnew` (9.0+) — add each argument not already present.
    AppendIfNew,
    /// `-prepend` (9.0+) — add the arguments at the front
    /// (tclsh 9.0.4: `filter a ; filter -prepend b` → `b a`).
    Prepend,
    /// `-remove` (9.0+) — remove the named items.
    Remove,
    /// `-clear` — empty the slot (takes no further arguments).
    Clear,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SlotOpSpec {
    pub(super) name: &'static str,
    pub(super) operation: SlotOp,
    since_9: bool,
}

pub(super) const TCLOO_SLOT_OPERATIONS: &[SlotOpSpec] = &[
    SlotOpSpec {
        name: "-append",
        operation: SlotOp::Append,
        since_9: false,
    },
    SlotOpSpec {
        name: "-appendifnew",
        operation: SlotOp::AppendIfNew,
        since_9: true,
    },
    SlotOpSpec {
        name: "-clear",
        operation: SlotOp::Clear,
        since_9: false,
    },
    SlotOpSpec {
        name: "-prepend",
        operation: SlotOp::Prepend,
        since_9: true,
    },
    SlotOpSpec {
        name: "-remove",
        operation: SlotOp::Remove,
        since_9: true,
    },
    SlotOpSpec {
        name: "-set",
        operation: SlotOp::Set,
        since_9: false,
    },
];

impl SlotOp {
    /// Parse an explicit slot-operation word, or `None` when `word` is not
    /// one.  Only the **first** argument of a slot call is ever an
    /// operation — tclsh 9.0.4: `filter a -set b` appends the three literal
    /// items `a`, `-set`, `b`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        TCLOO_SLOT_OPERATIONS
            .iter()
            .find(|entry| entry.name == word)
            .map(|entry| entry.operation)
    }

    /// Resolve an exact runtime slot method for `version`.
    ///
    /// `TclOO` slot methods do not abbreviate. The miss message uses `TclOO`'s
    /// no-Oxford-comma method list and is filtered before rendering so Tcl 8.6
    /// never advertises the 9.0-only operations.
    ///
    /// # Errors
    /// Tcl's byte-exact `unknown method` message.
    pub fn resolve_runtime(word: &[u8], version: TclVersion) -> Result<Self, Vec<u8>> {
        let available: Vec<&SlotOpSpec> = TCLOO_SLOT_OPERATIONS
            .iter()
            .filter(|entry| !entry.since_9 || version >= TclVersion::V9_0)
            .collect();
        let names: Vec<&str> = available.iter().map(|entry| entry.name).collect();
        match tcl_cmd_core::prefix::scan(&names, word, true) {
            tcl_cmd_core::prefix::Resolution::Exact(index) => Ok(available[index].operation),
            tcl_cmd_core::prefix::Resolution::UniquePrefix(_)
            | tcl_cmd_core::prefix::Resolution::Ambiguous
            | tcl_cmd_core::prefix::Resolution::NoMatch => {
                let mut message = b"unknown method \"".to_vec();
                message.extend_from_slice(word);
                message.extend_from_slice(b"\": must be ");
                message.extend_from_slice(&tcl_cmd_core::prefix::tcloo_choice_list_bytes(&names));
                Err(message)
            }
        }
    }
}

/// The slot behaviour of a list-valued member word: its default operation
/// and whether the slot deduplicates on append.
///
/// Defaults pinned against C Tcl — identical in 9.0.4 (`slots[]` in
/// `tclOODefineCmds.c`) and 8.6.16 (`tclOO.c`'s embedded
/// `--default-operation` forwards) — and confirmed live on tclsh 9.0.4:
///
/// | slot                    | default op | dedup |
/// |-------------------------|------------|-------|
/// | `filter` (both sides)   | `-append`  | no (`filter a; filter a b` → `a a b`) |
/// | `variable` (both sides) | `-append`  | yes (`variable a; variable a b` → `a b`) |
/// | `superclass`            | `-set`     | —     |
/// | `mixin` (both sides)    | `-set`     | —     |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotSpec {
    /// The operation a bare word list applies.
    pub default_op: SlotOp,
    /// Whether appending skips items already present (`variable` does,
    /// `filter` does not — oracle above).
    pub dedup: bool,
}

impl SlotSpec {
    /// Select a non-query slot call from original counted method words.
    /// Method availability and byte selection share the runtime slot owner;
    /// this describes grammar and supplies no successful effect or class join.
    #[must_use]
    pub fn split_original_call<'a, S: AsRef<[u8]>>(
        &self,
        args: &'a [S],
        protocol: tcl_syntax::naming::NativeNameProtocol,
    ) -> Option<(SlotOp, &'a [S])> {
        let first = args.first()?.as_ref();
        let tcl_syntax::naming::NativeNameProtocol::C(version) = protocol else {
            return None;
        };
        if version < TclVersion::V8_6 {
            return None;
        }
        if !first.starts_with(b"-") {
            return Some((self.default_op, args));
        }
        let selected = protocol.oo_method_input(first).ok()?;
        let operation = SlotOp::resolve_runtime(selected.selected(), version).ok()?;
        if operation == SlotOp::Clear && args.len() != 1 {
            return None;
        }
        Some((operation, &args[1..]))
    }

    /// Split one slot call's arguments into its effective operation and
    /// value words: an explicit leading operation word wins, a bare list
    /// takes the slot default.  Returns `None` for an unrecognised leading
    /// `-word` — real Tcl aborts the whole definition there (`unknown
    /// method "-bogus"`), so a consumer must not fold anything for it.
    #[must_use]
    pub fn split_call<'a>(&self, args: &'a [String]) -> Option<(SlotOp, &'a [String])> {
        match args.first() {
            Some(first) if first.starts_with('-') => {
                SlotOp::parse(first).map(|op| (op, &args[1..]))
            }
            _ => Some((self.default_op, args)),
        }
    }

    /// Fold one slot call into `current` — the single definition of what
    /// each operation does to the slot's list, shared by every consumer so
    /// the instance `filters`, class-object `class_filters`, `superclass`,
    /// `mixin`, and `variable` channels cannot diverge.
    ///
    /// An unrecognised leading operation word leaves the slot unchanged
    /// (see [`Self::split_call`]).
    pub fn apply(&self, current: &mut Vec<String>, args: &[String]) {
        let Some((op, values)) = self.split_call(args) else {
            return;
        };
        self.apply_values(current, op, values);
    }

    /// Fold independently resolved values through the same slot operation.
    /// Name parsing and provider identity remain the caller's separate owners.
    pub fn apply_values<T: Clone + PartialEq>(
        &self,
        current: &mut Vec<T>,
        op: SlotOp,
        values: &[T],
    ) {
        match op {
            SlotOp::Set => {
                current.clear();
                self.extend(current, values);
            }
            SlotOp::Append => self.extend(current, values),
            SlotOp::AppendIfNew => {
                for v in values {
                    if !current.iter().any(|c| c == v) {
                        current.push(v.clone());
                    }
                }
            }
            SlotOp::Prepend => {
                let mut fresh = Vec::with_capacity(values.len() + current.len());
                self.extend(&mut fresh, values);
                fresh.append(current);
                *current = fresh;
            }
            SlotOp::Remove => current.retain(|c| !values.iter().any(|v| v == c)),
            SlotOp::Clear => current.clear(),
        }
    }

    /// Append `values` honouring the slot's dedup rule.
    pub(super) fn extend<T: Clone + PartialEq>(self, current: &mut Vec<T>, values: &[T]) {
        for v in values {
            if !self.dedup || !current.iter().any(|c| c == v) {
                current.push(v.clone());
            }
        }
    }
}
