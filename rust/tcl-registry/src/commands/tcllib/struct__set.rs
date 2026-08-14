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

//! `struct::set` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "contains",
        arity: Arity::exact(2),
        detail: "Test if set contains an element.",
        synopsis: "struct::set contains set element",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "difference",
        arity: Arity::exact(2),
        detail: "Return elements in A but not in B.",
        synopsis: "struct::set difference setA setB",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "empty",
        arity: Arity::exact(1),
        detail: "Test if a set is empty.",
        synopsis: "struct::set empty set",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "equal",
        arity: Arity::exact(2),
        detail: "Test if two sets are equal.",
        synopsis: "struct::set equal setA setB",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exclude",
        arity: Arity::exact(2),
        detail: "Remove elements from a set variable.",
        synopsis: "struct::set exclude setVar element",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "include",
        arity: Arity::exact(2),
        detail: "Add an element to a set variable.",
        synopsis: "struct::set include setVar element",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "intersect",
        arity: Arity::at_least(0),
        detail: "Return the intersection of two or more sets.",
        synopsis: "struct::set intersect ?set ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "subsetof",
        arity: Arity::exact(2),
        detail: "Test if A is a subset of B.",
        synopsis: "struct::set subsetof setA setB",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "symdiff",
        arity: Arity::exact(2),
        detail: "Return the symmetric difference of two sets.",
        synopsis: "struct::set symdiff setA setB",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "union",
        arity: Arity::at_least(0),
        detail: "Return the union of two or more sets.",
        synopsis: "struct::set union ?set ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Return the number of elements in a set.",
        synopsis: "struct::set size set",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "intersect3",
        arity: Arity::exact(2),
        detail: "Return a three-element list: elements only in A, elements in both, elements only in B.",
        synopsis: "struct::set intersect3 setA setB",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "add",
        arity: Arity::exact(2),
        detail: "Add elements of set B to set variable A.",
        synopsis: "struct::set add setVar setB",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "subtract",
        arity: Arity::exact(2),
        detail: "Remove elements of set B from set variable A.",
        synopsis: "struct::set subtract setVar setB",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "struct::set subcommand ?args ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::set",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Set operations on Tcl lists (union, intersect, difference, etc.).",
            synopsis: &["struct::set subcommand ?args ...?"],
            snippet: "",
            source: "tcllib struct::set package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        tcllib_package: Some("struct::set"),
        ..CommandSpec::DEFAULT
    }
}
