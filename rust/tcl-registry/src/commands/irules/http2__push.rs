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

//! `HTTP2::push` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::push",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Accepts a resource as a parameter that can be pushed to the client using PUSH_PROMISE frames in HTTP/2 stream.",
            synopsis: &[
                "HTTP2::push <uri> ?-priority num? ?-nohost? <request headers>",
                "HTTP2::push <uri> ?-priority num? ?-content data | -ifile file? ?-noserver? ?-nohost? <request headers> -- <response headers>",
            ],
            snippet: "This command has two variants.\n\nThe first takes a requested resource, and then sends a PUSH_PROMISE frame describing that resource to the client.  The resource is requested from the server, and the payload is sent to the client on the pushed stream.\n\nThe second method of using this command describes both the request and the response.  The request is sent as a PUSH_PROMISE to the client, and the response follows.  The server is not contacted, and the content is pushed directly from the BigIP.\n\nNote that this command may cause iRule events to trigger on the newly pushed stream.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__push.html",
            examples: "when HTTP_REQUEST {\n    HTTP2::push /index.html host example.com\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP2::push <uri> ?options? ?request headers ...? ?-- response headers ...?",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-priority",
                    value: OptionValue::value("PRIORITY"),
                    detail: "Push priority number.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-content",
                    value: OptionValue::value("CONTENT"),
                    detail: "Pushed response content.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-ifile",
                    value: OptionValue::value("IFILE_OBJ"),
                    detail: "Serve content from iFile.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-noserver",
                    value: OptionValue::flag(),
                    detail: "Suppress \"Server: BigIP\" header.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-nohost",
                    value: OptionValue::flag(),
                    detail: "Disable Host header requirement.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::Http2State,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
