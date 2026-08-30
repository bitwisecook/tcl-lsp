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

//! `MR::stream` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::stream",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Start egressing bytes previously collected and stored.",
            synopsis: &["MR::stream ( 'end' )? (BYTES)"],
            snippet: "Start egressing bytes previously collected and stored say in sessionDB. If payload has been split in multiple segments, use end to indicate the final segment.\n\nSYNTAX\n\nMR::stream <payload>\n    Stream payload segment.\n\nMR::stream end <payload>\n    Stream payload segement. End indicates final segment.",
            source: "https://clouddocs.f5.com/api/irules/MR__stream.html",
            examples: "when MR_EGRESS {\n    MR::stream end \"abcd\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "MR::stream ( 'end' )? (BYTES)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
