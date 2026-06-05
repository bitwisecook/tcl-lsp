//! `SIP::call_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::call_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of the Call-ID header in a SIP request.",
            synopsis: &["SIP::call_id"],
            snippet: "Returns the value of the Call-ID header in a SIP request. Only the\nfirst 256 bytes of the Call-ID will be returned.",
            source: "https://clouddocs.f5.com/api/irules/SIP__call_id.html",
            examples: "when SIP_REQUEST_SEND {\n    log local0. \"Call ID [SIP::call_id]\"\n}",
            return_value: "Returns the value of the Call-ID header in a SIP request",
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
        ..CommandSpec::DEFAULT
    }
}
