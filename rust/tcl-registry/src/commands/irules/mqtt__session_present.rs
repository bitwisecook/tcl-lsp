//! `MQTT::session_present` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::session_present",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set session_present flag of MQTT CONNACK message.",
            synopsis: &["MQTT::session_present ('0' | '1')?"],
            snippet: "This command can be used to get or set session_present flag of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNACK",
            source: "https://clouddocs.f5.com/api/irules/MQTT__session_present.html",
            examples: "# Prevent session_present from being 1\nwhen MQTT_SERVER_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n        \"CONNACK\" {\n            if { [MQTT::session_present] == 1 } {\n                MQTT::session_present 0\n            }\n        }\n    }\n}",
            return_value: "When called without an argument, this command returns the session_present flag of MQTT CONNACK message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MQTT::session_present ('0' | '1')?" },
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
