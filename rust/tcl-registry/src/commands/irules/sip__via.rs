//! `SIP::via` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "proto",
        arity: Arity::new(0, 1),
        detail: "Get Via protocol.",
        synopsis: "SIP::via proto ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sent_by",
        arity: Arity::new(0, 1),
        detail: "Get Via sent-by field.",
        synopsis: "SIP::via sent_by ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "received",
        arity: Arity::new(0, 1),
        detail: "Get Via received parameter.",
        synopsis: "SIP::via received ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "branch",
        arity: Arity::new(0, 1),
        detail: "Get Via branch parameter.",
        synopsis: "SIP::via branch ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "maddr",
        arity: Arity::new(0, 1),
        detail: "Get Via maddr parameter.",
        synopsis: "SIP::via maddr ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ttl",
        arity: Arity::new(0, 1),
        detail: "Get Via ttl parameter.",
        synopsis: "SIP::via ttl ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::via",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets SIP via header information.",
            synopsis: &["SIP::via ?field? ?INDEX?", "SIP::via (proto | sent_by | received | branch | maddr | ttl) ?INDEX?"],
            snippet: "This set of commands allows you to get information in the SIP via header.",
            source: "https://clouddocs.f5.com/api/irules/SIP__via.html",
            examples: "when SIP_RESPONSE {\n  log local0. [SIP::via 0]\n  SIP::header remove Via 0\n  SIP::response rewrite 123 \"no xxx\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SIP::via ?field? ?INDEX?" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
