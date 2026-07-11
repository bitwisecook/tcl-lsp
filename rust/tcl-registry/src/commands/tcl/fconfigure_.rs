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
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-blocking",
        value: OptionValue::value("boolean"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffering",
        value: OptionValue::value("mode"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffersize",
        value: OptionValue::value("size"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-encoding",
        value: OptionValue::value("encoding"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-eofchar",
        value: OptionValue::value("chars"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-translation",
        value: OptionValue::value("mode"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // Tcl 9.0+ socket / terminal options (TIPs 528 / 160).
    OptionSpec {
        name: "-nodelay",
        value: OptionValue::value("boolean"),
        detail: "Disable Nagle's algorithm on TCP sockets (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-keepalive",
        value: OptionValue::value("boolean"),
        detail: "Enable TCP keepalive on sockets (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-inputmode",
        value: OptionValue::value("mode"),
        detail: "Terminal input mode: normal/password/raw (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-profile",
        value: OptionValue::value("profile"),
        detail: "Encoding error profile: strict/tcl8/replace (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
];

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
        options: OPTIONS,
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
