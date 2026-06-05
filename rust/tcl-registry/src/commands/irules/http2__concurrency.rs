//! `HTTP2::concurrency` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::concurrency",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to determine the number of active concurrent streams in the current HTTP/2 session.",
            synopsis: &["HTTP2::concurrency"],
            snippet: "Returns number of active concurrent streams in the current HTTP/2 session.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__concurrency.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"Number of active concurrent streams is [HTTP2::concurrency]\"\n}",
            return_value: "The return is a number indicating the active concurrent streams in current HTTP/2 session.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP2::concurrency" },
        ],
        ..CommandSpec::DEFAULT
    }
}
