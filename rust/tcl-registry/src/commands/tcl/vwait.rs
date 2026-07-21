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

//! `vwait` — wait for a variable to be modified.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Marks the end of options.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "All conditions for the wait operation must be met to complete the wait operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-extended",
        value: OptionValue::flag(),
        detail: "An extended result in list form is returned, see below for explanation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nofileevents",
        value: OptionValue::flag(),
        detail: "File events are not handled in the wait operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-noidleevents",
        value: OptionValue::flag(),
        detail: "Idle handlers are not invoked during the wait operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-notimerevents",
        value: OptionValue::flag(),
        detail: "Timer handlers are not serviced during the wait operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nowindowevents",
        value: OptionValue::flag(),
        detail: "Events of the windowing system are not handled during the wait operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-readable",
        value: OptionValue::value(""),
        detail: "Channel must name a Tcl channel open for reading.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value(""),
        detail: "The wait operation is constrained to milliseconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::var_name(),
        detail: "VarName must be the name of a global variable.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-writable",
        value: OptionValue::value(""),
        detail: "Channel must name a Tcl channel open for writing.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vwait varName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vwait",
        dialects: None,
        traits: Traits::BYTE_COMPILED
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::exact(1),
        // `VarWrite`, not `VarRead`: `Tcl_VwaitObjCmd` (tclEvent.c) never
        // reads the variable's value — it installs a
        // `TCL_TRACE_WRITES|TCL_TRACE_UNSETS` trace (creating the entry when
        // absent) and returns only once the variable has been *written* by
        // an event handler.  `vwait forever` on a never-set variable is the
        // canonical infinite-wait idiom, so modelling the operand as a read
        // produced a false W210 (read-before-set); as a write the post-state
        // is a defined variable, which is what the analyser records.
        arg_roles: &[(0, ArgRole::VarWrite)],
        // The value observed after the wait is whatever the event handler
        // stored — unknowable statically — so the written variable is typed
        // overdefined, never from vwait's own (empty-string) return type.
        var_write_typing: VarWriteTyping::Destructured,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Process events until a variable is written",
            synopsis: &["vwait varName", "vwait ?options? ?varName ...?"],
            snippet: "This command enters the Tcl event loop to process events, blocking the application if no events are ready.",
            source: "Tcl man page vwait.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
