//! `MQTT::will` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::will",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set will-topic, will-message, will-qos, and will-retain fields of MQTT CONNECT message.",
            synopsis: &["MQTT::will (('topic' (TOPIC)?) |"],
            snippet: "This command can be used to get or set will-topic, will-message, will-qos, and will-retain fields of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__will.html",
            examples: "# Enforce a mandatary default will message, if will is not present in connect\nwhen MQTT_CLIENT_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n        \"CONNECT\" {\n            if { [MQTT::will topic] == \"\" } {\n                MQTT::will topic \"/bigip/default/will/[MQTT::username]/[MQTT::client_id]/[client_addr]\"\n                MQTT::will message \"client disconnected without sending DISCONNECT message\"\n                MQTT::will qos 0\n                MQTT::will retain 0\n            }",
            return_value: "When called without an argument, each of the sub-commands return the will-topic, will-message, will-qos, or will-retain field of MQTT CONNECT message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::will (('topic' (TOPIC)?) |" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
