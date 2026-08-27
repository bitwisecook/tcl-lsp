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

//! `XLAT::src_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Retrieve the source translation port.",
            synopsis: &["XLAT::src_port"],
            snippet: "Retrieve the source translation port.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_port.html",
            examples: "when SA_PICKED {\n    log local0. \"[XLAT::src_port]\"\n}",
            return_value: "Return the source translation port.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["SA_PICKED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "XLAT::src_port",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
