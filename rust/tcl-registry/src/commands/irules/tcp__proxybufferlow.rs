//! `TCP::proxybufferlow` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybufferlow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets proxy buffer low threshold.",
            synopsis: &["TCP::proxybufferlow"],
            snippet:
                "Gets the threshold at which the proxy buffer starts sending new data, in bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__proxybufferlow.html",
            examples: "when SERVER_CONNECTED {\n    log local0.debug \"[TCP::proxybufferlow]\"\n}",
            return_value: "The proxy buffer low threshold.",
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
