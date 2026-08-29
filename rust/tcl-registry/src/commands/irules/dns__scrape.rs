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

//! `DNS::scrape` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::scrape",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows users to walk over a DNS message and parse out information from the packet based on user supplied arguments.",
            synopsis: &[
                "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
            ],
            snippet: "This iRules command allows users to walk over a DNS message and parse\nout information from the packet based on user supplied arguments.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__scrape.html",
            examples: "when DNS_RESPONSE {\n   foreach rr [DNS::scrape ANSWER type ttl qnamelen rdatalen] {\n     log local2. \"ANSWER: $rr\"\n   }\n   foreach rr [DNS::scrape AUTHORITY type ttl class qnamelen rdatalen] {\n     log local2. \"AUTHORITY: $rr\"\n   }\n   foreach rr [DNS::scrape ADDITIONAL type ttl class qnamelen rdatalen] {\n     log local2. \"ADDITIONAL: $rr\"\n   }\n }",
            return_value: "",
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
            synopsis: "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
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
