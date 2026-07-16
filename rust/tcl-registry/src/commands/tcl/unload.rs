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

//! `unload` — unload a shared library extension.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nocomplain",
        value: OptionValue::flag(),
        detail: "Suppresses all error messages.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-keeplibrary",
        value: OptionValue::flag(),
        detail: "This switch will prevent unload from issuing the operating system call that will unload the library from the process.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Marks the end of switches.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unload ?switches? fileName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unload",
        dialects: None,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Unload machine code",
            synopsis: &[
                "unload ?switches? fileName",
                "unload ?switches? fileName prefix",
                "unload ?switches? fileName prefix interp",
                "unload ?-keeplibrary? ?-nocomplain? fileName ?prefix? ?interp?",
            ],
            snippet: "This command tries to unload shared libraries previously loaded with load from the application's address space.",
            source: "Tcl man page unload.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
