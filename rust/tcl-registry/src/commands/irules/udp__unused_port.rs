//! `UDP::unused_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::unused_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an unused UDP port for the specified IP tuple.",
            synopsis: &["UDP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR"],
            snippet: "Returns an unused UDP port for the specified IP tuple.",
            source: "https://clouddocs.f5.com/api/irules/UDP__unused_port.html",
            examples: "when CLIENT_ACCEPTED {\n  set port [UDP::unused_port [IP::remote_addr] [UDP::remote_port] [IP::local_addr]]\n  UDP::respond \"Next unused port: $port\"\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "UDP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR" },
        ],
        ..CommandSpec::DEFAULT
    }
}
