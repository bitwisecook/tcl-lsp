//! `DIAMETER::route_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::route_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the routing status of the current message.",
            synopsis: &["DIAMETER::route_status"],
            snippet: "The DIAMETER::route_status command returns the routing status of the current\nmessage. Valid status are:\n  * \"unprocessed\"\n  * \"route found\"\n  * \"no route found\"\n  * \"dropped\"\n  * \"queue full\"\n  * \"no connection\"\n  * \"connection closing\"\n  * \"internal error\"\n\n\"route found\" is based on the DIAMETER RouteTable finding a route. It\nis not affected by the proxy’s ability to create a connection, so even\nif the server is not listening on the specified address or marked\ndown, it still returns status as \"route found\" if the RouteTable is\nable to find the route.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__route_status.html",
            examples: "",
            return_value: "Returns routing status of the current message",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
