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

//! `ADAPT::allow` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::allow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns the value of a boolean property.",
            synopsis: &["ADAPT::allow (ADAPT_CTX)? ('http_v1.0') (ADAPT_SIDE)? (BOOLEAN)?"],
            snippet: "The ADAPT::allow command sets or returns the value of one\nof a set of boolean 'allow' properties for the current or\nspecified side of the virtual server connection for which\nthe iRule is being executed. They are not part of the profile\nand therefore cannot be accessed via tmsh or the GUI.\n\nSyntax:\n\nADAPT::allow [<context>] <property>\n\n    * Gets the property value for the current side\n\nADAPT::allow [<context>] <property> request\n\n    * Gets the property value for the request-adapt side\n\nADAPT::allow [<context>] <property> response\n\n    * Gets the property value for the response-adapt side",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__allow.html",
            examples: "when HTTP_RESPONSE {\n    ADAPT::allow http_v1.0 yes\n}",
            return_value: "Returns the current of modified value of the property.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ADAPT::allow (ADAPT_CTX)? ('http_v1.0') (ADAPT_SIDE)? (BOOLEAN)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
