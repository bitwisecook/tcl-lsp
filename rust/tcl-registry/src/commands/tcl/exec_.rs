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

//! `exec` — invoke subprocesses.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exec ?switches? arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exec",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SINK | Traits::TAINT_SOURCE | Traits::UNSAFE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Process,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::None,
            },
            // INTERP_STATE.
            SideEffect {
                target: SideEffectTarget::InterpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
        // ``--`` is the option terminator that drives W304's
        // ``resolve_option_terminator`` lookup; the registry also
        // surfaces the two boolean switches for completion.
        options: const {
            &[
                OptionSpec {
                    name: "-encoding",
                    value: OptionValue::value("encodingName"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-ignorestderr",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-keepnewline",
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
        hover: Some(HoverSnippet::brief(
            "Invoke subprocesses.",
            &["exec ?-option ...? arg ?arg ...?"],
            "Tcl exec(1)",
        )),
        // A `SHELL_ATOM`-coloured value is token-safe and
        // suppresses T100.
        taint_sink_safe_colour: Some(TaintColour::SHELL_ATOM),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
