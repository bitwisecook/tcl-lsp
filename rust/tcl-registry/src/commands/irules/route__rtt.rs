//! `ROUTE::rtt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::rtt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the cached round-trip-time estimate.",
            synopsis: &["ROUTE::rtt DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the cached round-trip-time for the destination and/or\ngateway if the relevant TCP profile enables cmetrics-cache.\n\nThe return value only applies to the TMM executing the command. It\ndoes not consider cache entries on other TMMs.\n\nROUTE::rtt returns a value of 0 when there are no statistics available.\n\nNOTE: The returned value is scaled to units of 100ns; to express it\nin the same units as TCP::rtt multiply it by 32/10000.\n\nNOTE: When used with the fastL4 profile, RTT from client/server\nneeds to be enabled and the client and server need to be using TCP\ntimestamps.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__rtt.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"Cached rtt is: [ROUTE::rtt [IP::remote_addr]]\"\n}",
            return_value: "RTT in units of 100ns.",
        }),
        ..CommandSpec::DEFAULT
    }
}
