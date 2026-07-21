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

//! `snat` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "snat",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Assigns the specified SNAT translation address to the current connection.",
            synopsis: &["snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))"],
            snippet: "Causes the system to assign the specified source address to the\nserverside connection(s). The assignment is valid for the duration of\nthe clientside connection or until 'snat none' is called. The iRule\nSNAT command overrides the SNAT configuration of the virtual server or\na SNAT pool. It does not override the 'Allow SNAT' setting of a pool.",
            source: "https://clouddocs.f5.com/api/irules/snat.html",
            examples: "# Apply SNAT autmap if the selected pool member IP address is 1.1.1.1\nwhen LB_SELECTED {\nIf { [IP::addr [LB::server addr] equals 1.1.1.1] } {\n     snat automap\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP", "MR"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SnatSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
