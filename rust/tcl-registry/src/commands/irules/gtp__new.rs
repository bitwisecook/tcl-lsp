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

//! `GTP::new` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::new",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a new GTP message for given version & request-type.",
            synopsis: &["GTP::new VERSION TYPE"],
            snippet: "Creates a new GTP message for given version & request-type.\nValid values for version are 1 or 2 only.\nRequest-type: A value less than 256.\nReturns a TCL object of type \"GTP-Message\"",
            source: "https://clouddocs.f5.com/api/irules/GTP__new.html",
            examples: "when CLIENT_ACCEPTED {\n    set t2 [GTP::new 2 10]\n    log local0. \"GTP version [GTP::header version -message $t2]\"\n    log local0. \"GTP type [GTP::header type -message $t2]\"\n}",
            return_value: "Returns a TCL object of type \"GTP-Message\"",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::new VERSION TYPE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
