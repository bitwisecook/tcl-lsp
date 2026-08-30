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

//! `SSL::c3d` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "extension",
        arity: Arity::exact(2),
        detail: "Insert a certificate extension.",
        synopsis: "SSL::c3d extension <oid> <value>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cert",
        arity: Arity::exact(1),
        detail: "Set the C3D client certificate.",
        synopsis: "SSL::c3d cert <certificate>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "subject",
        arity: Arity::exact(2),
        detail: "Modify forged certificate subject CN.",
        synopsis: "SSL::c3d subject <field> <value>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cert_lifespan",
        arity: Arity::exact(1),
        detail: "Set the forged C3D certificate lifespan in hours.",
        synopsis: "SSL::c3d cert_lifespan <hours>",
        hover: Some(HoverSnippet {
            summary: "Sets the lifespan of the forged C3D certificate in hours.",
            synopsis: &["SSL::c3d cert_lifespan <hours>"],
            snippet: "Sets the lifespan of the forged C3D certificate in hours. Introduced in BIG-IP 21.1.0.",
            source: "https://clouddocs.f5.com/api/irules/SSL__c3d.html",
            examples: "SSL::c3d cert_lifespan 5",
            return_value: "",
        }),
        lifecycle: Lifecycle::introduced_in("21.1.0"),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cert_start_date",
        arity: Arity::new(0, 1),
        detail: "Choose the forged C3D certificate start date.",
        synopsis: "SSL::c3d cert_start_date ?override|original?",
        hover: Some(HoverSnippet {
            summary: "Controls the forged C3D certificate's notBefore value.",
            synopsis: &["SSL::c3d cert_start_date ?override|original?"],
            snippet: "Specifies whether to override the generated C3D certificate's notBefore value with the forging time or retain the original certificate's notBefore value. Introduced in BIG-IP 21.1.0.",
            source: "https://clouddocs.f5.com/api/irules/SSL__c3d.html",
            examples: "SSL::c3d cert_start_date override",
            return_value: "",
        }),
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "override",
                    detail: "Use the certificate forging time.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "original",
                    detail: "Retain the original certificate's notBefore value.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        lifecycle: Lifecycle::introduced_in("21.1.0"),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::c3d",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN.",
            synopsis: &[
                "SSL::c3d extension (ARG ARG)",
                "SSL::c3d cert CERTIFICATE",
                "SSL::c3d subject (ARG ARG)",
                "SSL::c3d cert_lifespan HOURS",
                "SSL::c3d cert_start_date (override | original)?",
            ],
            snippet: "Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN. When subject CN is modified CN, O, OU will be converted to PrintableString where possible or UTF-8. Expected input for subject CN is in UTF-8 format.",
            source: "https://clouddocs.f5.com/api/irules/SSL__c3d.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0.info \"CLIENTSSL_HANDSHAKE\"\n    SSL::c3d extension CP \"2.16.840.1.101.2.1.11.9, cpsuri:https://localhost/test-statement/pki/cps.txt, cpsuri:https://localhost/test-statement1/pki/cps.txt;2.16.840.1.101.2.1.11.19\"\n    SSL::c3d extension SAN \"DNS:*.test-client.com, IP:1.1.1.1\"\n    SSL::c3d extension 1.2.3.4 \"The oid-vaule for oid 1.2.3.4\"\n    SSL::c3d cert_lifespan 5\n    SSL::c3d cert_start_date override\n    SSL::c3d extension KU \"critical,digitalSignature,keyEncipherment\"\n    SSL::c3d extension EKU \"clientAuth,codeSigning\"\n    SSL::c3d extension SDA \"dateOfBirth:20250115120000Z,gender:M,countryOfCitizenship:US\"\n    if {[SSL::cert count] > 0} {\n        SSL::c3d subject commonName [X509::subject [SSL::cert 0] commonName]\n    }\n}",
            return_value: "SSL::c3d extension <oid oid-value> inserts an extension. SSL::c3d cert <certificate in DER> sets the C3D client certificate. SSL::c3d subject <nid value> sets the forged certificate subject. SSL::c3d cert_lifespan <hours> sets its lifespan. SSL::c3d cert_start_date [override | original] controls its notBefore value.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::c3d <subcommand> <args>",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
