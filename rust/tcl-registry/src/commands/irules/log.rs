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

//! `log` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "log",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        // Remote logging has remote address, facility, and message as its
        // three positional arguments. `-noname` is an OptionSpec and is
        // consumed before this positional arity is checked.
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Write a message to BIG-IP logging facilities.",
            synopsis: &[
                "log ?-noname? ?facility.level? message",
                "log ?-noname? remote_ip?:port? facility.level message",
            ],
            snippet: "Common form: `log local0. \"message\"`.",
            source: "https://clouddocs.f5.com/api/irules/log.html",
            examples: "",
            return_value: "",
        }),
        // Tainted data in a log message → log injection /
        // forging (IRULE3003).
        taint_log_sink: Some("IRULE3003"),
        forms: &[FormSpec {
            synopsis: "log ?-noname? ?facility.level? message",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-noname",
                value: OptionValue::flag(),
                detail: "Suppress iRule name prefix in log message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
