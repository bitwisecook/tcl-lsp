//! `UDP::local_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::local_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the local UDP port/service number.",
            synopsis: &["UDP::local_port (clientside | serverside)?"],
            snippet: "Returns the local UDP port/service number.",
            source: "https://clouddocs.f5.com/api/irules/UDP__local_port.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [matchclass [UDP::local_port] equals $::ValidUDPPorts ] } {\n    pool udp_pool\n  } else {\n     discard\n  }\n}",
            return_value: "Returns the local UDP port/service number",
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
            FormSpec { kind: FormKind::Default, synopsis: "UDP::local_port (clientside | serverside)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
