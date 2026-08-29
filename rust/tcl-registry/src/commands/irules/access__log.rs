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

//! `ACCESS::log` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::log",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Logs a message using APM logging framework.",
            synopsis: &["ACCESS::log (COMPONENT_LOGLEVEL)? MSG"],
            snippet: "ACCESS::log [component.][loglevel] <message>\n\nLogs the specified message using the optionally specified APM component name\nand log level as specified in the log setting for the access profile that is\nassigned to the virtual server.\nThe message is sent to the destination specified in the log setting.\nIf component is specified, it must be one of the supported values (see below)\nand must end with a dot character. If not specified, the accesscontrol component\nis assumed.\nIf log level is specified, it must be one of the supported values (see below).\nIf not specified, notice level is assumed.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__log.html",
            examples: "when HTTP_REQUEST {\n    ACCESS::log debug \"an Access Control debug log\"\n    ACCESS::log sso.error \"an SSO error log\"\n    ACCESS::log eca. \"an ECA notice log\"\n    ACCESS::log \"an Access Control notice log\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::log (COMPONENT_LOGLEVEL)? MSG",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
