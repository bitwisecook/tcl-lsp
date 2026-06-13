//! `UDP::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends data directly to a peer.",
            synopsis: &["UDP::respond RESPONSE_STRING"],
            snippet: "Sends the specified data directly to the peer. This command can be used\nto complete a protocol handshake inside an iRule.",
            source: "https://clouddocs.f5.com/api/irules/UDP__respond.html",
            examples: "when CLIENT_ACCEPTED {\n  set packet [binary format S {0x0000}]\n  UDP::respond $packet\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("udp"),
            profiles: &[],
            also_in: &[
                "SIP_REQUEST",
                "SIP_REQUEST_SEND",
                "SIP_RESPONSE",
                "STREAM_MATCHED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "UDP::respond RESPONSE_STRING" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::UdpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
