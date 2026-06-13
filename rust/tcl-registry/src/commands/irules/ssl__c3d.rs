//! `SSL::c3d` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "extension",
        arity: Arity::exact(2),
        detail: "Insert a certificate extension.",
        synopsis: "SSL::c3d extension <oid> <value>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
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
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
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
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::c3d",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN.",
            synopsis: &["SSL::c3d extension (ARG ARG)", "SSL::c3d cert CERTIFICATE", "SSL::c3d subject (ARG ARG)"],
            snippet: "Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN. When subject CN is modified CN, O, OU will be converted to PrintableString where possible or UTF-8. Expected input for subject CN is in UTF-8 format.",
            source: "https://clouddocs.f5.com/api/irules/SSL__c3d.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0.info \"CLIENTSSL_HANDSHAKE\"\n    SSL::c3d extension CP \"2.16.840.1.101.2.1.11.9, cpsuri:https://localhost/test-statement/pki/cps.txt, cpsuri:https://localhost/test-statement1/pki/cps.txt;2.16.840.1.101.2.1.11.19\"\n    SSL::c3d extension SAN \"DNS:*.test-client.com, IP:1.1.1.1\"\n    SSL::c3d extension 1.2.3.4 \"The oid-vaule for oid 1.2.3.4\"\n    if {[SSL::cert count] > 0} {\n        SSL::c3d subject commonName [X509::subject [SSL::cert 0] commonName]\n    }\n}",
            return_value: "SSL::c3d extension <oid oid-value> Inserts the <oid oid-value> as an extension to C3D certificate with OID=oid and value=oid-value.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::c3d <subcommand> <args>" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
