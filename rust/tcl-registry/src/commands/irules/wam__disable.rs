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

//! `WAM::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WAM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables Web Accelerator plugin processing on the connection.",
            synopsis: &["WAM::disable"],
            snippet: "Disables the WAM plugin for the current TCP connection. WAM will remain\ndisabled on the current TCP connection until it is closed or\nWAM::enable is called.",
            source: "https://clouddocs.f5.com/api/irules/WAM__disable.html",
            examples: "# Disable WAM for HTTP paths ending in .php\nwhen HTTP_REQUEST {\n  if { [HTTP::path] ends_with \".php\" } {\n    WAM::disable\n  } else {\n    WAM::enable\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "WAM::disable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("(removed)"),
        ..CommandSpec::DEFAULT
    }
}
