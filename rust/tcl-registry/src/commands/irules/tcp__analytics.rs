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

//! `TCP::analytics` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::analytics",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to categorize the connection for statistics collection purposes.",
            synopsis: &["TCP::analytics (enable | disable | key (KEY)?)"],
            snippet: "Enables or disables AVR TCP stat reporting (\"analytics\") for this connection and/or assigns user-defined keys.\n\nTCP::analytics enable\n    Enables analytics on this connection. AVR must be provisioned and the virtual must have a tcp-analytics profile attached. Collection will use the configuration in the profile. If the profile is configured to disable analytics by default, this gives users the ability to collect statistics by exception only.\n\nTCP::analytics disable\n    Disables analytics on this connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__analytics.html",
            examples: "rt collection for one subnet only.\n     when CLIENT_ACCEPTED {\n         if [IP::addr [IP::client_addr]/8 equals 10.0.0.0] {\n             TCP::analytics enable\n         }\n     }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::analytics (enable | disable | key (KEY)?)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
