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

//! `SSL::profile` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::profile",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Switch between different SSL profiles.",
            synopsis: &["SSL::profile PROFILE_OBJ"],
            snippet: "This command allows you to switch between SSL profiles, both client and server. Note: This should be done before the SSL negotiation occurs, or your rule will require the use of the SSL::renegotiate command.\n\nIn order to switch SSL profiles, a profile must be assigned to the virtual to begin with; switching the clientssl profile requires an existing clientssl profile, and similarly for serverssl profiles. You can also use SSL::disable to use SSL selectively.",
            source: "https://clouddocs.f5.com/api/irules/SSL__profile.html",
            examples: "when HTTP_REQUEST {\n    SSL::renegotiate\n}",
            return_value: "SSL::profile <profile_name> Switch to the defined SSL profile.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::profile PROFILE_OBJ",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
