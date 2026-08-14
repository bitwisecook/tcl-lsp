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

//! `SSL::maximum_record_size` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::maximum_record_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the maximum egress record size.",
            synopsis: &["SSL::maximum_record_size (SSL_RECORD_SIZE)?"],
            snippet: "SSL::maximum_record_size\n  Returns the currently set maximum egress record size.\nSSL::maximum_record_size #####\n  Set the maximum egress record size.",
            source: "https://clouddocs.f5.com/api/irules/SSL__maximum_record_size.html",
            examples: "when CLIENT_ACCEPTED {\n    SSL::maximum_record_size 1234\n}",
            return_value: "SSL::maximum_record_size Returns the currently set maximum egress record size. SSL::maximum_record_size ##### There is no return value.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::maximum_record_size (SSL_RECORD_SIZE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
