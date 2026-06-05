//! `HTTP::has_responded` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::has_responded",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
hover: Some(HoverSnippet {
            summary: "Returns true if this HTTP transaction has been prematurely completed by an iRule command or other filter logic.",
            synopsis: &["HTTP::has_responded"],
            snippet: "This can be triggered by HTTP::respond, HTTP::redirect, HTTP::retry, and some ACCESS commands.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__has_responded.html",
            examples: "when HTTP_REQUEST {\n  # Used for cases where only one response to the client is permitted.\n  # Another HTTP::respond might have been called in other iRULE script.\n  if {[HTTP::has_responded]} {\n    log local0. \"Have already responded.\"\n  } else {\n    HTTP::respond 200 content {<html><body>First and Only Response</body></html>}\n  }\n}",
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
