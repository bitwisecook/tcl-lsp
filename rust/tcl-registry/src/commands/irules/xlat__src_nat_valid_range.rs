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

//! `XLAT::src_nat_valid_range` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_nat_valid_range",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Return a list of valid source-translation endpoint ranges.",
            synopsis: &["XLAT::src_nat_valid_range"],
            snippet: "Returns a list of lists containing valid source-translation addresses and port-ranges (source-translation endpoints). This command must be called every time a new connection/listener needs to be created to retrieve the valid source translation information. This data must not be cached. Only the source-translation address used by the parent with a valid port-range is returned, not all of the endpoints in the source-translation object/pool. In PBA mode multiple source-translation addresses and port-ranges can be returned if the client has multiple active blocks.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_nat_valid_range.html",
            examples: "when SA_PICKED {\n    set sa_list [XLAT::src_nat_valid_range]\n\n    foreach sa $sa_list {\n        log local0. \"address=[lindex $sa 0] port-start=[lindex $sa 1] port-end=[lindex $sa 2]\"\n    }\n}",
            return_value: "A list of lists containing the valid source-translation endpoint ranges. For e.g. { {address-1 start-port end-port} {address-2 start-port end-port} ...}",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_DATA",
                "SA_PICKED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "XLAT::src_nat_valid_range",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
