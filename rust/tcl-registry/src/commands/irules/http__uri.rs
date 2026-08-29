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

//! `HTTP::uri` iRules command.
use crate::prelude::*;
use crate::taint::SetterConstraint;
use tcl_dialect::model::{SpecSurface};

/// The setter form of `HTTP::uri` requires its value to start
/// with `/` (IRULE3101). Registry-driven replacement for the hardcoded
/// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`.
const SETTER_CONSTRAINTS: &[SetterConstraint] = &[SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: tcl_core_types::DiagCode::Irule3101,
    message: "HTTP::uri value must start with '/'",
}];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::uri",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION)
            .union(Traits::UNNORMALISED_HTTP_GETTER),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::new(0, 1),
        options: const {
            &[OptionSpec {
                name: "-normalized",
                value: OptionValue::flag(),
                detail: "Return the canonicalised URI (URL evasion patterns rejected).",
                surface: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Returns or sets the URI part of the HTTP request.",
            synopsis: &["HTTP::uri (URI)?"],
            snippet: "Returns or sets the URI part of the HTTP request. This command replaces\nthe BIG-IP 4.X variable http_uri.\n\nFor the following URL:\nhttp://www.example.com:8080/main/index.jsp?user=test&login=check\nThe URI is: /main/index.jsp?user=test&login=check\n\nNote that in the HTTP_PROXY_REQUEST event, this command returns the complete\nproxy URI. This includes the scheme, host and port, and thus the result would be:\nhttp://www.example.com:8080/main/index.jsp?user=test&login=check",
            source: "https://clouddocs.f5.com/api/irules/HTTP__uri.html",
            examples: "when HTTP_PROXY_REQUEST {\n   log local.0 \"This proxy request is:[HTTP::uri]\"\n}",
            return_value: "Returns the URI part of the HTTP request.",
        }),
        setter_constraints: SETTER_CONSTRAINTS,
        // Measured on the appliance: the rule compiler refuses
        // `HTTP::uri` in `HTTP_RESPONSE` with `command is not valid in
        // current event context (HTTP_RESPONSE)`, even though the event
        // carries an HTTP profile — the request URI is simply not
        // addressable once the response is in hand
        // (`docs/design/bigip-irule-parser-measurements.md` §8, which
        // names this cell as *"exactly the mistakes an editor should
        // catch"*).
        excluded_events: &["HTTP_RESPONSE"],
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            // `LB_SELECTED` implies no HTTP profile, yet the rule
            // compiler accepts `HTTP::uri` there (§8) — an unconditional
            // event, like the `MR_*` and `SERVER_CONNECTED` rows beside
            // it.
            also_in: &[
                "LB_SELECTED",
                "MR_EGRESS",
                "MR_FAILED",
                "MR_INGRESS",
                "SERVER_CONNECTED",
            ],
            flow: false,
        }),
        forms: &[
            FormSpec {
                kind: FormKind::Getter,
                synopsis: "HTTP::uri ?-normalized?",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                kind: FormKind::Setter,
                synopsis: "HTTP::uri <URI>",
                ..FormSpec::DEFAULT
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PATH_PREFIXED)),
        ..CommandSpec::DEFAULT
    }
}
