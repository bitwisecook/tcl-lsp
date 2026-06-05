//! `ROUTE::clear` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::clear",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Removes a Congestion Metrics Cache entry.",
            synopsis: &["ROUTE::clear DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Removes the congestion metrics and MTU associated with a\ndestination IP address and/or gateway.\n\nClears the entry on all platform TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__clear.html",
            examples: "when CLIENT_ACCEPTED {\n    set bandwidth [ROUTE::bandwidth [IP::remote_addr]]\n    if { $bandwidth > 0 && $bandwidth < 1000 } {\n        # Reject cache entries below 1000 kbps\n        ROUTE::clear [IP::remote_addr]\n    }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
