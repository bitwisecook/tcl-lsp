//! `TCP::payload` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "replace",
        arity: Arity::exact(3),
        detail: "Replace bytes in collected payload.",
        synopsis: "TCP::payload replace <offset> <length> <data>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(0),
        detail: "Returns the amount of accumulated TCP data in bytes.",
        synopsis: "TCP::payload length",
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 4),
        hover: Some(HoverSnippet {
            summary: "Returns or changes the data collected by TCP::collect.",
            synopsis: &[
                "TCP::payload ?<size>?",
                "TCP::payload replace <offset> <length> <data>",
                "TCP::payload length",
            ],
            snippet: "Returns the accumulated TCP data content, or replaces collected payload with the specified data.",
            source: "https://clouddocs.f5.com/api/irules/TCP__payload.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect\n}",
            return_value: "",
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
            kind: FormKind::Getter,
            synopsis: "TCP::payload ?<size>?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
