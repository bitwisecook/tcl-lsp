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

//! `RESOLVER::summarize` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLVER::summarize",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a summary of the response.",
            synopsis: &["RESOLVER::summarize DNS_MESSAGE"],
            snippet: "Takes a dns_message structure and returns a summary as a list of resource records.",
            source: "https://clouddocs.f5.com/api/irules/RESOLVER-summarize.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set rrs [RESOLVER::summarize $result]\n}",
            return_value: "The summary will be a TCL list of resource record objects of the type specified in the query. Individual resource record objects are usable by the DNSMSG::record iRule command.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RESOLVER::summarize DNS_MESSAGE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
