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

//! `SDP::field` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or Sets the value in a given SDP field.",
            synopsis: &["SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?"],
            snippet: "This command will get or set the value of a specific SDP field",
            source: "https://clouddocs.f5.com/api/irules/SDP__field.html",
            examples: "when SIP_REQUEST {\n    log local0. \"SDP field b: [SDP::field b]\"\n    SDP::field c 0 \"IN IP4 10.10.1.150\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
