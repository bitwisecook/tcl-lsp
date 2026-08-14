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

//! `BIGPROTO::enable_fix_reset` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BIGPROTO::enable_fix_reset",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable or Disable Reset of FIX Protocol Connections",
            synopsis: &["BIGPROTO::enable_fix_reset BOOLEAN"],
            snippet: "When set to disabled, TCP RST frame will not be sent when BIG-IP detects there is a hash collision on ePVA offloading of FIX flows. Instead, it will try to re-offload the connection.",
            source: "https://clouddocs.f5.com/api/irules/BIGPROTO__enable_fix_reset.html",
            examples: "when CLIENT_ACCEPTED {\n    BIGPROTO::enable_fix_reset true\n    BIGPROTO::enable_fix_reset false\n            }",
            return_value: "none",
        }),
        forms: &[FormSpec {
            synopsis: "BIGPROTO::enable_fix_reset BOOLEAN",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
