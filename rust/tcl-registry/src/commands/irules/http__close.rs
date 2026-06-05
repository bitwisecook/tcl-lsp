//! `HTTP::close` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::close",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Closes the HTTP connection.",
            synopsis: &["HTTP::close"],
            snippet: "Closes the HTTP connection.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__close.html",
            examples: "when HTTP_REQUEST {\n  set method [HTTP::method]\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::close",
        }],
        ..CommandSpec::DEFAULT
    }
}
