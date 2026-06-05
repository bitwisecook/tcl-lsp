//! `TCP::proxybufferhigh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybufferhigh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets proxy buffer high threshold.",
            synopsis: &["TCP::proxybufferhigh"],
            snippet:
                "Gets the threshold at which the proxy buffer stops accepting new data, in bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__proxybufferhigh.html",
            examples: "when SERVER_CONNECTED {\n    log local0.debug \"[TCP::proxybufferhigh]\"\n}",
            return_value: "The proxy buffer high threshold.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: true,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
