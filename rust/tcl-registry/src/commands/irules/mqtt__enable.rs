//! `MQTT::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable MQTT parsing on a connection.",
            synopsis: &["MQTT::enable"],
            snippet: "This command enables MQTT parsing on a connection.",
            source: "https://clouddocs.f5.com/api/irules/MQTT__enable.html",
            examples: "when SERVER_CONNECTED {\n   MQTT::enable\n}",
            return_value: "None.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
