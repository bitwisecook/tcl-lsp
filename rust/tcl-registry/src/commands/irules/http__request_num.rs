//! `HTTP::request_num` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::request_num",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
hover: Some(HoverSnippet {
            summary: "Returns the ordinal number of the current HTTP request on the connection.",
            synopsis: &["HTTP::request_num"],
            snippet: "Returns the ordinal number of the current HTTP request on this\nTCP connection.  The first request is 1.  Useful for detecting\nkeep-alive request boundaries.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__request_num.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
