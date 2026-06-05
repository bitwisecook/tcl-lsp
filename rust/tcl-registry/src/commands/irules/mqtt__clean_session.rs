//! `MQTT::clean_session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::clean_session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set clean_session flag of MQTT CONNECT message.",
            synopsis: &["MQTT::clean_session ('0' | '1')?"],
            snippet: "This command can be used to get or set clean_session flag of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__clean_session.html",
            examples: "# Convert non-clean-session connections to clean-session connections\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n       \"CONNECT\" {\n           if { [MQTT::clean_session] == 1} {\n              MQTT::clean_session 0\n           }\n       }\n   }\n}",
            return_value: "When called without an argument, this command returns the clean_session flag of MQTT CONNECT message.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MQTT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
