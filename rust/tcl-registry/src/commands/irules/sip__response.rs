//! `SIP::response` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "code",
        arity: Arity::exact(0),
        detail: "Get response code.",
        synopsis: "SIP::response code",
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
        name: "phrase",
        arity: Arity::exact(0),
        detail: "Get response phrase.",
        synopsis: "SIP::response phrase",
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
        name: "rewrite",
        arity: Arity::new(1, 2),
        detail: "Rewrite response code and phrase.",
        synopsis: "SIP::response rewrite <code> ?phrase?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or rewrites the SIP response.",
            synopsis: &["SIP::response (code | phrase)", "SIP::response rewrite CODE (PHRASE)?"],
            snippet: "These commands allow you to get or rewrite the SIP response code or\nphrase.",
            source: "https://clouddocs.f5.com/api/irules/SIP__response.html",
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
            FormSpec { kind: FormKind::Default, synopsis: "SIP::response <subcommand> ?args?" },
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
        ..CommandSpec::DEFAULT
    }
}
