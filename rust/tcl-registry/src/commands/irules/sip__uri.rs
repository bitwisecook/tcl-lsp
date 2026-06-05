//! `SIP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the URI of the request.",
            synopsis: &["SIP::uri (URI_STRING)?"],
            snippet: "Returns or sets the complete URI of the request.",
            source: "https://clouddocs.f5.com/api/irules/SIP__uri.html",
            examples:
                "when SIP_REQUEST {\n  log local0. \"uri: [SIP::uri] via [SIP::header Via 0]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIP::uri (URI_STRING)?",
        }],
        ..CommandSpec::DEFAULT
    }
}
