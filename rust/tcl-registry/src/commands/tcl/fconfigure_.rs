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

//! `fconfigure` — set and get options on a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fconfigure channelId ?optionName? ?value ...?",
}];

/// Command spec for `fconfigure`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fconfigure",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED | Traits::CONFIGURES_CHANNEL,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[
            OptionSpec {
                name: "-blocking",
                takes_value: true,
                value_hint: "boolean",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-buffering",
                takes_value: true,
                value_hint: "mode",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-buffersize",
                takes_value: true,
                value_hint: "size",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-encoding",
                takes_value: true,
                value_hint: "encoding",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-eofchar",
                takes_value: true,
                value_hint: "chars",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-translation",
                takes_value: true,
                value_hint: "mode",
                detail: "",
                dialects: None,
            },
            // Tcl 9.0+ socket / terminal options (TIPs 528 / 160).
            OptionSpec {
                name: "-nodelay",
                takes_value: true,
                value_hint: "boolean",
                detail: "Disable Nagle's algorithm on TCP sockets (Tcl 9.0+).",
                dialects: Some(DialectSet::TCL90_PLUS),
            },
            OptionSpec {
                name: "-keepalive",
                takes_value: true,
                value_hint: "boolean",
                detail: "Enable TCP keepalive on sockets (Tcl 9.0+).",
                dialects: Some(DialectSet::TCL90_PLUS),
            },
            OptionSpec {
                name: "-inputmode",
                takes_value: true,
                value_hint: "mode",
                detail: "Terminal input mode: normal/password/raw (Tcl 9.0+).",
                dialects: Some(DialectSet::TCL90_PLUS),
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Set and get options on a channel.",
            synopsis: &["fconfigure channelId ?optionName? ?value ...?"],
            snippet: "",
            source: "Tcl fconfigure(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
