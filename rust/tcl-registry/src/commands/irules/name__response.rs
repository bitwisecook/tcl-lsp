//! `NAME::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NAME::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Returns a list of records received in response to a DNS query.",
            synopsis: &["NAME::response"],
            snippet: "Returns a list of records received in response to a DNS query made with the NAME_ _lookup command.",
            source: "https://clouddocs.f5.com/api/irules/NAME__response.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["NAME"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
