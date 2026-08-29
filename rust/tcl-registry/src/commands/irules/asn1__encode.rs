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

//! `ASN1::encode` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::encode",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Encodes ASN.1 records.",
            synopsis: &[
                "ASN1::encode ('BER' | 'DER') FORMAT (VALUE)*",
                "ASN1::encode ('insert' | 'replace') ELEMENT OFFSET FORMAT (VALUE)*",
            ],
            snippet: "This command is used to encode ASN.1 records. Data is formatted according to formatString.\n\nformatString can have the following characters:\n\n    a - Octet String\n    B - Bit String\n    b - Boolean\n    e - Enum\n    i - Integer\n    t - Tag of next element\n    ? - Don't output the component if the corresponding value is empty\n    ?hex-tag - Denotes that the specifier which follows is for an optional component. This is used for encoding or decoding an ASN.1 Set or Sequence which contains nested OPTIONAL or DEFAULT components. hex-tag, is a two-character hex byte of the expected tag.",
            source: "https://clouddocs.f5.com/api/irules/ASN1__encode.html",
            examples: "# LDAP String Modify\nappend base_mod $base \",dc=supercalafragalisticexpialadoshus\"\nASN1::encode replace $ele 1 \"a\" $base_mod\n\n# LDAP Encode/Rewrite - The size field is 4 elements forward from $ele\nASN1::encode replace $ele 4 \"i\" [incr size 2]\n\n# LDAP Encode/Rewrite - The time field is 5 elements forward from $ele\nASN1::encode replace $ele 5 \"i\" [expr $time + 100]\n\n# Encode an LDAP SearchRequest Extensible Match filter where RuleId and Type are optional,",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ASN1::encode ('BER' | 'DER') FORMAT (VALUE)*",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
