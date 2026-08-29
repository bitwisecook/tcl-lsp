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

//! `LSN::pool` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::pool",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Explicitly set the LSN pool used for translation.",
            synopsis: &["LSN::pool LSN_POOL"],
            snippet: "Explicitly set the LSN pool used for translation.\n\nLSN::pool <pool_name>",
            source: "https://clouddocs.f5.com/api/irules/LSN__pool.html",
            examples: "",
            return_value: "LSN::pool",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP", "MR", "RTSP", "SIP"],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_DATA",
                "LB_FAILED",
                "LB_SELECTED",
                "SA_PICKED",
            ],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "LSN::pool LSN_POOL",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
