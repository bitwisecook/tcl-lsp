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

//! `BOTDEFENSE::bot_signature` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_signature",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the name of the detected Bot Signature.",
            synopsis: &["BOTDEFENSE::bot_signature"],
            snippet: "Returns the name of the detected Bot Signature, or an empty string if no bot signature was detected.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_signature.html",
            examples: "# EXAMPLE: Log the bot signature.\nwhen BOTDEFENSE_REQUEST {\n    set log \"botdefense bot_signature is\"\n    append log \" [BOTDEFENSE::bot_signature]\"\n    HSL::send $hsl $log\n}",
            return_value: "Returns the name of the detected Bot Signature, or an empty string if no bot signature was detected.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::bot_signature",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
