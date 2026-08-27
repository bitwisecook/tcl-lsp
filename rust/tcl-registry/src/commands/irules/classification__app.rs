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

//! `CLASSIFICATION::app` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::app",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: Provides classification for the most explicit application name.",
            synopsis: &["CLASSIFICATION::app"],
            snippet: "This command provides classification for the most explicit application\nname. (Example: cnn, amazon)\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFICATION::app",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__app.html",
            examples: "when CLASSIFICATION_DETECTED {\n  if { [CLASSIFICATION::app] equals \"application1\"}  {\n    drop\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CLASSIFICATION"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "CLASSIFICATION::app",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
