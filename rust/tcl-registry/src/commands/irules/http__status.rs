//! `HTTP::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::status",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the response status code.",
            synopsis: &["HTTP::status"],
            snippet: "Returns the response status code as defined in RFC2616",
            source: "https://clouddocs.f5.com/api/irules/HTTP__status.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::status] == 404 } {\n    HTTP::redirect \"http://www.example.com/not_found.html\"\n }\n}",
            return_value: "Returns the response status code.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_INGRESS"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::status" },
        ],
        ..CommandSpec::DEFAULT
    }
}
