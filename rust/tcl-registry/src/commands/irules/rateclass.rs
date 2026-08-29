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

//! `rateclass` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "rateclass",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Selects the specified rate class to use when transmitting packets.",
            synopsis: &["rateclass RATE_CLASS"],
            snippet: "Causes the system to select the specified rate class to use when\ntransmitting packets.",
            source: "https://clouddocs.f5.com/api/irules/rateclass.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::addr [IP::client_addr] equals xxx.xxx.xxx.xxx] } {\n    log local0. \"[IP::client_addr] being handled by rateclass class1\"\n    rateclass class1\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "rateclass RATE_CLASS",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
