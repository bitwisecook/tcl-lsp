//! `SIP::route_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::route_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the routing status of the current message.",
            synopsis: &["SIP::route_status"],
            snippet: "The SIP::route_status command returns the routing status of the current\nmessage. Valid status are:\n  * \"unprocessed\"\n  * \"route found\"\n  * \"no route found\"\n  * \"dropped\"\n  * \"queue full\"\n  * \"no connection\"\n  * \"connection closing\"\n  * \"internal error\"\n\n\"route found\" is based on the SIP RouteTable finding a route. It is not\neffected by the proxy’s ability to create a connection, so even if the\nserver is not listening on the specified address or marked down, it\nmight still return status as \"route found\" if the RouteTable is able to\nfind the route.",
            source: "https://clouddocs.f5.com/api/irules/SIP__route_status.html",
            examples: "when SIP_RESPONSE_SEND {\n  log local0. [SIP::route_status]\n}",
            return_value: "Returns routing status of the current message",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SIP::route_status" },
        ],
        ..CommandSpec::DEFAULT
    }
}
