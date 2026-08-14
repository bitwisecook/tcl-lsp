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

//! Repeated argument-role layouts — a role that recurs at a fixed stride.
//!
//! A family of Tcl commands takes an unbounded, *regular* argument tail: a
//! name at every word (`global a b c`), a name at every other word
//! (`variable n v n v`), a name/value pair after a fixed prefix
//! (`namespace upvar NS o l o l`), or a repeating group with a trailing body
//! excluded (`foreach v1 l1 v2 l2 body`, `dict update d k v k v body`).
//!
//! Neither [`crate::CommandSpec::arg_roles`] (a fixed index table) nor an
//! `arg_role_resolver` closure is a good fit: the table cannot express an
//! unbounded tail, and a closure per command is data the registry cannot
//! inspect, document, serialise, or exhaustively test. Every LSP consumer
//! that needed one of these layouts therefore re-derived it by hand from the
//! command's *name* — three separate copies of the stride arithmetic in the
//! semantic-token walk alone (issue #1185).
//!
//! [`RepeatedArgLayout`] is that layout as data. It feeds
//! [`crate::CommandRegistry::arg_indices_for_role`] like any other role
//! source, so a consumer asking "which arguments are variable names" simply
//! gets the right answer for `global` without knowing what `global` is.

use crate::arg_role::ArgRole;

/// A role that recurs at a fixed stride over a command's argument tail.
///
/// Indices are 0-based over the **post-head** argument list, and — for a
/// layout declared on a [`crate::SubCommand`] — over the words *after* the
/// subcommand word, matching every other per-subcommand index in the
/// registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepeatedArgLayout {
    /// The role assigned at each covered position.
    pub role: ArgRole,
    /// First covered index.
    pub start: u8,
    /// Distance between covered positions. `1` covers every word from
    /// `start`; `2` covers every other one (the name of a name/value pair).
    pub stride: u8,
    /// How many words at the **end** of the argument list the layout does
    /// not cover — the trailing body of `foreach` / `dict update`.
    pub exclude_trailing: u8,
    /// The command accepts one *optional* leading word before `start` whose
    /// presence is signalled by nothing but the argument count — `upvar`'s
    /// `?level?`.
    ///
    /// When set, the repeating group is anchored to the **end** of the
    /// covered range rather than to `start`, so the layout shifts right by
    /// one exactly when the count says the optional word was supplied.
    /// `upvar a b` (2 words, no level) binds the local at index 1;
    /// `upvar 1 a b` (3 words, a level) binds it at index 2 — and both are
    /// the same declaration.
    pub optional_leading_word: bool,
    /// Whether the local this position names is bound only when a runtime
    /// **data** condition holds — a dict/array key actually being present —
    /// rather than unconditionally whenever the statement executes.
    ///
    /// `dict update dictVar key varName …` binds `varName` only if `key` is
    /// present in the dict at that moment (tclsh: an absent key leaves
    /// `varName` unset); a plain `global`/`variable`/`upvar`/`namespace
    /// upvar` local, by contrast, is bound whenever the statement runs at
    /// all — it does not depend on the *content* of anything, only on the
    /// statement not erroring.
    ///
    /// This is the fact
    /// `repeated_arg_layouts_never_pair_conditional_binding_with_an_ssa_def_role`
    /// (`tcl-registry/tests/registry_sweep.rs`) checks against
    /// [`ArgRole`]: every generic write-detection site the compiler has
    /// (`tcl-compiler`'s `ssa::defs_of_with_registry`, `var_scoping`,
    /// `cfg_builder`, `lowering`, `ir_helpers`) resolves "does this argument
    /// write a variable" through `ArgRole::VarWrite`, so `VarWrite` is read
    /// everywhere as "this position unconditionally defines the named
    /// variable when the statement executes". A `conditional_binding: true`
    /// layout must never
    /// use it: doing so is exactly the mechanism that produced the `dict
    /// update` W210 false positive investigated for issue #1247/#1278 —
    /// `VarWrite` fed an unconditional SSA def, which pre-empted the
    /// key-aware suppression `harvest_dict_with_suppression`
    /// (`tcl-compiler/src/analyser/diagnostics/helpers.rs`) had already
    /// computed for the very same read.
    ///
    /// One consumer reads a *second* role as a definite def and subtracts
    /// this flag by hand: `ssa::defs_of_with_registry` takes an opaque
    /// [`ArgRole::LoopVarList`] barrier position as a loop-variable binding
    /// (issue #1380 — a `{*}`-expanded `foreach`/`lmap` barriers, and its
    /// body's reads of the loop variable would otherwise draw W210), and
    /// skips the whole role for a command whose layout declares this flag.
    /// That is why the check below stays keyed to `VarWrite` alone: a
    /// `conditional_binding: true` layout may safely use `LoopVarList`.
    pub conditional_binding: bool,
}

impl RepeatedArgLayout {
    /// A default-shaped layout: every word from `start`, nothing excluded,
    /// no optional leading word.
    #[must_use]
    pub const fn every(role: ArgRole, start: u8) -> Self {
        Self {
            role,
            start,
            stride: 1,
            exclude_trailing: 0,
            optional_leading_word: false,
            conditional_binding: false,
        }
    }

    /// A layout covering every `stride`th word from `start`.
    #[must_use]
    pub const fn strided(role: ArgRole, start: u8, stride: u8) -> Self {
        Self {
            role,
            start,
            stride,
            exclude_trailing: 0,
            optional_leading_word: false,
            conditional_binding: false,
        }
    }

    /// The covered argument indices for a call supplying `arg_count` words.
    ///
    /// Returns an empty vector for a degenerate declaration (`stride == 0`)
    /// or a call too short to reach `start`, so a caller never has to
    /// pre-check either.
    #[must_use]
    pub fn indices(self, arg_count: usize) -> Vec<usize> {
        let stride = usize::from(self.stride);
        if stride == 0 {
            return Vec::new();
        }
        let start = usize::from(self.start);
        let end = arg_count.saturating_sub(usize::from(self.exclude_trailing));
        if end <= start {
            return Vec::new();
        }
        // The optional leading word is invisible except in the count: anchor
        // the group to the last covered slot and read the shift back off it.
        let shift = if self.optional_leading_word {
            (end - start - 1) % stride
        } else {
            0
        };
        (start + shift..end).step_by(stride).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_covers_the_whole_tail() {
        // `global a b c`
        let l = RepeatedArgLayout::every(ArgRole::VarWrite, 0);
        assert_eq!(l.indices(3), vec![0, 1, 2]);
        assert_eq!(l.indices(0), Vec::<usize>::new());
    }

    #[test]
    fn strided_covers_name_value_pairs() {
        // `variable n v n v`
        let l = RepeatedArgLayout::strided(ArgRole::VarWrite, 0, 2);
        assert_eq!(l.indices(4), vec![0, 2]);
        // A trailing odd name with no value is still a name.
        assert_eq!(l.indices(5), vec![0, 2, 4]);
        assert_eq!(l.indices(1), vec![0]);
    }

    #[test]
    fn excluded_trailing_body_is_not_covered() {
        // `foreach v1 l1 v2 l2 body` — specs at 0, 2; the body at 4 is not.
        let l = RepeatedArgLayout {
            exclude_trailing: 1,
            ..RepeatedArgLayout::strided(ArgRole::LoopVarList, 0, 2)
        };
        assert_eq!(l.indices(5), vec![0, 2]);
        assert_eq!(l.indices(3), vec![0]);
        // Head + body only: nothing to cover, and no panic.
        assert_eq!(l.indices(1), Vec::<usize>::new());
        assert_eq!(l.indices(0), Vec::<usize>::new());
    }

    #[test]
    fn optional_leading_word_shifts_by_the_argument_count() {
        // `upvar ?level? other local ?other local ...?`
        let l = RepeatedArgLayout {
            optional_leading_word: true,
            ..RepeatedArgLayout::strided(ArgRole::VarWrite, 1, 2)
        };
        // No level: locals at 1, 3.
        assert_eq!(l.indices(2), vec![1]);
        assert_eq!(l.indices(4), vec![1, 3]);
        // A level word shifts everything one right: locals at 2, 4.
        assert_eq!(l.indices(3), vec![2]);
        assert_eq!(l.indices(5), vec![2, 4]);
        // Too short to name anything.
        assert_eq!(l.indices(1), Vec::<usize>::new());
    }

    #[test]
    fn degenerate_declarations_are_inert() {
        let zero_stride = RepeatedArgLayout {
            stride: 0,
            ..RepeatedArgLayout::every(ArgRole::VarWrite, 0)
        };
        assert_eq!(zero_stride.indices(4), Vec::<usize>::new());
        let past_end = RepeatedArgLayout::every(ArgRole::VarWrite, 9);
        assert_eq!(past_end.indices(4), Vec::<usize>::new());
    }
}
