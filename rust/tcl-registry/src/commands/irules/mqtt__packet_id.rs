//! `MQTT::packet_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::packet_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set packet-id of MQTT message",
            synopsis: &["MQTT::packet_id (PACKETID)?"],
            snippet: "This command can be used to get or set packet-id field of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH (if QoS > 0)\n    PUBACK\n    PUBREC\n    PUBREL\n    PUBCOMP\n    SUBSCRIBE\n    SUBACK\n    UNSUBSCRIBE\n    UNSUBACK\n    PINGREQ\n    PINGRESP\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__packet_id.html",
            examples: "when SERVER_CONNECTED {\n   set suback_count 0\n   set rclist [list]\n}",
            return_value: "When called without an argument, this command returns the packet-id of MQTT message",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::packet_id (PACKETID)?" },
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
