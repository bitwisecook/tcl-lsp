//! `MQTT::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Transmit MQTT message to sender",
            synopsis: &["MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)"],
            snippet: "This command can be used to transmit MQTT message back to sender of the incoming message.\nIf called from MQTT_CLIENT_INGRESS message will be sent to the client.\nIf called from MQTT_SERVER_INGRESS message will be sent to the server.\nPlease note that current message will be forwarded to destination. Use MQTT::drop to drop the current message.\nThis command is valid for all MQTT message types:\n\n    CONNECT, CONNACK,\n    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n    SUBSCRIBE, SUBACK,\n    UNSUBSCRIBE, UNSUBACK,\n    PINGREQ, PINGRESP,\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__respond.html",
            examples: "#Enrich MQTT username with SSL client-certificate common name, reject unauthorized accesses:\nwhen CLIENT_ACCEPTED {\n    set cn \"\"\n}",
            return_value: "None.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::respond ( (('type' 'CONNACK') ('return_code' RETURN_CODE)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
