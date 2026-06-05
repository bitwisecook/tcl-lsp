//! `WEBSSO::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WEBSSO::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Forwards a request without doing SSO processing on it.",
            synopsis: &["WEBSSO::disable"],
            snippet: "This command causes APM to forward a request without doing SSO\nprocessing on it. If APM receives HTTP 401 response from server, 401\nresponse is forwarded to the end user. The scope of this iRule command\nis per HTTP request. Admin needs to execute it for each HTTP request.",
            source: "https://clouddocs.f5.com/api/irules/WEBSSO__disable.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
