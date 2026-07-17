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

//! `socket` — open a TCP network connection.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-myaddr",
        value: OptionValue::value("addr"),
        detail: "Client-side local interface.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-myport",
        value: OptionValue::value("port"),
        detail: "Client-side local port.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-async",
        value: OptionValue::flag(),
        detail: "Connect asynchronously.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-server",
        // Accept callback invoked as `command channel addr port` → 3 args.
        value: OptionValue::command_prefix_n("command", AppendedArity::Exactly(3)),
        detail: "Server accept callback.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-reuseaddr",
        value: OptionValue::value("boolean"),
        detail: "Allow server address reuse (default 1).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-reuseport",
        value: OptionValue::value("boolean"),
        detail: "Allow server port reuse (default 0).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "socket ?options? host port",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "socket -server command ?options? port",
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "socket",
        dialects: None,
        traits: Traits::BYTE_COMPILED
            | Traits::OPENS_CHANNEL
            | Traits::TAINT_SOURCE
            | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Channel),
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Open a TCP client or server socket channel.",
            synopsis: &[
                "socket ?options? host port",
                "socket -server command ?options? port",
                "socket -server command ?-option value ...? port",
                "socket ?-option value ...? host port",
            ],
            snippet: "`-server` creates a listening socket and invokes the callback with `channel clientaddr clientport` for each accepted connection.",
            source: "Tcl socket(3tcl)",
            examples: "",
            return_value: "",
        }),
        // `host`/`port` are network-address args — SSRF sink
        // (T104).
        taint_network_sink_args: Some(&[0, 1]),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
