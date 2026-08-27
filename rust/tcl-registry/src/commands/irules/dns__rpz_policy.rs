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

//! `DNS::rpz_policy` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::rpz_policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the RPZ policy associated with the DNS cache.",
            synopsis: &["DNS::rpz_policy"],
            snippet: "Returns the RPZ (Response Policy Zones) policy associated with the DNS cache.\n\nThe possible return values are:\n    * \"\" (empty string) if RPZ is not configured.\n    * \"NXDOMAIN\" if RPZ is configured to return an NXDOMAIN response on a match.\n    * \"WG <walled garden name>\" if RPZ is configured to return a Walled Garden redirect on a match.",
            source: "https://clouddocs.f5.com/api/irules/DNS__rpz_policy.html",
            examples: "when DNS_RESPONSE {\n     if { [DNS::origin] eq \"RPZ\"} {\n        log local0. \"[DNS::question name] resulted in an RPZ [DNS::rpz_policy]\"\n     }\n}",
            return_value: "* \"\" (empty string) if RPZ is not configured. * \"NXDOMAIN\" if RPZ is configured to return an NXDOMAIN response on a match. * \"WG <walled garden name>\" if RPZ is configured to return a Walled Garden redirect on a match.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DNS::rpz_policy",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
