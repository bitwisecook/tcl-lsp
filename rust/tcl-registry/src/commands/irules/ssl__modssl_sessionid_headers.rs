//! `SSL::modssl_sessionid_headers` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::modssl_sessionid_headers",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a list of fields for HTTP headers.",
            synopsis: &["SSL::modssl_sessionid_headers (initial | current)?"],
            snippet: "Returns a list of fields that the system will add to the HTTP headers, in order to emulate modssl behavior. The return type is a Tcl list; this list will be interpreted as a header-name/header-value pair by HTTP::header, for example.",
            source: "https://clouddocs.f5.com/api/irules/SSL__modssl_sessionid_headers.html",
            examples: "when HTTP_REQUEST {\n    HTTP::header insert [SSL::modssl_sessionid_headers]\n}",
            return_value: "SSL::modssl_sessionid_headers Returns a header name of \"SSLClientSessionId\", and a header value of the session id requested by the client.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::modssl_sessionid_headers (initial | current)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
