//! `UDP::hold` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::hold",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Hold client ingress until UDP::release is called.",
            synopsis: &["UDP::hold"],
            snippet: "Hold back processing of input packets until UDP::release is called.",
            source: "https://clouddocs.f5.com/api/irules/UDP__hold.html",
            examples: "when CLIENT_ACCEPTED {\n    UDP::hold\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "UDP::hold",
        }],
        ..CommandSpec::DEFAULT
    }
}
