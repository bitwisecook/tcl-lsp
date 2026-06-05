//! `SIP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the accumulated SIP data content.",
            synopsis: &[
                "SIP::payload (LENGTH | (OFFSET LENGTH))?",
                "SIP::payload length",
                "SIP::payload replace OFFSET LENGTH PAYLOAD",
                "SIP::payload insert OFFSET PAYLOAD",
            ],
            snippet: "Returns the accumulated SIP data content.",
            source: "https://clouddocs.f5.com/api/irules/SIP__payload.html",
            examples:
                "when SIP_REQUEST {\n    log local0. \"unmodified request [SIP::payload]\"\n}",
            return_value: "Returns the SIP data accumulated so far",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["MR_EGRESS", "MR_FAILED", "MR_INGRESS"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIP::payload (LENGTH | (OFFSET LENGTH))?",
        }],
        ..CommandSpec::DEFAULT
    }
}
