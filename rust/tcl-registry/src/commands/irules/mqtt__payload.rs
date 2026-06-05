//! `MQTT::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Manipulate payload of MQTT PUBLISH message",
            synopsis: &["MQTT::payload ?subcommand? ?args?", "MQTT::payload length", "MQTT::payload replace <data> ?offset? ?length?"],
            snippet: "This command can be used to manipulate payload of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH",
            source: "https://clouddocs.f5.com/api/irules/MQTT__payload.html",
            examples: "#Example: Redirect PUBLISH that has payloads with blocked keywords defined in\n#blacklisted_keywords_datagroup in first 200 bytes. Prepend a admin message in\n#the payload.\n#\nwhen MQTT_CLIENT_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n       \"PUBLISH\" {\n          if { [class exists  blacklisted_keywords_datagroup] } {\n             MQTT::collect 200\n          }\n       }\n    }\n}",
            return_value: "When called without an argument, this command returns the collected payload of MQTT PUBLISH message.",
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
