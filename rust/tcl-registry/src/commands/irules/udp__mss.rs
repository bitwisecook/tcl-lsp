//! `UDP::mss` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::mss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the on-wire Maximum Segment Size (MSS) for a UDP connection.",
            synopsis: &["UDP::mss"],
            snippet: "Returns the on-wire Maximum Segment Size (MSS) for a UDP connection.",
            source: "https://clouddocs.f5.com/api/irules/UDP__mss.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [UDP::mss] < 1000 } {\n    pool small_req_pool\n  } else {\n    pool large_req_pool\n  }\n}",
            return_value: "Returns the on-wire Maximum Segment Size (MSS) for a UDP connection",
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
        ..CommandSpec::DEFAULT
    }
}
