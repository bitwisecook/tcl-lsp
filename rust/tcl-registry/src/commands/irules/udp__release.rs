//! `UDP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allow client-side ingress to flow following a call to UDP::hold.",
            synopsis: &["UDP::release"],
            snippet:
                "Called at some point after UDP::hold was called.  Unblock ingress on client side.",
            source: "https://clouddocs.f5.com/api/irules/UDP__release.html",
            examples: "when LB_SELECTED {\n    UDP::release\n}",
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
        ..CommandSpec::DEFAULT
    }
}
