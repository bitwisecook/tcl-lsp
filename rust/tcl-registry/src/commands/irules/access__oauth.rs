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

//! `ACCESS::oauth` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::oauth",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "OAuth related ACCESS iRule",
            synopsis: &["ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)"],
            snippet: "OAuth related ACCESS iRule\n\nACCESS::oauth sign [ -header <raw-data> ] -payload <raw-data> -key <JWK object>\n                   [ -alg <signing algorithm> ] [ -ignore-cert-expiry ]\n\n     * Returns a JSON Web Signature token based on provided payload and signed\n       with provided JWK object. When the specified JWK object does not specify\n       a JWS signing algorithm, an additional signing algorithm is required\n       and must be provided with the -alg option.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__oauth.html",
            examples: "when ACCESS_SESSION_CLOSED {\n    call delete_jws_cache\n}",
            return_value: "JSON Web Signature string.",
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-header",
                    value: OptionValue::value("RAW_DATA"),
                    detail: "Raw data for JOSE header section.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-payload",
                    value: OptionValue::value("RAW_DATA"),
                    detail: "Raw data for JWS payload.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-key",
                    value: OptionValue::value("JWK_OBJECT"),
                    detail: "JWK object for signing.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-alg",
                    value: OptionValue::value("ALG"),
                    detail: "Signing algorithm.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-ignore-cert-expiry",
                    value: OptionValue::flag(),
                    detail: "Allow expired certificate.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
