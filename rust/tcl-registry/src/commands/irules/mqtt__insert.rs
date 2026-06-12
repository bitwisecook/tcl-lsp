//! `MQTT::insert` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::insert",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Insert an MQTT message",
            synopsis: &["MQTT::insert (('before' | 'after') ("],
            snippet: "This command can be used to insert an MQTT message before or after current message.\nSince MQTT_CLIENT_SHUTDOWN event does not have current message only 'MQTT::insert after' is supported for it.\n\nThis command is valid for all MQTT message types:\n\n    CONNECT, CONNACK,\n    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n    SUBSCRIBE, SUBACK,\n    UNSUBSCRIBE, UNSUBACK,\n    PINGREQ, PINGRESP,\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__insert.html",
            examples: "when MQTT_SERVER_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n       \"SUBACK\" {\n          if { [MQTT::packet_id > 1000] } {\n             MQTT::drop\n          }\n       }\n    }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::insert (('before' | 'after') (" },
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
