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

//! `ASM::disable` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::disable",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables plugin processing on the connection.",
            synopsis: &["ASM::disable"],
            snippet: "Disables the ASM plugin processing for the current TCP connection.\nASM will remain disabled on the current TCP connection until it is closed or\nASM::enable is called.",
            source: "https://clouddocs.f5.com/api/irules/ASM__disable.html",
            examples: "# for 11.4.0+ the command should be used in HTTP_REQUEST event\nwhen HTTP_CLASS_SELECTED {\n  ASM::enable\n  # Disable ASM for HTTP paths ending in .jpg\n  if { [HTTP::path] ends_with \".jpg\" } {\n    ASM::disable\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            // F5 documents HTTP_CLASS_SELECTED as the command's valid event.
            // The event itself implies HTTP rather than ASM, so record the
            // exceptional event explicitly while retaining the profile for an
            // informational "assumes profile" hint in other legal contexts.
            also_in: &["HTTP_CLASS_SELECTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ASM::disable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        xc_translatable: Some(true),
        ..CommandSpec::DEFAULT
    }
}
