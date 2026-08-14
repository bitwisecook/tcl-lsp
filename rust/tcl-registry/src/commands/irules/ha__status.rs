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

//! `HA::status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HA::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns true or false based on whether the unit the command is executed on is active or standby.",
            synopsis: &["HA::status (active | standby)"],
            snippet: "This iRule command returns true or false based on whether the unit the\ncommand is executed on is active or standby in the context of the\ncommand used. The primary use-case is for iRules that utilize sideband\nor HSL commands. This can be used to prevent the standby from opening\nextra connections.\nA Virtual IP (VIP) is bound to a Traffic Group, which handles failover\nfor the VIP. A unit can, at the same time, be \"active\" for one\ntraffic-group and \"standby\" for a different traffic-group.",
            source: "https://clouddocs.f5.com/api/irules/HA__status.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"active: [HA::status active]\"\n    log local0. \"standby: [HA::status standby]\"\n}",
            return_value: "HA::status active",
        }),
        excluded_events: &["RULE_INIT"],
        forms: &[FormSpec {
            synopsis: "HA::status (active | standby)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
