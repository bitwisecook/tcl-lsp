//! `MR::return` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::return",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current message to the originating connection.",
            synopsis: &["MR::return", "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )"],
            snippet: "The MR::return command instructs the Message Routing Framework to return the current message to the originating connection. The message's route status will be updated to 'returned by irule' or the provided route status. When the connection is received on the originating connection, MR_FAILED event will be raised.\n        \nReturns the current message to the originating connection with a route status of 'returned by irule'\n            \nReturns the current message to the originating connection and sets the route status to the route status specified.",
            source: "https://clouddocs.f5.com/api/irules/MR__return.html",
            examples: "when MR_INGRESS {\n    if {[DIAMETER::is_response]} {\n        incr pend_req -1\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::return" },
        ],
        ..CommandSpec::DEFAULT
    }
}
