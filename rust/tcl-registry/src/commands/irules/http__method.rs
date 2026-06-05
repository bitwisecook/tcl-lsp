//! `HTTP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::method",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the type of HTTP request method.",
            synopsis: &["HTTP::method"],
            snippet: "Returns the type of HTTP request method. This command replaces the\nBIG-IP 4.X variable http_method.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__method.html",
            examples: "when HTTP_REQUEST {\nlog local0. \"HTTP Method: [HTTP::method]\"\n}",
            return_value: "",
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
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::method" },
        ],
        ..CommandSpec::DEFAULT
    }
}
