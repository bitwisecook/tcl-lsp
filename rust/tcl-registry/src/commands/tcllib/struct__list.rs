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

//! `struct::list` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "assign",
        arity: Arity::at_least(2),
        detail: "Assign list elements to variables.",
        synopsis: "struct::list assign sequence var ?var ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dbJoin",
        arity: Arity::at_least(2),
        detail: "Perform a relational join on two lists.",
        synopsis: "struct::list dbJoin ?-inner|-left|-right|-full? ?-keys varname? keyedList1 keyedList2",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dbJoinKeyed",
        arity: Arity::at_least(2),
        detail: "Relational join on keyed lists.",
        synopsis: "struct::list dbJoinKeyed ?options? keyedList1 keyedList2",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "equal",
        arity: Arity::at_least(2),
        detail: "Test if two lists are structurally equal.",
        synopsis: "struct::list equal ?-simple? a b",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "filter",
        arity: Arity::exact(2),
        detail: "Return elements matching a condition.",
        synopsis: "struct::list filter sequence cmdprefix",
        // cmdprefix invoked as `cmdprefix element` → 1 appended arg.
        command_prefixes: &[(1, AppendedArity::Exactly(1))],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "filterfor",
        arity: Arity::exact(3),
        detail: "Filter using an expression over each element.",
        synopsis: "struct::list filterfor var sequence expr",
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Expr)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "flatten",
        arity: Arity::at_least(1),
        detail: "Flatten nested lists by one or more levels.",
        synopsis: "struct::list flatten ?-full? ?--? sequence",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "fold",
        arity: Arity::exact(3),
        detail: "Left-fold a list with an accumulator.",
        synopsis: "struct::list fold sequence initialValue cmdprefix",
        // cmdprefix invoked as `cmdprefix accumulator element` → 2 appended args.
        command_prefixes: &[(2, AppendedArity::Exactly(2))],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foreachperm",
        arity: Arity::exact(3),
        detail: "Iterate over all permutations of a list.",
        synopsis: "struct::list foreachperm var sequence body",
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iota",
        arity: Arity::exact(1),
        detail: "Generate a list of integers 0..n-1.",
        synopsis: "struct::list iota n",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lcsInvert",
        arity: Arity::exact(3),
        detail: "Invert a longest-common-subsequence result.",
        synopsis: "struct::list lcsInvert lcsData len1 len2",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "longestCommonSubsequence",
        arity: Arity::new(2, 3),
        detail: "Find the longest common subsequence of two lists.",
        synopsis: "struct::list longestCommonSubsequence list1 list2 ?maxOccurs?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::exact(2),
        detail: "Apply a command to each element and collect results.",
        synopsis: "struct::list map sequence cmdprefix",
        // cmdprefix invoked as `cmdprefix element` → 1 appended arg.
        command_prefixes: &[(1, AppendedArity::Exactly(1))],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mapfor",
        arity: Arity::exact(3),
        detail: "Map each element through a script and collect the results.",
        synopsis: "struct::list mapfor var sequence script",
        // Unlike `filterfor`, mapfor's third argument is a Tcl script
        // evaluated once per element (in the caller's frame), not an
        // expression — so it recurses as a body.
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "repeat",
        arity: Arity::at_least(2),
        detail: "Create a list by repeating elements.",
        synopsis: "struct::list repeat count element ?element ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reverse",
        arity: Arity::exact(1),
        detail: "Reverse the order of a list.",
        synopsis: "struct::list reverse sequence",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "shift",
        arity: Arity::exact(1),
        detail: "Remove and return the first element.",
        synopsis: "struct::list shift listVar",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "shuffle",
        arity: Arity::exact(1),
        detail: "Randomly reorder elements of a list.",
        synopsis: "struct::list shuffle list",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "swap",
        arity: Arity::exact(3),
        detail: "Swap two elements in a list.",
        synopsis: "struct::list swap listVar i j",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "longestCommonSubsequence2",
        arity: Arity::new(2, 3),
        detail: "Find the longest common subsequence (alternate algorithm).",
        synopsis: "struct::list longestCommonSubsequence2 list1 list2 ?maxOccurs?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lcsInvertMerge",
        arity: Arity::exact(3),
        detail: "Invert and merge a longest-common-subsequence result.",
        synopsis: "struct::list lcsInvertMerge lcsData len1 len2",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "split",
        arity: Arity::new(2, 4),
        detail: "Split a list using a command prefix as filter.",
        synopsis: "struct::list split sequence cmdprefix ?passVar? ?failVar?",
        // The twin of `filter`: cmdprefix invoked as `cmdprefix element` → 1
        // appended arg.  `passVar`/`failVar` are result out-variables, not
        // prefixes.
        command_prefixes: &[(1, AppendedArity::Exactly(1))],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::exact(2),
        detail: "Delete an element from a list variable by value.",
        synopsis: "struct::list delete listVar item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "repeatn",
        arity: Arity::at_least(1),
        detail: "Create a (nested) list by repeating a value.",
        synopsis: "struct::list repeatn value count ?count ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "firstperm",
        arity: Arity::exact(1),
        detail: "Return the first permutation of a list.",
        synopsis: "struct::list firstperm list",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextperm",
        arity: Arity::exact(1),
        detail: "Return the next permutation in lexicographic order.",
        synopsis: "struct::list nextperm perm",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "permutations",
        arity: Arity::exact(1),
        detail: "Return all permutations of a list.",
        synopsis: "struct::list permutations list",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "struct::list subcommand ?args ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::list",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Advanced list manipulation commands.",
            synopsis: &["struct::list subcommand ?args ...?"],
            snippet: "Provides operations beyond the core Tcl list commands: filtering, mapping, folding, shuffling, permutations, longest-common-subsequence, and more.",
            source: "tcllib struct::list package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        tcllib_package: Some("struct::list"),
        ..CommandSpec::DEFAULT
    }
}
