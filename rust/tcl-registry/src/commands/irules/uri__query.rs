//! `URI::query` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::query",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the query string portion of the given URI or the value of a query string parameter.",
            synopsis: &["URI::query URI_STRING (PARAMETER_NAME)?"],
            snippet: "Returns the query string portion of the given URI or the value of a\nquery string parameter.",
            source: "https://clouddocs.f5.com/api/irules/URI__query.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"Query string of URI [HTTP::uri] is [URI::query [HTTP::uri]]\"\n}",
            return_value: "Returns the query string portion of the given URI or the value of a query string parameter.",
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
            FormSpec { kind: FormKind::Default, synopsis: "URI::query URI_STRING (PARAMETER_NAME)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
