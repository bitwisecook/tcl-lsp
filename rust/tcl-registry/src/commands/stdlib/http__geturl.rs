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

//! `http::geturl` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::geturl",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Retrieve a URL — the primary command for the http package.",
            synopsis: &["http::geturl url ?options?"],
            snippet: "Retrieves the resource at *url* and returns a token that can be passed to the other ``http::`` commands.  Options include ``-query``, ``-headers``, ``-handler``, ``-command``, ``-timeout``, ``-type``, ``-method``, ``-keepalive`` and more.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        // `url` (arg 0) is a network-address arg — SSRF sink
        // (T104); `-headers` can carry credentials.
        taint_network_sink_args: Some(&[0]),
        credential_options: const { &["-headers"] },
        required_package: Some("http"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
