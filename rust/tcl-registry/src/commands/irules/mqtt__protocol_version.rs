//! `MQTT::protocol_version` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::protocol_version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set protocol revision level of MQTT CONNECT message",
            synopsis: &["MQTT::protocol_version (VERSION)?"],
            snippet: "This command can be used to get or set protocol revision level of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__protocol_version.html",
            examples: "# Upgrade protocol from 3.1 to 3.1.1\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n       \"CONNECT\" {\n          if {[MQTT::protocol_version] == 3 } {\n             MQTT::protocol_version 4\n             MQTT::protocol_name \"MQTT\"\n          }\n       }\n   }\n}",
            return_value: "When called without an argument, this command returns the protocol revision of MQTT CONNECT message",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MQTT::protocol_version (VERSION)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
