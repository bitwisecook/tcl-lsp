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

//! `clientside` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "clientside",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        // `clientside (NESTING_SCRIPT)?` — the bare query form (0 args,
        // returns 1/0) or a single optional nesting-script body (#501).
        arity: Arity::new(0, 1),
        // The optional nesting script (index 0) is a body evaluated in
        // the client-side context; it runs synchronously in the
        // caller's frame, so the default `BodyKind::Plain` applies.  The
        // 0-arg query form has no arg at index 0, so the role simply
        // does not apply there.
        arg_roles: &[(0, ArgRole::Body)],
        hover: Some(HoverSnippet {
            summary: "Causes the specified iRule commands to be evaluated under the client-side context.",
            synopsis: &["clientside (NESTING_SCRIPT)?"],
            snippet: "Causes the specified iRule commands to be evaluated under the client-side context. This command has no effect if the iRule is already being evaluated under the client-side context. If there is no argument, the command returns 1 if the current event is in the clientside context or 0 if not.",
            source: "https://clouddocs.f5.com/api/irules/clientside.html",
            examples: "when SERVER_CONNECTED {\n   # Check if the client IP address is 10.1.1.80\n   # [clientside {IP::remote_addr}] is equivalent to [IP::client_addr]\n   if { [IP::addr [clientside {IP::remote_addr}] equals 10.1.1.80] } {\n      # Do something like drop the packets in this example\n      discard\n   }\n}",
            return_value: "clientside Returns 1 if the current event is in the clientside context or 0 if not.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "clientside (NESTING_SCRIPT)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
