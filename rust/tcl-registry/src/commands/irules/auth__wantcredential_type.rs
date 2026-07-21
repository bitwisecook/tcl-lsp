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

//! `AUTH::wantcredential_type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns an authorization session authidXs credential type.",
            synopsis: &["AUTH::wantcredential_type AUTH_ID"],
            snippet: "Returns the authorization session authid’s credential type that the\nsystem last requested (when the system generated an AUTH_WANTCREDENTIAL\nevent). The value of the <authid> argument is either username,\npassword, x509, x509_issuer, or unknown, based upon the system’s\nassessment of the credential prompt string and style.\n\nAUTH::wantcredential_type <authid>\n\n     * Returns the authorization session authid’s credential type that the\n       system last requested (when the system generated an\n       AUTH_WANTCREDENTIAL event).",
            source: "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_type.html",
            examples: "when AUTH_WANTCREDENTIAL {\n  HTTP::respond 401 \"WWW-Authenticate\" \"Basic realm=\\\"\\\"\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AUTH::wantcredential_type AUTH_ID",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
