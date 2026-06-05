//! `HTTP::query` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::query",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::UNNORMALISED_HTTP_GETTER,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        options: &[OptionSpec {
            name: "-normalized",
            takes_value: false,
            value_hint: "",
            detail: "Return the canonicalised query (URL evasion patterns rejected).",
            dialects: None,
        }],
hover: Some(HoverSnippet {
            summary: "Returns or sets the query part of the HTTP request.",
            synopsis: &["HTTP::query (QUERY_STRING)?"],
            snippet: "Returns or sets the query part of the HTTP request. The query is defined as the\npart of the request past a ? character, if any.\nFor the following URL:\nhttp://www.example.com:8080/main/index.jsp?user=test&login=check\nThe query is:\nuser=test&login=check",
            source: "https://clouddocs.f5.com/api/irules/HTTP__query.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"http_path [HTTP::path]\"\n  log local0. \"http_query [HTTP::query]\"\n  HTTP::query user=test_user&login=test_login\n}",
            return_value: "Returns the query part of the HTTP request.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Getter, synopsis: "HTTP::query ?-normalized?" },
            FormSpec { kind: FormKind::Setter, synopsis: "HTTP::query <QUERY_STRING>" },
        ],
        ..CommandSpec::DEFAULT
    }
}
