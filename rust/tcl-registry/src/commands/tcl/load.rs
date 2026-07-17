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

//! `load` — load a shared library extension.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "load ?-global? ?-lazy? ?--? fileName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load",
        dialects: None,
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        // ``--`` is the option terminator that drives W304's
        // ``resolve_option_terminator`` lookup; the registry also
        // surfaces ``-global`` / ``-lazy`` for completion.
        options: const {
            &[
                OptionSpec {
                    name: "-global",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-lazy",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Load machine code and initialize new commands",
            synopsis: &[
                "load ?-global? ?-lazy? ?--? fileName",
                "load ?-global? ?-lazy? ?--? fileName prefix",
                "load ?-global? ?-lazy? ?--? fileName prefix interp",
                "load fileName ?prefix? ?interp?",
            ],
            snippet: "This command loads binary code from a file into the application's address space and calls an initialization procedure in the library to incorporate it into an interpreter.",
            source: "Tcl man page load.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Load),
        ..CommandSpec::DEFAULT
    }
}
