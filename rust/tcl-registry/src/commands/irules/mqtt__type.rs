//! `MQTT::type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get type of MQTT message",
            synopsis: &["MQTT::type"],
            snippet: "This command can be used to get type of MQTT message.\nThis command is valid for all MQTT message types:\n\n    CONNECT, CONNACK,\n    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n    SUBSCRIBE, SUBACK,\n    UNSUBSCRIBE, UNSUBACK,\n    PINGREQ, PINGRESP,\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__type.html",
            examples: "# Typical usage pattern...\n#\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n      \"CONNECT\" {\n         # Do connect processing\n      }\n      \"SUBSCRIBE\" {\n         # Do subscribe processing\n      }\n      \"PUBLISH\" {\n         # Do publish processing\n      }\n   }\n}",
            return_value: "A string representation of MQTT message types:",
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
