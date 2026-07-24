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

//! `RADIUS::avp` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::avp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns or adds/changes/removes RADIUS attribute-value pairs.",
            synopsis: &[
                "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                "RADIUS::avp 'insert' (ATTR_NAME|ATTR_CODE)",
            ],
            snippet: "This command returns or adds/changes/removes RADIUS attribute-value pairs. Radius profile must be applied for access to this command.",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__avp.html",
            examples: "when RULE_INIT {\n        set static::secret \"linus\"\n    }",
            return_value: "RADIUS::avp attr [attr_type] Returns the value of the specified RADIUS attribute. optional attr_type = ( octet | ip4 | ip6 | integer | string)",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_CLOSED",
                "CLIENT_DATA",
                "SERVER_CLOSED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
