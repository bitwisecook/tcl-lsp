//! `MQTT::length` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::length",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get length of MQTT message",
            synopsis: &["MQTT::length"],
            snippet: "This command can be used to get length of MQTT message.\nThis command is valid for all MQTT message types:\n\n    CONNECT, CONNACK,\n    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n    SUBSCRIBE, SUBACK,\n    UNSUBSCRIBE, UNSUBACK,\n    PINGREQ, PINGRESP,\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__length.html",
            examples: "# Drop connections with messages that exceed an administrative max\n#\nwhen MQTT_CLIENT_INGRESS {\n  set length [MQTT::length]\n  if { $length > 65536 } {\n     MQTT::disconnect\n  }\n}",
            return_value: "This command returns the length of MQTT message.",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::length" },
        ],
        ..CommandSpec::DEFAULT
    }
}
