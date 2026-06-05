//! `HTTP2::requests` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::requests",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to determine the count of requests received in the current HTTP/2 session.",
            synopsis: &["HTTP2::requests"],
            snippet: "Returns the count of requests received in the current HTTP/2 session. This includes the current request. Returns 0 if HTTP/2 is not active.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__requests.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"Number of requests received in current session is [HTTP2::requests]\"\n}",
            return_value: "The return is a number indicating count of requests received in current HTTP/2 session. The return is 0 if HTTP/2 is not active.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP2::requests" },
        ],
        ..CommandSpec::DEFAULT
    }
}
