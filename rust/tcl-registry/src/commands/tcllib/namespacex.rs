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

//! tcllib `namespacex` package — namespace-manipulation utilities.
//!
//! `namespacex` is an ensemble whose `hook`, `import`, `info`, and `state`
//! sub-commands take a further method word (`namespacex hook add`,
//! `namespacex state get`, …).  The registry models the first-level
//! sub-commands and documents the second-level methods in each synopsis.
//! Public command surface from `modules/namespacex/namespacex.man`.
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// The first-level `namespacex` sub-commands.
const SUBS: &[SubCommand] = &[
    SubCommand {
        name: "hook",
        arity: Arity::at_least(1),
        detail: "Manage a namespace command-resolution hook (add/proc/on/next).",
        synopsis: "namespacex hook add|proc|on|next ?namespace? ...",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "import",
        arity: Arity::at_least(1),
        detail: "Import commands from another namespace (fromns).",
        synopsis: "namespacex import fromns src ?cmd ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::at_least(1),
        detail: "Query namespace structure (allchildren/allvars/vars).",
        synopsis: "namespacex info allchildren|allvars|vars namespace",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "state",
        arity: Arity::at_least(1),
        detail: "Save, restore, or drop a namespace's variable state (get/set/drop).",
        synopsis: "namespacex state get|set|drop namespace ?state?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "normalize",
        arity: Arity::exact(1),
        detail: "Normalise a namespace name to its fully-qualified form.",
        synopsis: "namespacex normalize name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "strip",
        arity: Arity::exact(2),
        detail: "Strip a leading namespace prefix from a fully-qualified name.",
        synopsis: "namespacex strip prefix name",
        ..SubCommand::DEFAULT
    },
];

/// The `namespacex` ensemble command spec.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "namespacex",
        arity: Arity::at_least(1),
        subcommands: SUBS,
        hover: Some(HoverSnippet::brief(
            "Namespace-manipulation utilities (hook/import/info/state/normalize/strip).",
            &["namespacex subcommand ?arg ...?"],
            "tcllib namespacex package",
        )),
        tcllib_package: Some("namespacex"),
        required_package: Some("namespacex"),
        ..CommandSpec::DEFAULT
    }
}
