//! `MQTT::dup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::dup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set duplicate flag of MQTT PUBLISH message.",
            synopsis: &["MQTT::dup ('0' | '1')?"],
            snippet: "This command can be used to get or set duplicate flag of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH",
            source: "https://clouddocs.f5.com/api/irules/MQTT__dup.html",
            examples: "#Downgrading QoS to 0:\n\nwhen MQTT_CLIENT_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n        \"PUBLISH\" {\n            set in_qos [MQTT::qos]\n            if { $in_qos > 0 } {\n                set pktid [MQTT::packet_id]\n            }\n            MQTT::dup 0\n            MQTT::qos 0\n            if { $in_qos == 1 } {\n                MQTT::respond type PUBACK packet_id $pktid\n            } elseif { $in_qos == 2 } {\n                MQTT::respond type PUBREC packet_id $pktid\n            }",
            return_value: "When called without an argument, this command returns the duplicate flag of MQTT message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::dup ('0' | '1')?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
