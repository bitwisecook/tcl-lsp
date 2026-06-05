//! `ROUTE::mtu` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::mtu",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the cached MTU entry.",
            synopsis: &["ROUTE::mtu DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the cached MTU entry for the provided destination and/or gateway.\n\nUnlike other ROUTE::commands, this value is valid across all TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__mtu.html",
            examples: "when CLIENT_ACCEPTED {\n    set mtu [ROUTE::mtu [IP::remote_addr]]\n    if { $mtu > 0 && $mtu < 300 } {\n        #Ignore extremely small cached MTUs\n        ROUTE::clear [IP::remote_addr]\n    }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
