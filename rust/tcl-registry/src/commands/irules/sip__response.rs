//! `SIP::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or rewrites the SIP response.",
            synopsis: &["SIP::response (code | phrase)", "SIP::response rewrite CODE (PHRASE)?"],
            snippet: "These commands allow you to get or rewrite the SIP response code or\nphrase.",
            source: "https://clouddocs.f5.com/api/irules/SIP__response.html",
            examples: "when SIP_RESPONSE {\n  log local0. [SIP::via 0]\n  SIP::header remove Via 0\n  SIP::response rewrite 123 \"no xxx\"\n}",
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
        ..CommandSpec::DEFAULT
    }
}
