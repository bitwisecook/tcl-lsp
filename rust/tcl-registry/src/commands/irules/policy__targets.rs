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

//! `POLICY::targets` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::targets",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets properties of the policy rule targets for the policies associated with the virtual server that the iRule is enabled on.",
            synopsis: &["POLICY::targets ('ltm-policy' |"],
            snippet: "Returns or sets properties of the policy rule targets for the policies\nassociated with the virtual server that the iRule is enabled on. A\npolicy rule target can be considered an action that the policy uses if\nthe rule conditions are met.\n\nAs of v11.4 the following policy targets are available:\n wam              - Application Acceleration Manager (AAM)\n asm              - Application Security Manager\n log              - Log\n http-cookie      - HTTP cookie\n http-header      - HTTP header\n http-host        - HTTP host header\n http-referer     - HTTP referer header",
            source: "https://clouddocs.f5.com/api/irules/policy__targets.html",
            examples: "# Log the policy targets for this virtual server\nwhen HTTP_REQUEST {\n\n        # Log the policy targets enabled on this virtual server\n        log local0. \"\\[POLICY::targets\\]: [POLICY::targets]\"\n\n        # Loop through each possible target type and log whether it is enabled or not (1 for enabled, 0 for not enabled)\n        foreach target {asm wam log http-cookie http-header http-host http-referer http-set-cookie http-uri log tcl tcp-nagle} {",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "POLICY::targets ('ltm-policy' |",
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
