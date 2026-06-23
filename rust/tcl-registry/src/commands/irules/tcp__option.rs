//! `TCP::option` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "get",
        arity: Arity::exact(1),
        detail: "Get TCP option value.",
        synopsis: "TCP::option get <kind>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::new(2, 3),
        detail: "Set TCP option value.",
        synopsis: "TCP::option set <kind> <value> ?all?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "noset",
        arity: Arity::exact(1),
        detail: "Prevent option from being set.",
        synopsis: "TCP::option noset <kind>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet {
            summary: "Retrieves or changes TCP header options.",
            synopsis: &[
                "TCP::option get <kind>",
                "TCP::option set <kind> <value> ?all?",
                "TCP::option noset <kind>",
            ],
            snippet: "Gets or sets the value of the specified option of the TCP header.\nThe TCP::option get command is only functional when BIG-IP has been configured to collect options before the iRule is called. In v10, this is done with a db variable and is effective only on the clientside. When called in the serverside context it returns an error indicating that the specified option was not configured for collection.\n\nIn v11, this is configured through the TCP profile and the command can be used in either the serverside context or the clientside, depending on the profile configuration.",
            source: "https://clouddocs.f5.com/api/irules/TCP__option.html",
            examples: "#Insert the client REAL IP (mostly used when the client IP is SNATted).\nwhen SERVER_CONNECTED {\n    scan [IP::client_addr] {%d.%d.%d.%d} a b c d\n    TCP::option set 29 [binary format cccc $a $b $c $d] all\n}",
            return_value: "With the \"get\" keyword, returns the specified TCP option kind value. If the requested option kind was not configured for collection, an error indicating so is returned instead. If the option kind was specified but has not yet been seen on the current connection, the command returns null.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::option <get|set|noset> <kind> ?value? ?all?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
