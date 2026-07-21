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

//! `BWC::measure` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::measure",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to measure rate for a particular traffic flow or flows belonging to the bwc instance.",
            synopsis: &["BWC::measure ( ('start' | 'stop') |"],
            snippet: "After a flow has been assigned a policy, user can start or stop measurement on a per policy basis or on a per flow basis. Once the measurement is started the measured bandwidth can be read by the user using 'BWC::measure get ..' iRules. Optionally users can direct the bandwidth measurement results to a 'log publisher' configured on the BIGIP system. Based on the log_publisher setting the measurement results will be logged to the log server indicated in the 'log_publisher'. It is usually an external high speed log server.",
            source: "https://clouddocs.f5.com/api/irules/BWC__measure.html",
            examples: "when SERVER_CONNECTED {\n        TCP::collect     set count 0\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BWC::measure ( ('start' | 'stop') |",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
