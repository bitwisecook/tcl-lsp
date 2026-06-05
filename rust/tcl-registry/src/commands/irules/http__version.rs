//! `HTTP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::version",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets the HTTP version of the request or response.",
            synopsis: &["HTTP::version ('0.9' | '1.0' | '1.1')?", "HTTP::version '-string' (ANY_CHARS)?"],
            snippet: "Returns or sets the HTTP version of the request or response. This\ncommand replaces the BIG-IP 4.X variable http_version.\nIf needed, Connection and Host headers will automatically be added\nappropriately.\nHTTP::version will return the original version of the request or\nresponse, even if it has been changed.  Note that this will return\nthe \"effective\" version used, which may be different than the actual\nversion string in the request or response.  For example, invalid\nversion numbers may be parsed as 1.1 in order to increase\ninter-operability with common HTTP servers.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__version.html",
            examples: "when HTTP_RESPONSE {\n  HTTP::version \"1.1\"\n}",
            return_value: "Returns the HTTP version of the request or response",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::version ('0.9' | '1.0' | '1.1')?\nHTTP::version -string ?value?" },
        ],
        options: &[
            OptionSpec { name: "-string", takes_value: true, value_hint: "version", detail: "Get/set version as raw string.", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
