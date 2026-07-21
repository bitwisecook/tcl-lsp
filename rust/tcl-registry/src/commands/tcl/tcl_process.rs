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

//! `tcl::process` — subprocess management ensemble (Tcl 9.0+, TIP 462).
//!
//! Pure-host capability — never reachable in WASM/WASI builds (no
//! process model), so the registration exists for LSP recognition
//! and the runtime drops it on the trapping-stub list.  Two name
//! forms are registered for the namespace-relative and fully-
//! qualified spellings.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tcl::process subcommand ?arg ...?",
    dialects: None,
}];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet::brief(
            "Manage subprocesses started by exec / open |cmd.",
            &["tcl::process subcommand ?arg ...?"],
            "Tcl man page process.n",
        )),
        ..CommandSpec::DEFAULT
    }
}

static STATUS_OPTIONS: [OptionSpec; 2] = [
    OptionSpec {
        name: "-wait",
        value: OptionValue::flag(),
        detail: "Block until status is available.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Marks end of switches.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

static SUBCOMMANDS: [SubCommand; 4] = [
    SubCommand {
        name: "autopurge",
        arity: Arity::new(0, 1),
        detail: "Query (no args) or set (boolean) the autopurge flag.",
        synopsis: "tcl::process autopurge ?flag?",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::new(0, 0),
        detail: "Return list of subprocess PIDs.",
        synopsis: "tcl::process list",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "purge",
        arity: Arity::new(0, 1),
        detail: "Purge terminated subprocesses (optionally filtered by PID list).",
        synopsis: "tcl::process purge ?pids?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "status",
        arity: Arity::at_least(0),
        detail: "Return dict mapping PID -> status. `-wait` blocks until status available.",
        synopsis: "tcl::process status ?switches? ?pids?",
        return_type: Some(TclType::Dict),
        options: &STATUS_OPTIONS,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the namespace-relative form `tcl::process`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::process")
}

/// Command spec for the fully-qualified form `::tcl::process`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::process")
}
