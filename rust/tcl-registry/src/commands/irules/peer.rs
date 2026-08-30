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

//! `peer` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "peer",
        traits: Traits::IS_SIDE_SWITCH,
        side_switch_target: Some(SideSwitchTarget::Peer),
        surface: Some(SpecSurface::IRULES),
        // `peer NESTING_SCRIPT` — unlike clientside/serverside, peer has
        // no bare query form, so the script body is required: exactly
        // one argument at index 0 (#501).
        arity: Arity::new(1, 1),
        // The nesting script (index 0) is a body evaluated under the
        // peer-side context; it runs synchronously in the caller's
        // frame, so the default `BodyKind::Plain` applies.
        arg_roles: &[(0, ArgRole::Body)],
        hover: Some(HoverSnippet {
            summary: "Causes the specified iRule commands to be evaluated under the peer-side context.",
            synopsis: &["peer ANY_CHARS"],
            snippet: "Causes the specified iRule commands to be evaluated under the peer-side context.",
            source: "https://clouddocs.f5.com/api/irules/peer.html",
            examples: "when SERVER_CONNECTED {\n  peer { TCP::collect }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "peer ANY_CHARS",
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
