//! `UDP::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Drops the current UDP packet without removing the flow from the connection table.",
            synopsis: &["UDP::drop"],
            snippet: "Drops the current UDP packet without removing the flow from the\nconnection table",
            source: "https://clouddocs.f5.com/api/irules/UDP__drop.html",
            examples: "when SERVER_DATA {\n    if { [UDP::payload contains \"badstring\"] }{\n        UDP::drop\n    }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "UDP::drop" },
        ],
        ..CommandSpec::DEFAULT
    }
}
