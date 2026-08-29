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

//! `ICAP::header` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::header",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns ICAP attributes in the ICAP header.",
            synopsis: &[
                "ICAP::header 'names'",
                "ICAP::header 'at' HEADER_INDEX",
                "ICAP::header 'count' (HEADER_NAME)?",
                "ICAP::header 'exists' HEADER_NAME",
            ],
            snippet: "The ICAP::header command sets or returns attributes in the ICAP header.",
            source: "https://clouddocs.f5.com/api/irules/ICAP__header.html",
            examples: "when ICAP_RESPONSE {\n                ICAP::header remove X-ICAP-my-custom-header\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ICAP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ICAP::header 'names'",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
